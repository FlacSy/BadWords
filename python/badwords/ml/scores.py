"""Per-axis toxicity scores."""

from __future__ import annotations

from dataclasses import dataclass

#: The axis every model annotates, and the one a single number comes from.
TOXICITY = "toxicity"

#: What the pre-3.1 binary model called the same thing.
_LEGACY_TOXICITY = "toxic"


@dataclass(frozen=True, slots=True)
class Scores:
    """One text's probability on each axis the model was trained for.

    The axes come from the model's own ``config.json``, so a model with a
    different label set stays usable: ask by name, or iterate.

        scores = predictor.predict_scores("you are an idiot")
        scores.toxicity          # 0.94
        scores["insult"]         # 0.93
        scores.above(0.5)        # [("toxicity", 0.94), ("insult", 0.93)]
    """

    labels: tuple[str, ...]
    values: tuple[float, ...]

    def get(self, label: str, default: float = 0.0) -> float:
        """Probability for a named axis, or ``default`` if there is no such axis."""
        try:
            return self.values[self.labels.index(label)]
        except ValueError:
            return default

    def __getitem__(self, label: str) -> float:
        """Probability for a named axis.

        :raises KeyError: If the model has no such axis.
        """
        try:
            return self.values[self.labels.index(label)]
        except ValueError as error:
            message = f"{label!r} is not one of {self.labels}"
            raise KeyError(message) from error

    def __contains__(self, label: str) -> bool:
        """Whether the model scores this axis."""
        return label in self.labels

    @property
    def toxicity(self) -> float:
        """Probability on the overall-toxicity axis, ``0.0`` if there is none."""
        if TOXICITY in self.labels:
            return self[TOXICITY]
        return self.get(_LEGACY_TOXICITY)

    def strongest(self) -> tuple[str, float]:
        """Return the highest-scoring axis and its probability."""
        index = max(range(len(self.values)), key=self.values.__getitem__)
        return self.labels[index], self.values[index]

    def above(self, threshold: float) -> list[tuple[str, float]]:
        """Axes scoring at or above ``threshold``, strongest first."""
        pairs = zip(self.labels, self.values, strict=True)
        hits = [(label, value) for label, value in pairs if value >= threshold]
        return sorted(hits, key=lambda pair: pair[1], reverse=True)

    def as_dict(self) -> dict[str, float]:
        """Every axis with its probability."""
        return dict(zip(self.labels, self.values, strict=True))
