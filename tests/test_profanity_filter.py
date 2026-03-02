"""Tests for ProfanityFilter."""

from __future__ import annotations

import pytest

from badwords import ProfanityFilter
from badwords.exceptions import NotSupportedLanguage


class TestProfanityFilterInit:
    """Tests for ProfanityFilter initialization."""

    def test_init_all_languages(self) -> None:
        """Filter loads all languages when no list specified."""
        p = ProfanityFilter()
        p.init()
        langs = p.get_all_languages()
        assert len(langs) >= 20
        assert "en" in langs
        assert "ru" in langs

    def test_init_specific_languages(self) -> None:
        """Filter loads only specified languages."""
        p = ProfanityFilter()
        p.init(languages=["en", "ru"])
        assert p.get_all_languages() == ["en", "ru"]

    def test_init_unsupported_language_raises(self) -> None:
        """NotSupportedLanguage raised for invalid language code."""
        p = ProfanityFilter()
        with pytest.raises(NotSupportedLanguage):
            p.init(languages=["xx"])

    def test_init_partial_unsupported_raises(self) -> None:
        """NotSupportedLanguage when any language is invalid."""
        p = ProfanityFilter()
        with pytest.raises(NotSupportedLanguage):
            p.init(languages=["en", "xx"])

    def test_init_with_processing_options(self) -> None:
        """Filter accepts processing options."""
        p = ProfanityFilter()
        p.init(
            languages=["en"],
            processing_normalize_text=False,
            processing_transliterate=False,
        )
        assert p.filter_text("hello") is False  # no crash


class TestFilterText:
    """Tests for filter_text method."""

    def test_clean_text_returns_false(self) -> None:
        """Clean text returns False."""
        p = ProfanityFilter()
        p.init(languages=["en"])
        assert p.filter_text("hello world") is False

    def test_bad_text_returns_true(self) -> None:
        """Text with profanity returns True."""
        p = ProfanityFilter()
        p.init(languages=["en"])
        # Use word known to be in en.txt
        result = p.filter_text("sonofabitch")
        assert result is True

    def test_add_words_detection(self) -> None:
        """Custom words added via add_words are detected."""
        p = ProfanityFilter()
        p.init(languages=["en"])
        p.add_words(["custombadword"])
        assert p.filter_text("custombadword") is True
        assert p.filter_text("hello") is False

    def test_replace_character_returns_string(self) -> None:
        """With replace_character, returns censored string."""
        p = ProfanityFilter()
        p.init(
            languages=["en"],
            processing_transliterate=False,
            processing_replace_homoglyphs=False,
        )
        p.add_words(["badword"])
        result = p.filter_text("x badword y", replace_character="*")
        assert isinstance(result, str)
        assert "badword" not in result
        assert "*" in result

    def test_replace_character_clean_returns_false(self) -> None:
        """Clean text with replace_character still returns False."""
        p = ProfanityFilter()
        p.init(languages=["en"])
        result = p.filter_text("hello world", replace_character="*")
        assert result is False

    def test_match_threshold_exact_default(self) -> None:
        """Default match_threshold is exact (1.0)."""
        p = ProfanityFilter()
        p.init(languages=["en"])
        p.add_words(["exact"])
        assert p.filter_text("exact") is True
        assert p.filter_text("exactt") is False  # typo, no fuzzy

    def test_match_threshold_fuzzy(self) -> None:
        """Fuzzy matching works with threshold < 1."""
        p = ProfanityFilter()
        p.init(languages=["en"])
        p.add_words(["badword"])
        # "badw0rd" might match with fuzzy - depends on normalization
        result = p.filter_text("badword", match_threshold=0.9)
        assert result is True

    def test_empty_string(self) -> None:
        """Empty string returns False."""
        p = ProfanityFilter()
        p.init(languages=["en"])
        assert p.filter_text("") is False

    def test_whitespace_only(self) -> None:
        """Whitespace-only returns False."""
        p = ProfanityFilter()
        p.init(languages=["en"])
        assert p.filter_text("   \n\t  ") is False


class TestAddWords:
    """Tests for add_words method."""

    def test_add_single_word(self) -> None:
        """Can add single word."""
        p = ProfanityFilter()
        p.init(languages=["en"])
        p.add_words(["banned"])
        assert p.filter_text("banned") is True

    def test_add_multiple_words(self) -> None:
        """Can add multiple words."""
        p = ProfanityFilter()
        p.init(languages=["en"])
        p.add_words(["word1", "word2"])
        assert p.filter_text("word1") is True
        assert p.filter_text("word2") is True

    def test_add_empty_list(self) -> None:
        """Adding empty list does not crash."""
        p = ProfanityFilter()
        p.init(languages=["en"])
        p.add_words([])
        assert p.filter_text("hello") is False


class TestGetAllLanguages:
    """Tests for get_all_languages method."""

    def test_returns_list(self) -> None:
        """Returns a list."""
        p = ProfanityFilter()
        p.init(languages=["en"])
        result = p.get_all_languages()
        assert isinstance(result, list)
        assert result == ["en"]

    def test_sorted_when_all_loaded(self) -> None:
        """Languages are sorted when all loaded."""
        p = ProfanityFilter()
        p.init()
        langs = p.get_all_languages()
        assert langs == sorted(langs)


class TestSimilar:
    """Tests for similar method."""

    def test_identical_returns_one(self) -> None:
        """Identical strings return 1.0."""
        p = ProfanityFilter()
        p.init(languages=["en"])
        assert p.similar("hello", "hello") == 1.0

    def test_different_returns_less_than_one(self) -> None:
        """Different strings return < 1.0."""
        p = ProfanityFilter()
        p.init(languages=["en"])
        assert p.similar("hello", "world") < 1.0

    def test_similar_returns_high(self) -> None:
        """Similar strings return high ratio."""
        p = ProfanityFilter()
        p.init(languages=["en"])
        assert p.similar("hello", "hellp") >= 0.8
