"""The label space, shared by data preparation, training and export.

Seven axes, in this order. `civil_comments` annotates all of them as the
fraction of annotators who picked that axis, so targets are soft (0.0 - 1.0)
rather than binary; every other source knows `toxicity` alone and leaves the
rest unsupervised.
"""

from __future__ import annotations

LABELS: tuple[str, ...] = (
    "toxicity",
    "severe_toxicity",
    "obscene",
    "threat",
    "insult",
    "identity_attack",
    "sexual_explicit",
)

#: Index of the overall-toxicity axis, which every source supervises.
TOXICITY = 0

#: Above this an axis counts as present when a hard label is needed.
DECISION_THRESHOLD = 0.5

TEXT_COLUMN = "text"
