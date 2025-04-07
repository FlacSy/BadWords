import os
import re
from difflib import SequenceMatcher


class ProfanityFilter:
    """A class for filtering profanity from text.
    """

    def __init__(self, languages: list[str] = None, all_languages: bool = False):
        """Initialize the profanity filter.

        :param languages: List of languages to load profanity words for.
        :param all_languages: Flag to load profanity words for all available languages.
        """
        self.script_dir = os.path.dirname(os.path.realpath(__file__))
        self.language_files: dict[str, str] = self.initialize_language_files()
        self.languages = languages or list(self.language_files.keys()) if all_languages else languages
        self.bad_words: dict[str, set[str]] = self.initialize_bad_words()
        self.patterns: dict[str, re.Pattern] = self.compile_patterns()
        self.custom_bad_words: set[str] = set()

    def initialize_language_files(self) -> dict[str, str]:
        """Initialize language files.

        :return: Dictionary mapping language names to file paths.
        """
        resource_dir = os.path.join(self.script_dir, "resource")
        return {os.path.splitext(filename)[0]: os.path.join(resource_dir, filename) for filename in os.listdir(resource_dir)}

    def initialize_bad_words(self) -> dict[str, set[str]]:
        """Initialize profanity words for each language.

        :return: Dictionary mapping language names to sets of profanity words.
        """
        bad_words = {}
        for language in self.languages:
            file_path = self.language_files.get(language)
            if file_path:
                with open(file_path, encoding="utf-8") as file:
                    bad_words[language] = {line.strip() for line in file}
        return bad_words

    def compile_patterns(self) -> dict[str, re.Pattern]:
        """Compile regular expression patterns for profanity words.

        :return: Dictionary mapping language names to compiled regex patterns.
        """
        return {language: re.compile(r"\b(?:" + "|".join(map(re.escape, words)) + r")\b", re.IGNORECASE) for language, words in self.bad_words.items()}

    def add_words(self, words: list[str]):
        """Add custom profanity words to the filter.

        :param words: List of custom profanity words.
        """
        self.custom_bad_words.update(words)

    def similar(self, a: str, b: str) -> float:
        """Compute similarity ratio between two strings.

        :param a: First string.
        :param b: Second string.
        :return: Similarity ratio.
        """
        return SequenceMatcher(None, a, b).ratio()

    def filter_text(self, text: str, match_threshold: float = 0.8, replace_character=None):
        """Check if the given text contains profanity.

        :param text: Input text to check.
        :param match_threshold: Threshold for similarity match.
        :param replace_character: Character to replace profane words with. If None, return True/False.
        :return: True if profanity found, False otherwise. If replace_character is specified, return filtered text.
        """
        all_bad_words = set.union(self.custom_bad_words, *self.bad_words.values())

        words_in_text = text.lower().split(" ")
        filtered_text = text.lower()
        for word in words_in_text:
            for bad_word in all_bad_words:
                if self.similar(word, bad_word) > match_threshold:
                    if replace_character is not None:
                        filtered_text = filtered_text.replace(word, replace_character * len(word))
                    else:
                        return True if replace_character is None else filtered_text
        return False if replace_character is None else filtered_text

    def get_all_languages(self) -> list[str]:
        """Get a list of all available languages.

        :return: List of all language names.
        """
        return list(self.language_files.keys())
