"""Exceptions module."""

from typing import Self


class NotSupportedLanguage(BaseException):
    """Unsupport language check."""

    def __str__(self: Self) -> str:
        """String-like representation of exception."""
        return "This language is not supported"
