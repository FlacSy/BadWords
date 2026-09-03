"""Load every language at once.

Run: python -m examples.python.all_languages
"""

from __future__ import annotations

from badwords import Options, ProfanityFilter


def main() -> None:
    """Load all languages and check a few phrases."""
    p = ProfanityFilter()
    p.init()

    languages = p.loaded_languages()
    print(f"{len(languages)} languages loaded, {p.word_count()} entries")
    print(", ".join(languages))

    phrases = [
        "hello world",
        "das ist scheisse",
        "eres un gilipollas",
        "che cazzo fai",
        "quelle connerie",
        "ты полный мудак",
    ]
    for phrase in phrases:
        matches = p.find(phrase)
        if matches:
            langs = ", ".join(sorted({m.language or "custom" for m in matches}))
            print(f"  BLOCKED ({langs}): {p.censor(phrase)}")
        else:
            print(f"  OK:                {phrase}")

    # Fuzzy matching catches deliberate typos.
    p.add_words(["badword"])
    print("\nfuzzy 'badw0rd':", p.is_profane("badw0rd", Options(match_threshold=0.9)))


if __name__ == "__main__":
    main()
