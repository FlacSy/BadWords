#!/usr/bin/env python3
"""Basic usage of badwords (Python).

Run: python examples/python/basic.py
     or: python -m examples.python.basic (from project root)
"""

from badwords import ProfanityFilter

def main() -> None:
    p = ProfanityFilter()
    p.init(languages=["en", "ru"])

    # Check clean text
    print("'hello world' contains profanity:", p.filter_text("hello world"))

    # Check text with profanity
    print("'sonofabitch' contains profanity:", p.filter_text("sonofabitch"))

    # Add custom words
    p.add_words(["custombad"])
    print("'custombad' (custom) contains profanity:", p.filter_text("custombad"))

    # Censoring
    p.init(languages=["en"], processing_transliterate=False, processing_replace_homoglyphs=False)
    p.add_words(["bad"])
    result = p.filter_text("a bad word", replace_character="*")
    print("Censored:", result)

    print("\nAvailable languages:", p.get_all_languages())

if __name__ == "__main__":
    main()
