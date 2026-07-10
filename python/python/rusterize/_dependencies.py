from __future__ import annotations

import importlib.util
import sys
from functools import cache
from typing import TYPE_CHECKING, Any


class _Missing:
    """Placeholder for an uninstalled optional dependency"""

    def __init__(self, name: str):
        self._name = name

    def __getattr__(self, attr: str) -> Any:
        raise ModuleNotFoundError(f"{attr} requires the {self._name!r} module to be installed.")


def _lazy_import(name: str) -> tuple[Any, bool]:
    """
    Lazy-import `name`. Real module loads on first access.
    Returns (module_or_placeholder, available).
    """
    if name in sys.modules:
        return sys.modules[name], True
    try:
        spec = importlib.util.find_spec(name)
    except ModuleNotFoundError:
        spec = None

    if spec is None or spec.loader is None:
        return _Missing(name), False

    spec.loader = importlib.util.LazyLoader(spec.loader)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module, True


if TYPE_CHECKING:
    import geopandas
    import polars
    import polars_st
    import xarray
else:
    geopandas, GEOPANDAS_AVAILABLE = _lazy_import("geopandas")
    polars, POLARS_AVAILABLE = _lazy_import("polars")
    polars_st, POLARS_ST_AVAILABLE = _lazy_import("polars_st")
    xarray, XARRAY_AVAILABLE = _lazy_import("xarray")
    _, RIOXARRAY_AVAILABLE = _lazy_import("rioxarray")


def _xarray_available() -> bool:
    return XARRAY_AVAILABLE and RIOXARRAY_AVAILABLE


def _polars_available() -> bool:
    return POLARS_AVAILABLE


@cache
def _might_be(cls: type, type_: str) -> bool:
    """Infer if a class hierarchy contains a specific module name."""
    try:
        return any(f"{type_}." in str(o) for o in cls.mro())
    except TypeError:
        return False


def _check_for_geopandas(obj: Any) -> bool:
    return GEOPANDAS_AVAILABLE and _might_be(type(obj), "geopandas")


def _check_for_polars_st(obj: Any) -> bool:
    return POLARS_ST_AVAILABLE and _might_be(type(obj), "polars")


__all__ = [
    "_check_for_geopandas",
    "_check_for_polars_st",
    "_xarray_available",
    "geopandas",
    "polars",
    "xarray",
]
