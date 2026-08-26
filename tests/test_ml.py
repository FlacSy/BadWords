"""Tests for the ML module.

Skipped unless the optional dependencies and a model are both available:
``pip install 'badwords-py[ml]'`` and either a cached model, a source checkout
with ``ml/models``, or ``BADWORDS_ML_PATH``.
"""

from __future__ import annotations

import zipfile
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from pathlib import Path

onnxruntime = pytest.importorskip("onnxruntime")
transformers = pytest.importorskip("transformers")

from badwords.ml import ModelDownloadError, ToxicityPredictor
from badwords.ml._paths import (
    REQUIRED_FILES,
    _safe_extract,
    is_complete,
)


def model_dir() -> Path | None:
    """Return a usable model directory, or None."""
    from badwords.ml import get_model_dir

    try:
        return get_model_dir(download=False)
    except Exception:
        return None


requires_model = pytest.mark.skipif(model_dir() is None, reason="no ML model available")


class TestModelPaths:
    """Locating a model."""

    def test_is_complete_requires_every_file(self, tmp_path: Path) -> None:
        assert not is_complete(tmp_path)
        for name in REQUIRED_FILES:
            (tmp_path / name).write_text("x")
        assert is_complete(tmp_path)

    def test_partial_directory_is_not_accepted(self, tmp_path: Path) -> None:
        """A half-extracted download must not look like a working model.

        2.x checked only model.onnx, so an interrupted extract was cached
        forever and failed later inside the tokenizer.
        """
        (tmp_path / "model.onnx").write_text("x")
        assert not is_complete(tmp_path)

    def test_env_override_reports_what_is_missing(
        self,
        tmp_path: Path,
        monkeypatch: pytest.MonkeyPatch,
    ) -> None:
        from badwords.ml import get_model_dir
        from badwords.ml._paths import ModelNotFoundError

        monkeypatch.setenv("BADWORDS_ML_PATH", str(tmp_path))
        with pytest.raises(ModelNotFoundError, match=r"model\.onnx"):
            get_model_dir(download=False)


class TestSafeExtract:
    """Archive extraction."""

    def test_rejects_paths_escaping_the_destination(self, tmp_path: Path) -> None:
        """Zip slip: an entry must not be able to write outside the target."""
        archive = tmp_path / "evil.zip"
        with zipfile.ZipFile(archive, "w") as zf:
            zf.writestr("../escaped.txt", "nope")

        with pytest.raises(ModelDownloadError, match="unsafe path"):
            _safe_extract(archive, tmp_path / "out")
        assert not (tmp_path / "escaped.txt").exists()

    def test_extracts_normal_archives(self, tmp_path: Path) -> None:
        archive = tmp_path / "fine.zip"
        with zipfile.ZipFile(archive, "w") as zf:
            zf.writestr("model.onnx", "x")
            zf.writestr("nested/config.json", "{}")

        destination = tmp_path / "out"
        _safe_extract(archive, destination)
        assert (destination / "model.onnx").exists()
        assert (destination / "nested" / "config.json").exists()


@requires_model
class TestPredictor:
    """Inference against the real model."""

    @pytest.fixture(scope="class")
    def predictor(self) -> ToxicityPredictor:
        p = ToxicityPredictor()
        p.load()
        return p

    def test_construction_does_not_load(self) -> None:
        """Constructing must not touch the disk or the network."""
        p = ToxicityPredictor()
        assert p._session is None

    def test_scores_are_probabilities(self, predictor: ToxicityPredictor) -> None:
        for text in ["hello there", "you are a fucking idiot"]:
            score = predictor.predict(text)
            assert 0.0 <= score <= 1.0

    def test_separates_toxic_from_clean(self, predictor: ToxicityPredictor) -> None:
        clean = predictor.predict("what a lovely afternoon")
        toxic = predictor.predict("you are a fucking idiot")
        assert toxic > 0.5
        assert clean < 0.5
        assert toxic > clean

    def test_is_toxic_uses_the_threshold(self, predictor: ToxicityPredictor) -> None:
        assert predictor.is_toxic("you are a fucking idiot") is True
        assert predictor.is_toxic("what a lovely afternoon") is False
        assert predictor.is_toxic("what a lovely afternoon", threshold=0.0) is True

    def test_batch_agrees_with_single(self, predictor: ToxicityPredictor) -> None:
        """Batched scores agree with single ones.

        Batching pads, and the quantized model is not perfectly invariant to
        padding, so agreement is to a few hundredths rather than exact.
        """
        texts = ["hello there", "you are a fucking idiot", "a normal sentence"]
        batch = predictor.predict_batch(texts)
        single = [predictor.predict(t) for t in texts]

        assert len(batch) == len(texts)
        for got, want in zip(batch, single, strict=True):
            assert abs(got - want) < 0.1
            assert (got > 0.5) == (want > 0.5)

    def test_empty_batch(self, predictor: ToxicityPredictor) -> None:
        assert predictor.predict_batch([]) == []

    def test_no_torch_needed(self) -> None:
        """The inference path must not import torch.

        `optimum` pulls torch in unconditionally, which is why it is gone.
        """
        import sys

        for module in list(sys.modules):
            if module == "torch" or module.startswith("torch."):
                del sys.modules[module]

        p = ToxicityPredictor()
        p.load()
        p.predict("some text")
        assert "torch" not in sys.modules


@requires_model
class TestHybridFilter:
    """The rule-plus-model facade."""

    def test_certain_cases_do_not_call_the_model(self) -> None:
        from badwords.ml import HybridFilter

        f = HybridFilter(languages=["en"])

        exact = f.check("this is shit")
        assert exact.is_profane is True
        assert exact.decided_by == "rules"
        assert exact.ml_score is None

        clean = f.check("a perfectly ordinary sentence")
        assert clean.is_profane is False
        assert clean.decided_by == "rules"
        assert clean.ml_score is None
        # The model was never loaded, so nothing was downloaded either.
        assert f.predictor._session is None

    def test_uncertain_cases_go_to_the_model(self) -> None:
        from badwords.ml import HybridFilter

        f = HybridFilter(languages=["en"], call_range=(0.85, 0.99))
        result = f.check("you are a dikhead")

        assert result.decided_by == "model"
        assert result.ml_score is not None
        assert 0.85 <= result.rule_score < 0.99

    def test_check_many_matches_check(self) -> None:
        from badwords.ml import HybridFilter

        f = HybridFilter(languages=["en"], call_range=(0.85, 0.99))
        texts = ["this is shit", "a clean sentence", "you are a dikhead"]

        batched = f.check_many(texts)
        assert [r.is_profane for r in batched] == [f.check(t).is_profane for t in texts]

    def test_rejects_an_invalid_call_range(self) -> None:
        from badwords.ml import HybridFilter

        with pytest.raises(ValueError, match="call_range"):
            HybridFilter(languages=["en"], call_range=(0.99, 0.5))
