"""Module for checking text for badwords."""

from __future__ import annotations

import re
from difflib import SequenceMatcher
from pathlib import Path
from typing import Self


class ProfanityFilter:
    """A class for filtering profanity from text."""

    def __init__(self: Self,
        languages: list[str] | None = None,
        all_languages: bool | None = None,
    ) -> None:
        """Initialize the profanity filter.

        :param languages: List of languages to load profanity words for.
        :param all_languages: Flag to load profanity words for all available languages.
        """
        self.script_dir = Path(__file__).parent
        self.language_files: dict[str, str] = self.initialize_language_files()
        self.languages = languages or list(self.language_files.keys()) if\
            all_languages else languages
        self.bad_words: dict[str, set[str]] = self.initialize_bad_words()
        self.patterns: dict[str, re.Pattern] = self.compile_patterns()
        self.custom_bad_words: set[str] = set()

    def initialize_language_files(self: Self) -> dict[str, str]:
        """Initialize language files.

        :return: Dictionary mapping language names to file paths.
        """
        resource_dir = Path(self.script_dir) / "resource"

        result = {}

        for filename in Path(resource_dir).iterdir():
            path = Path(filename)
            root = path.parent / path.stem

            result[root] = Path(resource_dir) / filename

        return result

    def initialize_bad_words(self: Self) -> dict[str, set[str]]:
        """Initialize profanity words for each language.

        :return: Dictionary mapping language names to sets of profanity words.
        """
        bad_words = {}

        for language in self.languages:
            file_path = self.language_files.get(language)
            if file_path:
                with Path(file_path).open(encoding="utf-8") as file:
                    bad_words[language] = {line.strip() for line in file}

        return bad_words

    def compile_patterns(self: Self) -> dict[str, re.Pattern]:
        """Compile regular expression patterns for profanity words.

        :return: Dictionary mapping language names to compiled regex patterns.
        """
        result = {}

        for language, words in self.bad_words.items():
            result[language] = re.compile(
                r"\b(?:" + "|".join(map(re.escape, words)) + r")\b", re.IGNORECASE,
            )

    def add_words(self: Self, words: list[str]) -> None:
        """Add custom profanity words to the filter.

        :param words: List of custom profanity words.
        """
        self.custom_bad_words.update(words)

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

        all_bad_words = set.union(self.custom_bad_words, *self.bad_words.values())

        filtered_text = text.lower()
        words_in_text = filtered_text.split()

        for word in words_in_text:
            for bad_word in all_bad_words:
                if match_threshold != 1 and \
                  self.similar(word, bad_word) > match_threshold:
                    if replace_character:
                        return filtered_text.replace(word)
                    return True
        return False

    def get_all_languages(self: Self) -> list[str]:
        """Get a list of all available languages.

        :return: List of all language names.
        """
        return list(self.language_files.keys())
