"""The result of a match."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

MatchKind = Literal["exact", "fuzzy", "leet", "collapsed", "substring", "phrase"]


@dataclass(frozen=True, slots=True)
class Match:
    """One detected occurrence.

    :param word: The dictionary entry that matched, as written in the word list.
    :param matched_text: The matched slice, always equal to ``text[start:end]``.
    :param start: Byte offset into the original text.
    :param end: Byte offset into the original text, exclusive.
    :param language: Language the entry came from, or ``None`` for added words.
    :param score: Similarity; ``1.0`` for anything but a fuzzy match.
    :param kind: How it was found.
    """

    word: str
    matched_text: str
    start: int
    end: int
    language: str | None
    score: float
    kind: MatchKind


def match_from_tuple(raw: tuple[str, str, int, int, str | None, float, str]) -> Match:
    """Build a :class:`Match` from what the native layer returns."""
    word, matched_text, start, end, language, score, kind = raw
    return Match(word, matched_text, start, end, language, score, kind)  # type: ignore[arg-type]
