"""Tests for each supported language."""

from __future__ import annotations

import pytest

from badwords import ProfanityFilter

# All languages from python/badwords/resource/words/
LANGUAGES = [
    "br",
    "cz",
    "da",
    "de",
    "du",
    "en",
    "fi",
    "fr",
    "gr",
    "hu",
    "in",
    "it",
    "ja",
    "ko",
    "lt",
    "no",
    "pl",
    "po",
    "ro",
    "ru",
    "sp",
    "sw",
    "th",
    "tu",
    "ua",
]


@pytest.mark.parametrize("lang", LANGUAGES)
def test_language_loads(lang: str) -> None:
    """Each language loads successfully."""
    p = ProfanityFilter()
    p.init(languages=[lang])
    assert p.get_all_languages() == [lang]


@pytest.mark.parametrize("lang", LANGUAGES)
def test_language_detects_added_word(lang: str) -> None:
    """Filter detects added word for each language."""
    p = ProfanityFilter()
    p.init(languages=[lang])
    test_word = f"langtest_{lang}"
    p.add_words([test_word])
    assert p.filter_text(test_word) is True
    assert p.filter_text("clean text") is False


@pytest.mark.parametrize("lang", LANGUAGES)
def test_language_censor_works(lang: str) -> None:
    """Censoring works for each language."""
    p = ProfanityFilter()
    p.init(
        languages=[lang],
        processing_transliterate=False,
        processing_replace_homoglyphs=False,
    )
    p.add_words(["badword"])
    result = p.filter_text("x badword y", replace_character="*")
    assert isinstance(result, str)
    assert "badword" not in result
    assert "*" in result


@pytest.mark.parametrize("lang", LANGUAGES)
def test_language_similar_method(lang: str) -> None:
    """similar() method works after language initialization."""
    p = ProfanityFilter()
    p.init(languages=[lang])
    assert p.similar("abc", "abc") == 1.0
    assert p.similar("abc", "xyz") < 1.0
