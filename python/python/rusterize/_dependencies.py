# Adapted from https://github.com/pola-rs/polars/blob/628f2273373e1d61d68c6b381e2d67702c990a0c/py-polars/src/polars/_dependencies.py

import re
import sys
from functools import cache
from importlib import import_module
from importlib.util import find_spec
from types import ModuleType
from typing import TYPE_CHECKING, Any, ClassVar


class _LazyModule(ModuleType):
    """Module that can act both as a lazy-loader and as a proxy."""

    __lazy__ = True

    _mod_pfx: ClassVar[dict[str, str]] = {
        "geopandas": "gpd.",
        "polars": "pl.",
        "polars_st": "st.",
        "xarray": "xr.",
    }

    def __init__(
        self,
        module_name: str,
        *,
        module_available: bool,
    ):
        """
        Initialise lazy-loading proxy module.

        Parameters
        ----------
        module_name : str
            the name of the module to lazy-load (if available).

        module_available : bool
            indicate if the referenced module is actually available (we will proxy it
            in both cases, but raise a helpful error when invoked if it doesn't exist).
        """
        self._module_available = module_available
        self._module_name = module_name
        self._globals = globals()
        super().__init__(module_name)

    def _import(self) -> ModuleType:
        # import the referenced module, replacing the proxy in this module's globals
        module = import_module(self.__name__)
        self._globals[self._module_name] = module
        self.__dict__.update(module.__dict__)
        return module

    def __getattr__(self, name: str) -> Any:
        # have "hasattr('__wrapped__')" return False without triggering import
        if name == "__wrapped__":
            msg = f"{self._module_name!r} object has no attribute {name!r}"
            raise AttributeError(msg)

        # accessing the proxy module's attributes triggers import of the real thing
        if self._module_available:
            # import the module and return the requested attribute
            module = self._import()
            return getattr(module, name)

        # user has not installed the proxied/lazy module
        elif name == "__name__":
            return self._module_name
        elif re.match(r"^__\w+__$", name) and name != "__version__":
            # allow some minimal introspection on private module
            # attrs to avoid unnecessary error-handling elsewhere
            return None
        else:
            # all other attribute access raises a helpful exception
            pfx = self._mod_pfx.get(self._module_name, "")
            msg = f"{pfx}{name} requires {self._module_name!r} module to be installed"
            raise ModuleNotFoundError(msg) from None


def _lazy_import(module_name: str) -> tuple[ModuleType, bool]:
    """
    Lazy import the given module; avoids up-front import costs.

    Parameters
    ----------
    module_name : str
        name of the module to import, eg: "pyarrow".

    Notes
    -----
    If the requested module is not available (eg: has not been installed), a proxy
    module is created in its place, which raises an exception on any attribute
    access. This allows for import and use as normal, without requiring explicit
    guard conditions - if the module is never used, no exception occurs; if it
    is, then a helpful exception is raised.

    Returns
    -------
    tuple of (Module, bool)
        A lazy-loading module and a boolean indicating if the requested/underlying
        module exists (if not, the returned module is a proxy).
    """
    # check if module is LOADED
    if module_name in sys.modules:
        return sys.modules[module_name], True

    # check if module is AVAILABLE
    try:
        module_spec = find_spec(module_name)
        module_available = not (module_spec is None or module_spec.loader is None)
    except ModuleNotFoundError:
        module_available = False

    # create lazy/proxy module that imports the real one on first use
    # (or raises an explanatory ModuleNotFoundError if not available)
    return (
        _LazyModule(
            module_name=module_name,
            module_available=module_available,
        ),
        module_available,
    )


if TYPE_CHECKING:
    import geopandas
    import polars
    import polars_st
    import rioxarray
    import xarray
else:
    geopandas, GEOPANDAS_AVAILABLE = _lazy_import("geopandas")
    polars, POLARS_AVAILABLE = _lazy_import("polars")
    polars_st, POLARS_ST_AVAILABLE = _lazy_import("polars_st")
    xarray, XARRAY_AVAILABLE = _lazy_import("xarray")
    rioxarray, RIOXARRAY_AVAILABLE = _lazy_import("rioxarray")


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
