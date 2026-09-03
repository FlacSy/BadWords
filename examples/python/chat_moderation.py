"""Chat moderation.

Run: python -m examples.python.chat_moderation
"""

from __future__ import annotations

from badwords import ProfanityFilter


def main() -> None:
    """Moderate a handful of messages."""
    p = ProfanityFilter()
    p.init(languages=["en", "ru"])
    p.add_words(["spam_link", "scam_bot"])
    # Words that must never be flagged, whatever a rule says.
    p.add_whitelist(["assessment"])

    messages = [
        "Hey! Check out this cool link",
        "Hello, how are you?",
        "Visit spam_link for free stuff",
        "This is scam_bot trying to reach you",
        "your assessment was wrong",
    ]

    # One call, one GIL release, one scratch buffer.
    for message, blocked in zip(messages, p.is_profane_many(messages), strict=True):
        if blocked:
            print(f"[BLOCKED] {p.censor(message)}")
        else:
            print(f"[OK]      {message}")


if __name__ == "__main__":
    main()
