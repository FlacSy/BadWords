"""Rule-based filtering with the model as a tie-breaker.

The rules are three orders of magnitude faster than the model, and on most
text they are also certain: an exact dictionary hit needs no second opinion,
and text with nothing even close to an entry needs none either. Only the band
in between - a fuzzy match good enough to be suspicious but not good enough to
act on - is worth a model call.

    from badwords.ml import HybridFilter

    f = HybridFilter(languages=["en", "ru"])
    f.is_profane("you are a dikhead")   # rules unsure -> model decides
    f.is_profane("hello there")         # rules certain -> model not called
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from badwords import Options, ProfanityFilter
from badwords.ml.predictor import DEFAULT_THRESHOLD, ToxicityPredictor

if TYPE_CHECKING:
    from collections.abc import Sequence
    from pathlib import Path

    from badwords.match import Match

#: Rule scores in this band go to the model. Below it the text is treated as
#: clean, at or above it as profane.
#:
#: The default only escalates near-misses, which is the cheap configuration:
#: text with nothing resembling a dictionary entry never reaches the model, so
#: the hybrid cannot catch toxicity the rules had no hint of. Use
#: ``call_range=(0.0, 0.99)`` to send everything the rules did not decide
#: outright to the model instead - far better recall, far more model calls.
DEFAULT_CALL_RANGE = (0.90, 0.99)


@dataclass(frozen=True, slots=True)
class HybridResult:
    """What the filter decided, and how.

    :param is_profane: The verdict.
    :param rule_score: Best similarity the rules found; 0.0 if nothing matched.
    :param ml_score: Model probability, or ``None`` when the model was not called.
    :param decided_by: ``"rules"`` or ``"model"``.
    :param matches: What the rules found.
    """

    is_profane: bool
    rule_score: float
    ml_score: float | None
    decided_by: str
    matches: list[Match] = field(default_factory=list)


class HybridFilter:
    """Rules first, model only for the uncertain band.

    With the default :data:`DEFAULT_CALL_RANGE` the model sees only text the
    rules nearly matched. Set ``call_range=(0.0, ...)`` to escalate everything
    the rules did not decide outright.
    """

    __slots__ = ("_call_max", "_call_min", "_decision_threshold", "_filter", "_predictor")

    def __init__(
        self,
        languages: Sequence[str] | None = None,
        *,
        call_range: tuple[float, float] = DEFAULT_CALL_RANGE,
        decision_threshold: float = DEFAULT_THRESHOLD,
        model_dir: Path | str | None = None,
        filter_options: Options | None = None,
    ) -> None:
        """Build a hybrid filter.

        :param languages: Languages to load; ``None`` loads all of them.
        :param call_range: Rule scores between these two call the model.
        :param decision_threshold: Model probability at or above which text is
            treated as profane.
        :param model_dir: Model directory; resolved on first model call when omitted.
        :param filter_options: Options for the rule pass. The threshold is
            replaced by the lower end of ``call_range``.
        :raises ValueError: If ``call_range`` is not an ascending pair in (0, 1].
        """
        call_min, call_max = call_range
        if not 0.0 <= call_min <= call_max <= 1.0:
            message = f"call_range must be ascending within [0, 1], got {call_range}"
            raise ValueError(message)

        self._call_min = call_min
        self._call_max = call_max
        self._decision_threshold = decision_threshold

        base = filter_options or Options()
        self._filter = ProfanityFilter()
        self._filter.init(
            languages=list(languages) if languages is not None else None,
            options=base.replace(match_threshold=call_min),
        )
        self._predictor = ToxicityPredictor(model_dir, threshold=decision_threshold)

    @property
    def filter(self) -> ProfanityFilter:
        """The rule-based filter, for adding words or a whitelist."""
        return self._filter

    @property
    def predictor(self) -> ToxicityPredictor:
        """The model, for warming it up with :meth:`ToxicityPredictor.load`."""
        return self._predictor

    def check(self, text: str) -> HybridResult:
        """Classify one text, reporting how the decision was reached."""
        matches = self._filter.find(text)
        rule_score = max((m.score for m in matches), default=0.0)

        if rule_score >= self._call_max:
            return HybridResult(
                is_profane=True,
                rule_score=rule_score,
                ml_score=None,
                decided_by="rules",
                matches=matches,
            )
        if rule_score < self._call_min:
            return HybridResult(
                is_profane=False,
                rule_score=rule_score,
                ml_score=None,
                decided_by="rules",
                matches=matches,
            )

        ml_score = self._predictor.predict(text)
        return HybridResult(
            is_profane=ml_score >= self._decision_threshold,
            rule_score=rule_score,
            ml_score=ml_score,
            decided_by="model",
            matches=matches,
        )

    def is_profane(self, text: str) -> bool:
        """Whether the text is profane."""
        return self.check(text).is_profane

    def check_many(self, texts: Sequence[str]) -> list[HybridResult]:
        """Classify several texts, batching the model calls into one pass."""
        results: list[HybridResult | None] = []
        uncertain: list[tuple[int, str]] = []

        for index, text in enumerate(texts):
            matches = self._filter.find(text)
            rule_score = max((m.score for m in matches), default=0.0)

            if rule_score >= self._call_max:
                results.append(
                    HybridResult(
                        is_profane=True,
                        rule_score=rule_score,
                        ml_score=None,
                        decided_by="rules",
                        matches=matches,
                    )
                )
            elif rule_score < self._call_min:
                results.append(
                    HybridResult(
                        is_profane=False,
                        rule_score=rule_score,
                        ml_score=None,
                        decided_by="rules",
                        matches=matches,
                    )
                )
            else:
                results.append(None)
                uncertain.append((index, text))

        if uncertain:
            scores = self._predictor.predict_batch([text for _, text in uncertain])
            for (index, _), ml_score in zip(uncertain, scores, strict=True):
                matches = self._filter.find(texts[index])
                rule_score = max((m.score for m in matches), default=0.0)
                results[index] = HybridResult(
                    is_profane=ml_score >= self._decision_threshold,
                    rule_score=rule_score,
                    ml_score=ml_score,
                    decided_by="model",
                    matches=matches,
                )

        return [result for result in results if result is not None]
