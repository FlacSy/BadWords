"""Benchmarks.

Run with: make bench-python
"""

from __future__ import annotations

import pytest
from badwords import Options, ProfanityFilter

CLEAN = "Hello, this is a clean message for testing."
PROFANE = "sonofabitch"
BATCH = [
    "Hello world",
    "This is fine",
    "sonofabitch",
    "Another clean message",
    "Yet another one",
]


@pytest.fixture(scope="module")
def filter_en_ru() -> ProfanityFilter:
    """Filter with English and Russian loaded."""
    p = ProfanityFilter()
    p.init(languages=["en", "ru"])
    return p


@pytest.fixture(scope="module")
def filter_with_custom_word() -> ProfanityFilter:
    """Filter with one custom word, for predictable censoring."""
    p = ProfanityFilter()
    p.init(languages=["en"])
    p.add_words(["badword"])
    return p


@pytest.mark.benchmark
def test_bench_clean_text(benchmark, filter_en_ru: ProfanityFilter) -> None:
    """Clean text, exact matching."""
    benchmark(filter_en_ru.is_profane, CLEAN)


@pytest.mark.benchmark
def test_bench_profane_text(benchmark, filter_en_ru: ProfanityFilter) -> None:
    """Text with profanity, exact matching."""
    benchmark(filter_en_ru.is_profane, PROFANE)


@pytest.mark.benchmark
def test_bench_censor(benchmark, filter_with_custom_word: ProfanityFilter) -> None:
    """Censoring."""
    benchmark(filter_with_custom_word.censor, "x badword y", "*")


@pytest.mark.benchmark
def test_bench_find(benchmark, filter_en_ru: ProfanityFilter) -> None:
    """Collecting matches rather than short-circuiting."""
    benchmark(filter_en_ru.find, CLEAN)


@pytest.mark.benchmark
def test_bench_batch(benchmark, filter_en_ru: ProfanityFilter) -> None:
    """Five texts through the batch API."""
    benchmark(filter_en_ru.is_profane_many, BATCH)


@pytest.mark.benchmark
def test_bench_batch_one_by_one(benchmark, filter_en_ru: ProfanityFilter) -> None:
    """Five texts one call at a time, to show what the batch API saves."""
    benchmark(lambda: [filter_en_ru.is_profane(t) for t in BATCH])


@pytest.mark.benchmark
def test_bench_fuzzy(benchmark, filter_en_ru: ProfanityFilter) -> None:
    """Fuzzy matching, which the index made viable."""
    benchmark(filter_en_ru.is_profane, CLEAN, Options(match_threshold=0.9))
