"""Integration tests - full scenarios."""

from __future__ import annotations

import pytest

from badwords import ProfanityFilter


class TestFullWorkflow:
    """End-to-end workflow tests."""

    def test_chat_moderation_scenario(self) -> None:
        """Simulate chat moderation with custom words."""
        p = ProfanityFilter()
        p.init(languages=["en", "ru"])
        p.add_words(["spam_link", "scam_bot"])

        assert p.filter_text("Check out this spam_link") is True
        assert p.filter_text("Hello, how are you?") is False

    def test_censoring_workflow(self) -> None:
        """Censoring with replace_character."""
        p = ProfanityFilter()
        p.init(
            languages=["en"],
            processing_transliterate=False,
            processing_replace_homoglyphs=False,
        )
        p.add_words(["bad"])

        result = p.filter_text("a bad word", replace_character="*")
        assert result == "a *** word"

    def test_multiple_languages_loaded(self) -> None:
        """Words from multiple languages are detected."""
        p = ProfanityFilter()
        p.init(languages=["en", "ru", "de"])
        langs = p.get_all_languages()
        assert len(langs) == 3
        assert "en" in langs and "ru" in langs and "de" in langs

    def test_filter_text_before_init_raises(self) -> None:
        """filter_text without init raises RuntimeError."""
        p = ProfanityFilter()
        with pytest.raises(RuntimeError):
            p.filter_text("test")
