#!/usr/bin/env python3
"""Load all available languages.

Run: python examples/python/all_languages.py
"""

from badwords import ProfanityFilter


def main() -> None:
    p = ProfanityFilter()
    p.init()  # No languages = load all

    langs = p.get_all_languages()
    print(f"Loaded {len(langs)} languages: {langs}")

    # Fuzzy matching
    p.add_words(["badword"])
    print(
        "\nFuzzy match 'badw0rd' (threshold=0.9):",
        p.filter_text("badw0rd", match_threshold=0.9),
    )


if __name__ == "__main__":
    main()
