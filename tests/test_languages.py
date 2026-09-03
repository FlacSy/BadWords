"""Language codes, aliases and per-language loading."""

from __future__ import annotations

import pytest
from badwords import ProfanityFilter
from badwords.exceptions import NotSupportedLanguage

# Canonical codes, as shipped in resources/data/languages.json.
LANGUAGES = [
    "cs",
    "da",
    "de",
    "el",
    "en",
    "es",
    "es_419",
    "fi",
    "fr",
    "hu",
    "id",
    "it",
    "ja",
    "ko",
    "nl",
    "no",
    "pl",
    "pt",
    "pt_br",
    "ro",
    "ru",
    "sv",
    "th",
    "tr",
    "uk",
]

# The codes used before 3.0.0 keep working.
ALIASES = {
    "sp": "es",
    "du": "nl",
    "po": "pt",
    "gr": "el",
    "ua": "uk",
    "cz": "cs",
    "tu": "tr",
    "br": "pt_br",
    "in": "id",
    "sw": "sv",
    "lt": "es_419",
}

# These four collide with a different real language, so using them warns.
DEPRECATED_ALIASES = ["br", "in", "lt", "sw"]


@pytest.fixture(scope="module")
def filter_all() -> ProfanityFilter:
    """Filter with every language loaded."""
    p = ProfanityFilter()
    p.init()
    return p


@pytest.mark.parametrize("lang", LANGUAGES)
def test_language_loads(lang: str) -> None:
    """Each language loads on its own."""
    p = ProfanityFilter()
    p.init(languages=[lang])
    assert p.loaded_languages() == [lang]
    assert p.word_count() > 0


@pytest.mark.parametrize("lang", LANGUAGES)
def test_language_detects_added_word(lang: str) -> None:
    """Custom words work whichever language is loaded."""
    p = ProfanityFilter()
    p.init(languages=[lang])
    p.add_words([f"langtest{lang}"])
    assert p.is_profane(f"langtest{lang}") is True
    assert p.is_profane("clean text") is False


@pytest.mark.parametrize("lang", LANGUAGES)
def test_language_censor_works(lang: str) -> None:
    """Censoring works whichever language is loaded."""
    p = ProfanityFilter()
    p.init(
        languages=[lang],
        processing_transliterate=False,
        processing_replace_homoglyphs=False,
    )
    p.add_words(["badword"])
    result = p.censor("x badword y")
    assert result == "x ******* y"


@pytest.mark.parametrize(("alias", "canonical"), sorted(ALIASES.items()))
def test_alias_resolves(alias: str, canonical: str, filter_all: ProfanityFilter) -> None:
    """Pre-3.0 codes resolve to their canonical form."""
    assert filter_all.resolve_language(alias) == canonical


@pytest.mark.parametrize("alias", DEPRECATED_ALIASES)
def test_misleading_alias_warns(alias: str) -> None:
    """Codes that collide with another real language warn on use."""
    p = ProfanityFilter()
    with pytest.deprecated_call():
        p.init(languages=[alias])
    assert p.loaded_languages() == [ALIASES[alias]]


def test_plain_alias_does_not_warn(recwarn: pytest.WarningsRecorder) -> None:
    """An alias that is merely non-standard loads quietly."""
    p = ProfanityFilter()
    p.init(languages=["sp"])
    assert p.loaded_languages() == ["es"]
    assert not [w for w in recwarn if issubclass(w.category, DeprecationWarning)]


@pytest.mark.parametrize("code", ["ES_419", "es-419", " es_419 "])
def test_codes_are_normalized(code: str, filter_all: ProfanityFilter) -> None:
    """Case, whitespace and hyphens do not matter."""
    assert filter_all.resolve_language(code) == "es_419"


@pytest.mark.parametrize(
    "attempt",
    ["../../../etc/passwd", "../en", "en/../../secret", "/etc/passwd"],
)
def test_language_codes_cannot_traverse_paths(attempt: str) -> None:
    """A code is looked up in the registry, never used as a path."""
    p = ProfanityFilter()
    with pytest.raises(NotSupportedLanguage):
        p.init(languages=[attempt])


def test_all_languages_available(filter_all: ProfanityFilter) -> None:
    """Every shipped language is discoverable."""
    assert sorted(filter_all.available_languages()) == sorted(LANGUAGES)
