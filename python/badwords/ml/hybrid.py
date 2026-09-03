"""Rules in front of the model.

The rules are three orders of magnitude faster than the model, and when they
find a dictionary entry outright they are also right - that verdict needs no
second opinion. What they cannot do is recognise toxicity built out of
ordinary words, which is most of it: on held-out English rows the dictionary
alone reaches 27% recall against the model's 87%. In Russian the same rules
reach 50%, so how much the model adds depends on the language.

So the split is deliberately one-sided. A certain rule hit answers
immediately; *everything else* goes to the model, text the rules found nothing
in included. Treating "the rules saw nothing" as "clean" is what made the
earlier hybrid score worse than the model it wraps.

    from badwords.ml import HybridFilter

    f = HybridFilter(languages=["en", "ru"])
    f.is_profane("you are a dikhead")   # dictionary hit -> no model call
    f.is_profane("people like you should not exist")  # model decides
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Literal

from badwords import Options, ProfanityFilter
from badwords.ml.predictor import DEFAULT_THRESHOLD, ToxicityPredictor

if TYPE_CHECKING:
    from collections.abc import Sequence
    from pathlib import Path

    from badwords.match import Match
    from badwords.ml.scores import Scores

#: Rule score at or above which the rules answer alone. ``1.0`` is an exact
#: dictionary hit; lower values let a fuzzy near-match decide too.
DEFAULT_CERTAIN_AT = 1.0

#: Rule options for the hybrid: every character-level evasion detector is on,
#: because each one flags zero of 73,302 ordinary English words - they cost
#: nothing here and they catch the spellings a dictionary alone misses.
DEFAULT_RULE_OPTIONS = Options(
    split_on_punctuation=True,
    collapse_repeats=True,
    leetspeak=True,
)


@dataclass(frozen=True, slots=True)
class HybridResult:
    """What the filter decided, and on what evidence.

    :param is_profane: The verdict.
    :param rule_score: Best rule score; 0.0 if nothing matched.
    :param scores: Every axis, present only when the model was called.
    :param decided_by: ``"rules"`` or ``"model"``.
    :param matches: What the rules found, whoever decided.
    """

    is_profane: bool
    rule_score: float
    scores: Scores | None
    decided_by: Literal["rules", "model"]
    matches: list[Match] = field(default_factory=list)

    @property
    def ml_score(self) -> float | None:
        """Overall toxicity from the model, or ``None`` if it was not called."""
        return None if self.scores is None else self.scores.toxicity


class HybridFilter:
    """A rule filter and a model, used together."""

    __slots__ = ("_certain_at", "_filter", "_options", "_predictor", "_threshold")

    def __init__(
        self,
        languages: Sequence[str] | None = None,
        *,
        certain_at: float = DEFAULT_CERTAIN_AT,
        threshold: float = DEFAULT_THRESHOLD,
        model_dir: Path | str | None = None,
        filter_options: Options | None = None,
    ) -> None:
        """Build a hybrid filter.

        :param languages: Languages to load; ``None`` loads all of them.
        :param certain_at: Rule score at or above which the rules answer alone.
        :param threshold: Model probability at or above which text is profane.
        :param model_dir: Model directory; resolved on the first model call.
        :param filter_options: Options for the rule pass.
        :raises ValueError: If ``certain_at`` is outside (0, 1].
        """
        if not 0.0 < certain_at <= 1.0:
            message = f"certain_at must be within (0, 1], got {certain_at}"
            raise ValueError(message)

        self._certain_at = certain_at
        self._threshold = threshold
        self._options = filter_options or DEFAULT_RULE_OPTIONS
        self._filter = ProfanityFilter()
        self._filter.init(
            languages=list(languages) if languages is not None else None,
            options=self._options,
        )
        self._predictor = ToxicityPredictor(model_dir, threshold=threshold)

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
        return self.check_many([text])[0]

    def check_many(self, texts: Sequence[str]) -> list[HybridResult]:
        """Classify several texts, batching the model calls into one pass."""
        found = [self._filter.find(text, self._options) for text in texts]
        rule_scores = [max((m.score for m in matches), default=0.0) for matches in found]

        results: list[HybridResult | None] = [
            HybridResult(
                is_profane=True,
                rule_score=score,
                scores=None,
                decided_by="rules",
                matches=matches,
            )
            if score >= self._certain_at
            else None
            for score, matches in zip(rule_scores, found, strict=True)
        ]

        pending = [index for index, result in enumerate(results) if result is None]
        if pending:
            scored = self._predictor.predict_scores_batch([texts[index] for index in pending])
            for index, scores in zip(pending, scored, strict=True):
                results[index] = HybridResult(
                    is_profane=scores.toxicity >= self._threshold,
                    rule_score=rule_scores[index],
                    scores=scores,
                    decided_by="model",
                    matches=found[index],
                )

        return [result for result in results if result is not None]

    def is_profane(self, text: str) -> bool:
        """Whether the text is profane."""
        return self.check(text).is_profane
