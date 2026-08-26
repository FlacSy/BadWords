"""Basic usage.

Run: python -m examples.python.basic
"""

from __future__ import annotations

from badwords import Options, ProfanityFilter


def main() -> None:
    """Show the core API."""
    p = ProfanityFilter()
    p.init(languages=["en", "ru"])

    print("clean text:  ", p.is_profane("hello world"))
    print("profane text:", p.is_profane("sonofabitch"))

    p.add_words(["custombad"])
    print("custom word: ", p.is_profane("custombad"))

    # Censoring keeps everything that is not part of a match.
    print("censored:    ", p.censor("hey shit, ok"))

    # find() says what matched, where, and in which language.
    for match in p.find("what a shitty, damn mess"):
        print(
            f"  {match.matched_text!r} at {match.start}..{match.end} "
            f"({match.word!r}, {match.language}, {match.kind})"
        )

    # Evasion detection is opt-in, because each detector costs false positives.
    strict = Options(split_on_punctuation=True, collapse_repeats=True, leetspeak=True)
    for text in ["shiiit", "you.shit", "sh1t"]:
        print(f"  {text!r:12} default={p.is_profane(text)} strict={p.is_profane(text, strict)}")

    print("loaded languages:", p.loaded_languages())


if __name__ == "__main__":
    main()
