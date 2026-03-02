#!/usr/bin/env python3
"""Chat moderation example.

Run: python examples/python/chat_moderation.py
"""

from badwords import ProfanityFilter


def main() -> None:
    p = ProfanityFilter()
    p.init(languages=["en", "ru"])
    p.add_words(["spam_link", "scam_bot"])

    messages = [
        "Hey! Check out this cool link",
        "Hello, how are you?",
        "Visit spam_link for free stuff",
        "This is scam_bot trying to reach you",
    ]

    for msg in messages:
        is_bad = p.filter_text(msg)
        status = "BLOCKED" if is_bad else "OK"
        print(f"[{status}] {msg}")


if __name__ == "__main__":
    main()
