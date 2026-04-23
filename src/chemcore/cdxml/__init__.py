from __future__ import annotations

from typing import TYPE_CHECKING, Any

from .cdxml_fragment_display import extract_display_fragments

if TYPE_CHECKING:
    from .extract_cdxml import ExtractResult

__all__ = [
    "ExtractResult",
    "extract_display_fragments",
    "extract_cdxml",
]


def extract_cdxml(*args: Any, **kwargs: Any):
    from .extract_cdxml import extract_cdxml as _extract_cdxml

    return _extract_cdxml(*args, **kwargs)
