"""Benchmarks for ProfanityFilter."""

from __future__ import annotations

import pytest

from badwords import ProfanityFilter


@pytest.fixture(scope="module")
def filter_en_ru() -> ProfanityFilter:
    p = ProfanityFilter()
    p.init(languages=["en", "ru"])
    return p


@pytest.mark.benchmark
def test_bench_filter_clean(
    benchmark: pytest.BenchmarkFixture, filter_en_ru: ProfanityFilter
) -> None:
    """Benchmark: clean text (no profanity)."""
    benchmark(filter_en_ru.filter_text, "Hello, this is a clean message for testing.")


@pytest.mark.benchmark
def test_bench_filter_bad(
    benchmark: pytest.BenchmarkFixture, filter_en_ru: ProfanityFilter
) -> None:
    """Benchmark: text with profanity."""
    benchmark(filter_en_ru.filter_text, "sonofabitch")


@pytest.fixture(scope="module")
def filter_with_custom_word() -> ProfanityFilter:
    p = ProfanityFilter()
    p.init(languages=["en"])
    p.add_words(["badword"])
    return p


@pytest.mark.benchmark
def test_bench_filter_censor(
    benchmark: pytest.BenchmarkFixture, filter_with_custom_word: ProfanityFilter
) -> None:
    """Benchmark: censor profanity with replace_character."""
    benchmark(
        filter_with_custom_word.filter_text,
        "x badword y",
        replace_character="*",
    )


@pytest.mark.benchmark
def test_bench_filter_many(
    benchmark: pytest.BenchmarkFixture, filter_en_ru: ProfanityFilter
) -> None:
    """Benchmark: multiple texts in sequence."""
    texts = [
        "Hello world",
        "Clean message",
        "Some text with potential",
        "Another clean one",
        "Final test string",
    ]

    def run() -> None:
        for text in texts:
            filter_en_ru.filter_text(text)

    benchmark(run)
