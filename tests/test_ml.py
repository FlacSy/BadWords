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


class TestScores:
    """The per-axis score container. Needs no model."""

    def make(self) -> object:
        from badwords.ml import Scores

        return Scores(
            labels=("toxicity", "insult", "threat"),
            values=(0.91, 0.87, 0.02),
        )

    def test_axes_are_reachable_by_name(self) -> None:
        scores = self.make()
        assert scores["insult"] == pytest.approx(0.87)
        assert scores.get("threat") == pytest.approx(0.02)
        assert "toxicity" in scores

    def test_unknown_axis_raises_but_get_defaults(self) -> None:
        scores = self.make()
        with pytest.raises(KeyError, match="obscene"):
            _ = scores["obscene"]
        assert scores.get("obscene") == 0.0
        assert scores.get("obscene", -1.0) == -1.0

    def test_toxicity_is_the_overall_axis(self) -> None:
        scores = self.make()
        assert scores.toxicity == pytest.approx(0.91)

    def test_toxicity_falls_back_to_the_legacy_name(self) -> None:
        from badwords.ml import Scores

        legacy = Scores(labels=("clean", "toxic"), values=(0.2, 0.8))
        assert legacy.toxicity == pytest.approx(0.8)

    def test_strongest_and_above_rank_by_score(self) -> None:
        scores = self.make()
        assert scores.strongest() == ("toxicity", pytest.approx(0.91))
        assert [axis for axis, _ in scores.above(0.5)] == ["toxicity", "insult"]
        assert scores.above(0.95) == []

    def test_as_dict_covers_every_axis(self) -> None:
        scores = self.make()
        assert scores.as_dict() == {
            "toxicity": pytest.approx(0.91),
            "insult": pytest.approx(0.87),
            "threat": pytest.approx(0.02),
        }


class TestReleaseAssets:
    """Publishing rules that keep installed copies working."""

    def test_the_asset_name_is_versioned(self) -> None:
        from badwords.ml import _paths

        # A 2.x/3.0 client reads output 1 through a softmax. On the multi-label
        # model that is `severe_toxicity`, so it would call obvious profanity
        # clean and say nothing. The generation has to be in the file name.
        assert _paths.ASSET_NAME != _paths.LEGACY_ASSET_NAME
        assert _paths.ASSET_NAME.endswith(".zip")

    def test_downloads_come_from_a_pinned_release(self) -> None:
        from badwords.ml import _paths

        # Not "latest": a later release may carry a differently shaped model.
        assert _paths.MODEL_RELEASE_TAG != "latest"
        assert _paths.MODEL_RELEASE_TAG.startswith("v")


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

    def test_a_certain_rule_hit_does_not_call_the_model(self) -> None:
        from badwords.ml import HybridFilter

        f = HybridFilter(languages=["en"])

        exact = f.check("this is shit")
        assert exact.is_profane is True
        assert exact.decided_by == "rules"
        assert exact.scores is None
        assert exact.ml_score is None
        # Nothing was loaded, so nothing was downloaded either.
        assert f.predictor._session is None

    def test_text_the_rules_do_not_settle_goes_to_the_model(self) -> None:
        from badwords.ml import HybridFilter

        f = HybridFilter(languages=["en"])

        # No dictionary entry is anywhere near this, and that is exactly the
        # case an earlier version answered "clean" without asking anyone.
        result = f.check("a perfectly ordinary sentence")
        assert result.decided_by == "model"
        assert result.scores is not None
        assert result.is_profane is False

    def test_the_model_supplies_every_axis(self) -> None:
        from badwords.ml import HybridFilter

        f = HybridFilter(languages=["en"])
        result = f.check("you are a worthless waste of oxygen")

        assert result.decided_by == "model"
        assert result.scores is not None
        assert set(result.scores.labels) == set(f.predictor.labels)
        assert result.ml_score == pytest.approx(result.scores.toxicity)

    def test_check_many_matches_check(self) -> None:
        from badwords.ml import HybridFilter

        f = HybridFilter(languages=["en"])
        texts = ["this is shit", "a clean sentence", "you are a dikhead"]

        batched = f.check_many(texts)
        assert [r.decided_by for r in batched] == [f.check(t).decided_by for t in texts]
        assert [r.is_profane for r in batched] == [f.check(t).is_profane for t in texts]

    def test_rejects_a_certainty_outside_the_unit_range(self) -> None:
        from badwords.ml import HybridFilter

        with pytest.raises(ValueError, match="certain_at"):
            HybridFilter(languages=["en"], certain_at=1.5)
