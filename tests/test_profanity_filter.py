"""Tests for the ProfanityFilter API."""

from __future__ import annotations

import pytest
from badwords import Match, Options, ProfanityFilter
from badwords.exceptions import NotSupportedLanguage


@pytest.fixture
def filter_en() -> ProfanityFilter:
    """Filter with English loaded."""
    p = ProfanityFilter()
    p.init(languages=["en"])
    return p


class TestInit:
    """Initialization."""

    def test_init_all_languages(self) -> None:
        """No language list loads every language."""
        p = ProfanityFilter()
        p.init()
        assert len(p.loaded_languages()) == 25
        assert "en" in p.loaded_languages()
        assert "ru" in p.loaded_languages()

    def test_init_specific_languages(self) -> None:
        """Only the requested languages are loaded."""
        p = ProfanityFilter()
        p.init(languages=["en", "ru"])
        assert p.loaded_languages() == ["en", "ru"]

    def test_available_is_independent_of_loaded(self) -> None:
        """Loading a subset does not shrink what is available."""
        p = ProfanityFilter()
        p.init(languages=["en"])
        assert p.loaded_languages() == ["en"]
        assert len(p.available_languages()) == 25

    def test_init_twice_widens_the_set(self) -> None:
        """A second init with more languages works.

        In 2.x this raised, because init overwrote the available list.
        """
        p = ProfanityFilter()
        p.init(languages=["en"])
        p.init(languages=["en", "ru"])
        assert p.loaded_languages() == ["en", "ru"]

    def test_load_languages_is_additive(self) -> None:
        """load_languages adds to what is already loaded."""
        p = ProfanityFilter()
        p.init(languages=["en"])
        p.load_languages(["ru"])
        assert p.loaded_languages() == ["en", "ru"]

    def test_unload_languages(self) -> None:
        """Unloading drops only that language's words."""
        p = ProfanityFilter()
        p.init(languages=["en", "ru"])
        p.unload_languages(["en"])
        assert p.loaded_languages() == ["ru"]
        assert p.is_profane("хуй")
        assert not p.is_profane("shit")

    def test_unsupported_language_raises(self) -> None:
        """An unknown code raises NotSupportedLanguage."""
        p = ProfanityFilter()
        with pytest.raises(NotSupportedLanguage):
            p.init(languages=["xx"])

    def test_partially_unsupported_raises(self) -> None:
        """One bad code fails the whole call."""
        p = ProfanityFilter()
        with pytest.raises(NotSupportedLanguage):
            p.init(languages=["en", "xx"])

    def test_processing_options(self) -> None:
        """Processing options are accepted."""
        p = ProfanityFilter()
        p.init(
            languages=["en"],
            processing_normalize_text=False,
            processing_transliterate=False,
        )
        assert p.is_profane("hello") is False

    def test_methods_before_init_raise(self) -> None:
        """Using a filter before init is an error."""
        p = ProfanityFilter()
        with pytest.raises(RuntimeError, match="not initialized"):
            p.is_profane("test")


class TestMatching:
    """is_profane, find and censor."""

    def test_clean_text(self, filter_en: ProfanityFilter) -> None:
        """Clean text is not flagged."""
        assert filter_en.is_profane("hello world") is False

    def test_profane_text(self, filter_en: ProfanityFilter) -> None:
        """Profanity is detected."""
        assert filter_en.is_profane("sonofabitch") is True

    def test_find_reports_position_and_language(self, filter_en: ProfanityFilter) -> None:
        """Find returns spans that index the original text."""
        text = "well, shit happens"
        matches = filter_en.find(text)
        assert len(matches) == 1
        found = matches[0]
        assert isinstance(found, Match)
        assert found.matched_text == "shit"
        assert text.encode()[found.start : found.end].decode() == "shit"
        assert found.language == "en"
        assert found.kind == "exact"
        assert found.score == 1.0

    def test_censor_keeps_punctuation(self, filter_en: ProfanityFilter) -> None:
        """Punctuation attached to a word survives censoring."""
        assert filter_en.censor("hey shit, ok") == "hey ****, ok"
        assert filter_en.censor("(shit)!", "#") == "(####)!"

    def test_censor_returns_input_when_clean(self, filter_en: ProfanityFilter) -> None:
        """Nothing to censor means the input comes back unchanged."""
        assert filter_en.censor("a clean sentence") == "a clean sentence"

    def test_is_profane_agrees_with_find(self, filter_en: ProfanityFilter) -> None:
        """The two entry points cannot disagree."""
        for text in ["hello", "shit", "", "   ", "a shit b"]:
            assert filter_en.is_profane(text) == bool(filter_en.find(text))

    def test_batch_matches_single(self, filter_en: ProfanityFilter) -> None:
        """Batch calls return what the single-text calls would."""
        texts = ["hello", "shit", "clean text"]
        assert filter_en.is_profane_many(texts) == [filter_en.is_profane(t) for t in texts]
        assert filter_en.censor_many(texts) == [filter_en.censor(t) for t in texts]
        assert filter_en.find_many(texts) == [filter_en.find(t) for t in texts]


