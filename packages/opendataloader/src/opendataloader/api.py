"""Local PDF conversion through the bundled native library."""

import ctypes
import os
from functools import singledispatch
from pathlib import Path
from threading import RLock
from typing import Literal

_CHAR_POINTER = ctypes.c_char_p
_CHAR_POINTERS = ctypes.POINTER(_CHAR_POINTER)
_OPTION_JSON = 1
_OPTION_MARKDOWN = 2
_STATUS_OK = 0
_STATUS_INVALID_ARGUMENT = 1
_STATUS_CONVERSION_ERROR = 2
_STATUS_PANIC = 3
_LIBRARY: ctypes.CDLL | None = None
_LIBRARY_LOCK = RLock()



type _OutputFormatT = Literal["json", "markdown"]


@singledispatch
def _paths(input_path: object) -> list[Path]:
    raise TypeError("input_path must be a str, Path, or list of either")


@_paths.register
def _(input_path: str) -> list[Path]:
    return [Path(input_path)]


@_paths.register
def _(input_path: Path) -> list[Path]:
    return [input_path]


@_paths.register(list)
def _(input_paths: list[str | Path]) -> list[Path]:
    return [Path(input_path) for input_path in input_paths]


def _load_library() -> ctypes.CDLL:
    global _LIBRARY
    with _LIBRARY_LOCK:
        if _LIBRARY is not None:
            return _LIBRARY
        library_path = Path(
            os.environ.get("OPENDATALOADER_PDF_LIBRARY", Path(__file__).with_name("libodl.so"))
        )
        try:
            library = ctypes.CDLL(library_path)
        except OSError as error:
            raise OSError(f"unable to load native library {library_path}: {error}") from error
        library.odl_convert.argtypes = [_CHAR_POINTERS, ctypes.c_uint, _CHAR_POINTER, ctypes.c_uint]
        library.odl_convert.restype = ctypes.c_int
        library.odl_last_error.argtypes = []
        library.odl_last_error.restype = _CHAR_POINTER
        _LIBRARY = library
        return library


def _native_error(library: ctypes.CDLL) -> str:
    message = library.odl_last_error()
    return message.decode("utf-8", errors="replace") if message else "native conversion failed"


def convert(
    input_path: str | Path | list[str | Path],
    output_dir: str | Path,
    format: set[_OutputFormatT] | list[_OutputFormatT] | None = None,
) -> None:
    paths = _paths(input_path)
    if not paths:
        raise ValueError("input_path must contain at least one PDF path")
    for path in paths:
        if not path.is_file():
            raise FileNotFoundError(path)

    formats = {"json"} if format is None else set(format)
    if not formats or not formats <= {"json", "markdown"}:
        raise ValueError("format must contain only 'json' and 'markdown'")
    destination = Path(output_dir)
    destination.mkdir(parents=True, exist_ok=True)
    mode = (_OPTION_JSON if "json" in formats else 0) | (
        _OPTION_MARKDOWN if "markdown" in formats else 0
    )
    encoded_paths = [str(path).encode() for path in paths]
    native_paths = (_CHAR_POINTER * len(encoded_paths))(
        *(ctypes.c_char_p(path) for path in encoded_paths)
    )
    native_output = str(destination).encode()
    library = _load_library()
    with _LIBRARY_LOCK:
        status = library.odl_convert(native_paths, len(paths), native_output, mode)
        if status == _STATUS_OK:
            return None
        detail = _native_error(library)
    if status == _STATUS_INVALID_ARGUMENT:
        raise ValueError(detail)
    if status == _STATUS_CONVERSION_ERROR:
        raise OSError(f"conversion failed for {paths}: {detail}")
    if status == _STATUS_PANIC:
        raise RuntimeError(f"native conversion panicked for {paths}: {detail}")
    raise RuntimeError(f"unknown native conversion status {status}: {detail}")
