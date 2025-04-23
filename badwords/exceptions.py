"""Exceptions module."""

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from typing import Self
else:
    Self = "NotSupportedLanguage"


class NotSupportedLanguage(BaseException):
    """Unsupport language check."""

    def __str__(self: Self) -> str:
        """String-like representation of exception."""
        return "This language is not supported"
