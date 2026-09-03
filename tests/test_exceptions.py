"""The NotSupportedLanguage exception."""

from __future__ import annotations

import pytest
from badwords import ProfanityFilter
from badwords.exceptions import NotSupportedLanguage


def test_is_an_exception() -> None:
    """It is a real exception type."""
    assert issubclass(NotSupportedLanguage, Exception)


def test_raised_for_an_unknown_code() -> None:
    """An unknown language code raises it."""
    p = ProfanityFilter()
    with pytest.raises(NotSupportedLanguage):
        p.init(languages=["definitely-not-a-language"])


def test_message_names_the_code_and_the_alternatives() -> None:
    """The message says what failed and what is available.

    2.x raised a bare 'This language is not supported' with no clue which of
    the requested codes was the problem.
    """
    p = ProfanityFilter()
    with pytest.raises(NotSupportedLanguage) as excinfo:
        p.init(languages=["en", "xx"])

    message = str(excinfo.value)
    assert "xx" in message
    assert "en" in message


def test_catchable_as_value_error() -> None:
    """It subclasses ValueError, so existing except clauses keep working."""
    p = ProfanityFilter()
    with pytest.raises(ValueError, match="xx"):
        p.init(languages=["xx"])
