"""Local PDF conversion through ``libodl_pdf.so``."""


import ctypes
import os
from contextlib import contextmanager
from functools import singledispatch
from pathlib import Path
from threading import RLock
from typing import Literal

_LIBRARY_PATH = Path(
    os.environ.get("OPENDATALOADER_PDF_LIBRARY", Path(__file__).with_name("libodl.so"))
)
_LIBRARY = ctypes.CDLL(_LIBRARY_PATH)

_CHAR_POINTER = ctypes.c_char_p
_CHAR_POINTERS = ctypes.POINTER(_CHAR_POINTER)



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


def convert(
    input_path: str | Path | list[str | Path],
    output_dir: str | Path,
    format: set[_OutputFormatT] | None = None,
) -> None:
   raise NotImplementedError()
