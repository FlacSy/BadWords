"""Exceptions module."""


class NotSupportedLanguage(Exception):
    """Unsupported language check."""

    def __str__(self) -> str:
        """String-like representation of exception."""
        return "This language is not supported"