class TestDictionary:
    """Custom words and the whitelist."""

    def test_add_words(self, filter_en: ProfanityFilter) -> None:
        """Added words are detected."""
        filter_en.add_words(["custombadword"])
        assert filter_en.is_profane("custombadword") is True
        assert filter_en.contains_word("custombadword")

    def test_remove_words(self, filter_en: ProfanityFilter) -> None:
        """Removed words stop being detected."""
        filter_en.add_words(["custombadword"])
        filter_en.remove_words(["custombadword"])
        assert filter_en.is_profane("custombadword") is False

    def test_clear_words(self, filter_en: ProfanityFilter) -> None:
        """Clearing empties the dictionary."""
        assert filter_en.word_count() > 0
        filter_en.clear_words()
        assert filter_en.word_count() == 0
        assert filter_en.is_profane("shit") is False

    def test_whitelist_suppresses_a_match(self, filter_en: ProfanityFilter) -> None:
        """A whitelisted word is never reported."""
        assert filter_en.is_profane("damn") is True
        filter_en.add_whitelist(["damn"])
        assert filter_en.is_profane("damn") is False
        assert filter_en.is_whitelisted("damn") is True

    def test_whitelist_can_be_cleared(self, filter_en: ProfanityFilter) -> None:
        """Clearing the whitelist restores matching."""
        filter_en.add_whitelist(["damn"])
        filter_en.clear_whitelist()
        assert filter_en.is_profane("damn") is True


class TestOptions:
    """Per-call options."""

    def test_detectors_are_off_by_default(self) -> None:
        """Evasion detection is opt-in."""
        p = ProfanityFilter()
        p.init(languages=[])
        p.add_words(["badword"])
        for text in ["you.badword", "baaadword", "b4dword"]:
            assert p.is_profane(text) is False

    def test_split_on_punctuation(self) -> None:
        """Inner punctuation becomes a separator when asked."""
        p = ProfanityFilter()
        p.init(languages=[])
        p.add_words(["badword"])
        assert p.is_profane("you.badword", Options(split_on_punctuation=True)) is True

    def test_collapse_repeats(self) -> None:
        """Stretched words are caught when asked."""
        p = ProfanityFilter()
        p.init(languages=[])
        p.add_words(["badword"])
        assert p.is_profane("baaadword", Options(collapse_repeats=True)) is True
        assert p.is_profane("bookkeeper", Options(collapse_repeats=True)) is False

    def test_leetspeak(self) -> None:
        """Digits read as letters when asked."""
        p = ProfanityFilter()
        p.init(languages=[])
        p.add_words(["badword"])
        assert p.is_profane("b4dword", Options(leetspeak=True)) is True
        assert p.is_profane("100k", Options(leetspeak=True)) is False

    def test_substring_mode(self) -> None:
        """Substring matching catches glued evasion."""
        p = ProfanityFilter()
        p.init(languages=[])
        p.add_words(["badword"])
        opts = Options(match_mode="substring")
        assert p.is_profane("xxbadwordxx", opts) is True
        assert p.is_profane("xxbadwordxx") is False

    def test_fuzzy_threshold(self) -> None:
        """Fuzzy matching catches typos."""
        p = ProfanityFilter()
        p.init(languages=[])
        p.add_words(["badword"])
        assert p.is_profane("badwrod", Options(match_threshold=0.9)) is True
        assert p.is_profane("badwrod") is False

    def test_max_matches(self, filter_en: ProfanityFilter) -> None:
        """max_matches truncates the result."""
        assert len(filter_en.find("shit fuck bitch", Options(max_matches=1))) == 1

    def test_invalid_match_mode(self, filter_en: ProfanityFilter) -> None:
        """An unknown mode is rejected."""
        with pytest.raises(ValueError, match="match_mode"):
            filter_en.is_profane("x", Options(match_mode="nonsense"))  # type: ignore[arg-type]

    def test_default_options_can_be_set(self) -> None:
        """A filter can carry its own defaults."""
        p = ProfanityFilter()
        p.init(languages=[], options=Options(collapse_repeats=True))
        p.add_words(["badword"])
        assert p.is_profane("baaadword") is True
        assert p.options.collapse_repeats is True


class TestUtility:
    """similar and normalize."""

    def test_similar(self, filter_en: ProfanityFilter) -> None:
        """Similarity is 1.0 for identical strings."""
        assert filter_en.similar("hello", "hello") == 1.0
        assert filter_en.similar("hello", "hellp") >= 0.8
        assert filter_en.similar("abc", "xyz") < 1.0

    def test_normalize(self, filter_en: ProfanityFilter) -> None:
        """Normalize exposes the matcher's canonical form."""
        assert filter_en.normalize("FUCK") == filter_en.normalize("fuck")
        assert filter_en.normalize("") == ""


class TestDeprecated:
    """The 2.x API keeps working."""

    def test_filter_text_returns_bool(self, filter_en: ProfanityFilter) -> None:
        """Without a replacement character it returns a bool."""
        with pytest.deprecated_call():
            assert filter_en.filter_text("hello world") is False
        with pytest.deprecated_call():
            assert filter_en.filter_text("sonofabitch") is True

    def test_filter_text_censors_whole_tokens(self, filter_en: ProfanityFilter) -> None:
        """Censoring keeps its 2.x behaviour: the whole token goes."""
        with pytest.deprecated_call():
            assert filter_en.filter_text("hey shit, ok", replace_character="*") == "hey ***** ok"

    def test_filter_text_returns_false_when_clean(self, filter_en: ProfanityFilter) -> None:
        """Clean text returns False even with a replacement character."""
        with pytest.deprecated_call():
            assert filter_en.filter_text("all clean", replace_character="*") is False

    def test_get_all_languages(self, filter_en: ProfanityFilter) -> None:
        """get_all_languages still reports the loaded languages."""
        with pytest.deprecated_call():
            assert filter_en.get_all_languages() == ["en"]
