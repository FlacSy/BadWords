"""A library for effective moderation of content."""

from __future__ import annotations

from pathlib import Path

from ._native import PyProfanityFilter
from .exceptions import NotSupportedLanguage

# Resource directory next to this package
_RESOURCE_DIR = Path(__file__).parent / "resource"


class ProfanityFilter:
    """High-performance profanity filter (Rust backend).

    Create filter, then call init() to load languages::

        p = ProfanityFilter()
        p.init(languages=["en", "ru"])
    """

    def __init__(self) -> None:
        self._native: PyProfanityFilter | None = None

    def init(
        self,
        languages: list[str] | None = None,
        *,
        processing_normalize_text: bool = True,
        processing_aggressive_normalize: bool = True,
        processing_transliterate: bool = True,
        processing_replace_homoglyphs: bool = True,
    ) -> None:
        """Initialize the filter and load word lists.

        :param languages: List of language codes to load. None = all available.
        """
        self._native = PyProfanityFilter(
            str(_RESOURCE_DIR),
            processing_normalize_text,
            processing_aggressive_normalize,
            processing_transliterate,
            processing_replace_homoglyphs,
        )
        try:
            self._native.init(languages)
        except ValueError as e:
            if "not supported" in str(e).lower():
                raise NotSupportedLanguage from e
            raise

    def add_words(self, words: list[str]) -> None:
        """Add custom profanity words."""
        self._ensure_init()
        self._native.add_words(words)

    def filter_text(
        self,
        text: str,
        match_threshold: float | None = None,
        replace_character: str | None = None,
    ) -> bool | str:
        """Check text for profanity. Returns bool or censored str."""
        self._ensure_init()
        return self._native.filter_text(
            text,
            match_threshold or 1.0,
            replace_character,
        )

    def get_all_languages(self) -> list[str]:
        """Return list of loaded language codes."""
        self._ensure_init()
        return self._native.get_all_languages()

    def similar(self, a: str, b: str) -> float:
        """Similarity ratio between two strings."""
        self._ensure_init()
        return self._native.similar(a, b)

    def _ensure_init(self) -> None:
        if self._native is None:
            raise RuntimeError("ProfanityFilter not initialized. Call init() first.")
