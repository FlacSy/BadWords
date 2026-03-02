"""Tests for exceptions."""

from __future__ import annotations

import pytest

from badwords.exceptions import NotSupportedLanguage


class TestNotSupportedLanguage:
    """Tests for NotSupportedLanguage exception."""

    def test_is_base_exception_subclass(self) -> None:
        """NotSupportedLanguage inherits from BaseException."""
        assert issubclass(NotSupportedLanguage, BaseException)

    def test_str_representation(self) -> None:
        """String representation returns expected message."""
        exc = NotSupportedLanguage()
        assert str(exc) == "This language is not supported"

    def test_can_be_raised_and_caught(self) -> None:
        """Exception can be raised and caught."""
        with pytest.raises(NotSupportedLanguage) as exc_info:
            raise NotSupportedLanguage()
        assert exc_info.value is not None
