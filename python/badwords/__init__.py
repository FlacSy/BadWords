"""High-performance profanity filter with a Rust core.

Example::

    from badwords import ProfanityFilter

    p = ProfanityFilter()
    p.init(languages=["en", "ru"])

    p.is_profane("some bad text")   # -> bool
    p.censor("some bad text", "*")  # -> str
    p.find("some bad text")         # -> list[Match]
"""

from __future__ import annotations

import warnings
from typing import TYPE_CHECKING, Any

from ._native import ProfanityFilter as _NativeFilter
from ._native import __version__ as __version__
from .exceptions import NotSupportedLanguage
from .match import Match, MatchKind, match_from_tuple
from .options import DEFAULT_OPTIONS, MatchMode, Options

if TYPE_CHECKING:
    from collections.abc import Iterable, Sequence

__all__ = [
    "DEFAULT_OPTIONS",
    "Match",
    "MatchKind",
    "MatchMode",
    "NotSupportedLanguage",
    "Options",
    "ProfanityFilter",
    "__version__",
]

_UNINITIALIZED = "ProfanityFilter is not initialized. Call init() first."


class ProfanityFilter:
    """Profanity filter.

    Create one, call :meth:`init` to load word lists, then match::

        p = ProfanityFilter()
        p.init(languages=["en", "ru"])
        p.is_profane("some text")

    Word lists are compiled into the extension; pass ``resource_dir`` to
    :meth:`init` to use your own.
    """

    __slots__ = ("_default_options", "_native")

    def __init__(self) -> None:
        """Create an uninitialized filter."""
        self._native: _NativeFilter | None = None
        self._default_options = DEFAULT_OPTIONS

    # -- setup ---------------------------------------------------------------

    def init(
        self,
        languages: Sequence[str] | None = None,
        *,
        options: Options | None = None,
        resource_dir: str | None = None,
        processing_normalize_text: bool = True,
        processing_aggressive_normalize: bool = True,
        processing_transliterate: bool = True,
        processing_replace_homoglyphs: bool = True,
    ) -> None:
        """Load word lists.

        :param languages: Language codes to load; ``None`` loads all of them.
            ISO 639-1 codes and the pre-3.0 codes both work.
        :param options: Default options for calls that do not pass their own.
        :param resource_dir: Directory of custom word lists. Defaults to the
            lists compiled into the extension.
        :param processing_normalize_text: Apply NFKC, case folding and the
            confusable-character tables.
        :param processing_aggressive_normalize: Also drop underscores.
        :param processing_transliterate: Fold latin and cyrillic onto one script.
        :param processing_replace_homoglyphs: Fold cross-script lookalikes.
        :raises NotSupportedLanguage: If a language code is unknown.
        """
        self._native = _NativeFilter(
            resource_dir,
            processing_normalize_text,
            processing_aggressive_normalize,
            processing_transliterate,
            processing_replace_homoglyphs,
        )
        if options is not None:
            self._default_options = options
        self._native.reload_languages(list(languages) if languages is not None else None)
        self._emit_warnings()

    @property
    def options(self) -> Options:
        """Default options for calls that do not pass their own."""
        return self._default_options

    @options.setter
    def options(self, options: Options) -> None:
        self._default_options = options

    # -- languages -----------------------------------------------------------

    def load_languages(self, languages: Sequence[str] | None = None) -> None:
        """Load languages in addition to those already loaded."""
        self._ensure_init().load_languages(list(languages) if languages is not None else None)
        self._emit_warnings()

    def unload_languages(self, languages: Sequence[str]) -> None:
        """Unload languages, dropping the words only they provided."""
        self._ensure_init().unload_languages(list(languages))

    def available_languages(self) -> list[str]:
        """Every language that could be loaded."""
        return self._ensure_init().available_languages()

    def loaded_languages(self) -> list[str]:
        """Languages whose word lists are loaded, as canonical codes."""
        return self._ensure_init().loaded_languages()

    def resolve_language(self, code: str) -> str:
        """Canonical code for a code or alias.

        :raises NotSupportedLanguage: If it is neither.
        """
        return self._ensure_init().resolve_language(code)

    # -- dictionary ----------------------------------------------------------

    def add_words(self, words: Iterable[str]) -> None:
        """Add words on top of the loaded languages."""
        self._ensure_init().add_words(list(words))

    def remove_words(self, words: Iterable[str]) -> None:
        """Remove words. A word a loaded language also provides stays."""
        self._ensure_init().remove_words(list(words))

    def clear_words(self) -> None:
        """Drop every word, including those from loaded languages."""
        self._ensure_init().clear_words()

    def word_count(self) -> int:
        """Number of distinct entries."""
        return self._ensure_init().word_count()

    def contains_word(self, word: str) -> bool:
        """Whether a word is in the dictionary, after normalization."""
        return self._ensure_init().contains_word(word)

    # -- whitelist -----------------------------------------------------------

    def add_whitelist(self, words: Iterable[str]) -> None:
        """Never report these words, even when a rule would match them."""
        self._ensure_init().add_whitelist(list(words))

    def remove_whitelist(self, words: Iterable[str]) -> None:
        """Drop words from the whitelist."""
        self._ensure_init().remove_whitelist(list(words))

    def clear_whitelist(self) -> None:
        """Empty the whitelist."""
        self._ensure_init().clear_whitelist()

    def is_whitelisted(self, word: str) -> bool:
        """Whether a word is whitelisted, after normalization."""
        return self._ensure_init().is_whitelisted(word)

    # -- matching ------------------------------------------------------------

    def is_profane(self, text: str, options: Options | None = None) -> bool:
        """Whether the text contains profanity."""
        return self._ensure_init().is_profane(text, **self._opts(options))

    def find(self, text: str, options: Options | None = None) -> list[Match]:
        """Every match, sorted by position and non-overlapping."""
        raw = self._ensure_init().find(text, **self._opts(options))
        return [match_from_tuple(item) for item in raw]

    def censor(
        self, text: str, replace_character: str = "*", options: Options | None = None
    ) -> str:
        """Replace every match, keeping everything else.

        Punctuation attached to a word survives::

            p.censor("hey shit, ok")   # "hey ****, ok"
        """
        return self._ensure_init().censor(text, replace_character, **self._opts(options))

    def is_profane_many(self, texts: Sequence[str], options: Options | None = None) -> list[bool]:
        """:meth:`is_profane` over many texts, releasing the GIL once."""
        return self._ensure_init().is_profane_many(list(texts), **self._opts(options))

    def find_many(self, texts: Sequence[str], options: Options | None = None) -> list[list[Match]]:
        """:meth:`find` over many texts."""
        raw = self._ensure_init().find_many(list(texts), **self._opts(options))
        return [[match_from_tuple(item) for item in group] for group in raw]

    def censor_many(
        self,
        texts: Sequence[str],
        replace_character: str = "*",
        options: Options | None = None,
    ) -> list[str]:
        """:meth:`censor` over many texts."""
        return self._ensure_init().censor_many(
            list(texts), replace_character, **self._opts(options)
        )

    # -- utility -------------------------------------------------------------

    def similar(self, a: str, b: str) -> float:
        """Jaro-Winkler similarity of two strings."""
        return self._ensure_init().similar(a, b)

    def normalize(self, text: str) -> str:
        """The normalized form of a text, as the matcher sees it."""
        return self._ensure_init().normalize(text)

    # -- deprecated ----------------------------------------------------------

    def filter_text(
        self,
        text: str,
        match_threshold: float | None = None,
        replace_character: str | None = None,
    ) -> bool | str:
        """Check or censor text.

        .. deprecated:: 3.0.0
            Use :meth:`is_profane`, :meth:`censor` or :meth:`find`. The return
            type depends on the arguments, and censoring replaces the whole
            whitespace-delimited token including any punctuation attached to it.
        """
        warnings.warn(
            "filter_text() is deprecated; use is_profane(), censor() or find()",
            DeprecationWarning,
            stacklevel=2,
        )
        return self._ensure_init().filter_text(text, match_threshold or 1.0, replace_character)

    def get_all_languages(self) -> list[str]:
        """Loaded languages.

        .. deprecated:: 3.0.0
            The name is misleading. Use :meth:`loaded_languages` or
            :meth:`available_languages`.
        """
        warnings.warn(
            "get_all_languages() is deprecated; use loaded_languages() or available_languages()",
            DeprecationWarning,
            stacklevel=2,
        )
        return self._ensure_init().loaded_languages()

    # -- internals -----------------------------------------------------------

    def _ensure_init(self) -> _NativeFilter:
        if self._native is None:
            raise RuntimeError(_UNINITIALIZED)
        return self._native

    def _opts(self, options: Options | None) -> dict[str, Any]:
        return (options or self._default_options).as_kwargs()

    def _emit_warnings(self) -> None:
        if self._native is None:
            return
        for kind, message in self._native.take_warnings():
            if kind == "deprecated_alias":
                warnings.warn(message, DeprecationWarning, stacklevel=3)
            elif kind == "empty_word_list":
                warnings.warn(message, UserWarning, stacklevel=3)
