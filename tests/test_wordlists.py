"""Tests for the shipped word lists and their normalization."""

from __future__ import annotations

from pathlib import Path

import pytest
from badwords import ProfanityFilter

WORDS_DIR = Path(__file__).parent.parent / "python" / "badwords" / "resource" / "words"

# Profanity that must be caught in English. Regression guard: before the word list
# was flattened, en.txt held regex patterns that the engine matched literally, so
# none of these were detected.
EN_MUST_DETECT = [
    "fuck",
    "fucking",
    "fucked",
    "motherfucker",
    "shit",
    "shitty",
    "bullshit",
    "dipshit",
    "asshole",
    "dumbass",
    "bitch",
    "bitches",
    "cunt",
    "dickhead",
    "cocksucker",
    "pussy",
    "whore",
    "slut",
    "twat",
    "wanker",
    "bastard",
    "douchebag",
    "fucktard",
    "nigger",
    "faggot",
    "retard",
    "goddamn",
]

# Ordinary English that must never be flagged. "disk", "sock", "sum" and friends
# guard the homoglyph/transliteration ordering: folding homoglyphs after
# transliteration turned latin "s" into "c", so "disk" matched "dick".
EN_MUST_NOT_DETECT = [
    "disk",
    "disks",
    "sock",
    "socks",
    "sum",
    "sums",
    "summer",
    "summing",
    "soon",
    "slit",
    "fuss",
    "pisces",
    "classic",
    "assassin",
    "grass",
    "analysis",
    "cocktail",
    "associate",
    "password",
    "document",
    "hello",
]


@pytest.fixture(scope="module")
def en_filter() -> ProfanityFilter:
    """Filter with English loaded and default processing options."""
    p = ProfanityFilter()
    p.init(languages=["en"])
    return p


@pytest.mark.parametrize("word", EN_MUST_DETECT)
def test_english_profanity_is_detected(en_filter: ProfanityFilter, word: str) -> None:
    """Common English profanity is detected."""
    assert en_filter.filter_text(word) is True


@pytest.mark.parametrize("word", EN_MUST_NOT_DETECT)
def test_ordinary_english_is_not_flagged(
    en_filter: ProfanityFilter,
    word: str,
) -> None:
    """Ordinary English words are not flagged."""
    assert en_filter.filter_text(word) is False


@pytest.mark.parametrize(
    ("clean", "profane"),
    [
        ("sock", "cock"),
        ("disk", "dick"),
        ("sum", "cum"),
        ("slit", "clit"),
        ("hat", "xat"),
    ],
)
def test_letters_are_not_conflated(clean: str, profane: str) -> None:
    """Normalization keeps s/c and h/x apart."""
    p = ProfanityFilter()
    p.init(languages=[])
    p.add_words([profane])
    assert p.filter_text(profane) is True
    assert p.filter_text(clean) is False


# Regex metacharacters. The engine matches word list entries literally after
# normalization, so a pattern like `schei(ss|ß)e?` is stripped to the junk token
# "scheissse" and can never match anything.
REGEX_METACHARACTERS = "[]()|?+*{}\\"


@pytest.mark.parametrize("path", sorted(WORDS_DIR.glob("*.txt")), ids=lambda p: p.stem)
def test_wordlist_has_no_regex_patterns(path: Path) -> None:
    """Word lists hold plain words, not regex patterns."""
    lines = path.read_text(encoding="utf-8-sig").splitlines()
    patterns = [line for line in lines if any(c in line for c in REGEX_METACHARACTERS)]
    assert patterns == []


@pytest.mark.parametrize("path", sorted(WORDS_DIR.glob("*.txt")), ids=lambda p: p.stem)
def test_wordlist_entries_match_themselves(path: Path) -> None:
    """Every entry survives normalization and is detected by its own language."""
    p = ProfanityFilter()
    p.init(languages=[path.stem])
    lines = [
        line.strip()
        for line in path.read_text(encoding="utf-8-sig").splitlines()
        if line.strip() and " " not in line.strip()
    ]
    assert [w for w in lines if p.filter_text(w) is not True] == []


@pytest.mark.parametrize("path", sorted(WORDS_DIR.glob("*.txt")), ids=lambda p: p.stem)
def test_wordlist_entries_are_single_tokens(path: Path) -> None:
    """Every entry is a single token; multi-word entries can never match."""
    lines = path.read_text(encoding="utf-8-sig").splitlines()
    multiword = [line for line in lines if line.strip() and " " in line.strip()]
    if path.stem in {"ko", "no"}:
        pytest.xfail(f"{path.stem}.txt still ships multi-word entries")
    assert multiword == []


def test_english_wordlist_is_plain_sorted_and_unique() -> None:
    """en.txt stays a sorted, deduplicated list of plain lowercase words."""
    lines = (WORDS_DIR / "en.txt").read_text(encoding="utf-8").splitlines()
    assert lines == sorted(lines)
    assert len(lines) == len(set(lines))
    assert all(line == line.strip().lower() for line in lines)
    assert all(line.isascii() and line.isalnum() for line in lines)


def test_english_wordlist_entries_all_match_themselves() -> None:
    """Every en.txt entry survives normalization and matches itself."""
    p = ProfanityFilter()
    p.init(languages=["en"])
    lines = (WORDS_DIR / "en.txt").read_text(encoding="utf-8").splitlines()
    assert [line for line in lines if p.filter_text(line) is not True] == []
