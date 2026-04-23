from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from .cdxml.extract_cdxml import ExtractResult

__all__ = [
    "ExtractResult",
    "extract_cdxml",
]


def extract_cdxml(*args: Any, **kwargs: Any):
    from .cdxml import extract_cdxml as _extract_cdxml

    return _extract_cdxml(*args, **kwargs)
