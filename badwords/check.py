"""Module for checking text for badwords."""

from __future__ import annotations

from difflib import SequenceMatcher
from pathlib import Path
from typing import TYPE_CHECKING

from .exceptions import NotSupportedLanguage
from .text_processor import TextProcessor

if TYPE_CHECKING:
    from typing import Self
else:
    Self = "NotSupportedLanguage"


class ProfanityFilter:
    """A class for filtering profanity from text."""

    def init(
        self: Self,
        languages: list[str] | None = None,
        processing_normalize_text: bool = True,
        processing_aggressive_normalize: bool = True,
        processing_transliterate: bool = True,
        processing_replace_homoglyphs: bool = True,
    ) -> None:
        """Initialize the profanity filter.

        :param languages: List of languages to load profanity words for.
        :param processing_normalize_text: Enable text normalization for all loaded words and word to filter
        :param processing_aggressive_normalize: Enable aggressive text normalization for all loaded words and word to filter
        :param processing_transliterate: Enable transliteration of text for all loaded words and word to filter
        :param processing_replace_homoglyphs: Enable replacing of homoglyphs in the text for all loaded words and word to filter
        """
        self.resource_dir = Path(__file__).parent / "resource"

        self.text_processor = TextProcessor(
            processing_normalize_text=processing_normalize_text,
            processing_aggressive_normalize=processing_aggressive_normalize,
            processing_transliterate=processing_transliterate,
            processing_replace_homoglyphs=processing_replace_homoglyphs
        )

        self.language_files = self.initialize_language_files()

        if languages:
            if all(i in self.language_files for i in languages):
                self.language_files = languages
            else:
                raise NotSupportedLanguage

        self.bad_words = self.initialize_bad_words()

    def initialize_language_files(self: Self) -> list[str]:
        """Initialize language files.

        :return: Dictionary mapping language names to file paths.
        """
        return [str(path)[-6:-4] for path in (self.resource_dir).iterdir()]

    def initialize_bad_words(self: Self) -> set[str]:
        """Initialize the set of bad words from language files."""
        bad_words: set[str] = set()
        
        for lang in self.language_files:
            try:
                # Sanitize the language code to prevent path traversal
                lang = lang.lower().strip()
                if not lang.isalpha():
                    continue
                    
                file_path = self.resource_dir / f"{lang}.bdw"
                if not file_path.exists():
                    continue
                    
                with file_path.open(encoding="utf-8") as f:
                    words = f.read().splitlines()
                    processed_words = [self.text_processor.process_text(word) for word in words]
                    bad_words.update(processed_words)
            except Exception as e:
                print(f"Error loading language file for {lang}: {e}")
                continue
                
        return bad_words

    def add_words(self: Self, words: list[str]) -> None:
        """Add custom profanity words to the filter.

        :param words: List of custom profanity words.
        """
        processed_words = [self.text_processor.process_text(word) for word in words]
        self.bad_words.update(processed_words)

    def similar(self: Self, a: str, b: str) -> float:
        """Compute similarity ratio between two strings.

        :param a: First string.
        :param b: Second string.
        :return: Similarity ratio.
        """
        return SequenceMatcher(None, a, b).ratio()

    def filter_text(
            self: Self, text: str,
            match_threshold: float | None = None,
            replace_character: str | None = None,
        ) -> bool | str:
        """Check if the given text contains profanity.

        :param text: Input text to check.
        :param match_threshold: Threshold for similarity match.
        :param replace_character: Character to replace profane words with. If None,
            return True/False.
        :return: True if profanity found, False otherwise. If replace_character is
            specified, return filtered text.
        """
        if not match_threshold:
            match_threshold = 1

        # Process the input text through all transformations
        processed_text = self.text_processor.process_text(text)
        words = processed_text.split()

        for word in words:
            # Check exact match
            if word in self.bad_words:
                if replace_character:
                    return text.replace(word, replace_character * len(word))
                return True

            # Check similar matches if threshold is less than 1
            if 0 < match_threshold < 1:
                for bad_word in self.bad_words:
                    if self.similar(word, bad_word) > match_threshold:
                        if replace_character:
                            return text.replace(word, replace_character * len(word))
                        return True

        return False

    def get_all_languages(self: Self) -> list[str]:
        """Get a list of all available languages.

        :return: List of all language names.
        """
        return self.language_files
