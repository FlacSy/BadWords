"""End-to-end scenarios."""

from __future__ import annotations

import pytest
from badwords import Options, ProfanityFilter


def test_chat_moderation() -> None:
    """Block and censor messages with project-specific words."""
    p = ProfanityFilter()
    p.init(languages=["en", "ru"])
    p.add_words(["spam_link", "scam_bot"])

    assert p.is_profane("Check out this spam_link") is True
    assert p.is_profane("Hello, how are you?") is False
    assert p.censor("Check out this spam_link") == "Check out this *********"


def test_censoring_workflow() -> None:
    """Censoring leaves the rest of the sentence untouched."""
    p = ProfanityFilter()
    p.init(
        languages=["en"],
        processing_transliterate=False,
        processing_replace_homoglyphs=False,
    )
    p.add_words(["bad"])
    assert p.censor("a bad word") == "a *** word"
    assert p.censor("a bad word!", "#") == "a ### word!"


def test_multiple_languages_loaded() -> None:
    """Words from several languages are detected at once."""
    p = ProfanityFilter()
    p.init(languages=["en", "ru", "de"])
    assert p.loaded_languages() == ["en", "ru", "de"]
    assert p.is_profane("shit") is True
    assert p.is_profane("хуй") is True


def test_moderation_pipeline_with_options() -> None:
    """A stricter option set catches evasion the default misses."""
    p = ProfanityFilter()
    p.init(languages=["en"])
    strict = Options(
        split_on_punctuation=True,
        collapse_repeats=True,
        leetspeak=True,
    )

    evasions = ["shiiit", "you.shit", "sh1t"]
    assert [p.is_profane(t, strict) for t in evasions] == [True, True, True]

    # And still leaves ordinary text alone.
    clean = ["a bookkeeper", "100k users", "classic design", "the assessment"]
    assert [p.is_profane(t, strict) for t in clean] == [False, False, False, False]


def test_find_drives_a_report() -> None:
    """Find gives enough detail to explain a decision."""
    p = ProfanityFilter()
    p.init(languages=["en"])
    matches = p.find("what a shitty, damn mess")

    assert [m.matched_text for m in matches] == ["shitty", "damn"]
    assert all(m.language == "en" for m in matches)
    assert all(m.kind == "exact" for m in matches)


def test_filter_text_before_init_raises() -> None:
    """Using a filter before init is an error."""
    p = ProfanityFilter()
    with pytest.raises(RuntimeError):
        p.is_profane("test")
