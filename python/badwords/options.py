"""Matching options."""

from __future__ import annotations

from dataclasses import asdict, dataclass, replace
from typing import Any, Literal

MatchMode = Literal["token", "substring"]


@dataclass(frozen=True, slots=True)
class Options:
    """How a single call should match.

    Every evasion detector is off by default, so :class:`Options` with no
    arguments reproduces the behaviour of badwords-py 2.x.

    :param match_threshold: Similarity a fuzzy match needs. ``1.0`` is exact only.
    :param match_mode: ``"token"`` matches whole words, ``"substring"`` also
        matches entries occurring inside a longer word.
    :param split_on_punctuation: Also test the pieces a token splits into on
        inner punctuation, catching ``you.fuck``.
    :param collapse_repeats: Also test forms with repeated letters collapsed,
        catching ``fuuuck`` and ``ffuck``.
    :param leetspeak: Also test a form with digits read as letters (``sh1t``).
    :param phrases: Match multi-word entries against consecutive words.
    :param min_substring_len: In substring mode, ignore entries shorter than this.
    :param max_matches: Stop after this many matches.
    """

    match_threshold: float = 1.0
    match_mode: MatchMode = "token"
    split_on_punctuation: bool = False
    collapse_repeats: bool = False
    leetspeak: bool = False
    phrases: bool = True
    min_substring_len: int = 6
    max_matches: int | None = None

    @classmethod
    def aggressive(cls) -> Options:
        """Every detector on, fuzzy at 0.9, substring matching enabled.

        Expect false positives; measure against your own corpus first.
        """
        return cls(
            match_threshold=0.9,
            match_mode="substring",
            split_on_punctuation=True,
            collapse_repeats=True,
            leetspeak=True,
        )

    def replace(self, **changes: Any) -> Options:  # noqa: ANN401
        """Return a copy with some fields changed."""
        return replace(self, **changes)

    def as_kwargs(self) -> dict[str, Any]:
        """Keyword arguments for the native layer."""
        return asdict(self)


DEFAULT_OPTIONS = Options()
