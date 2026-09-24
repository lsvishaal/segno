#!/usr/bin/env python3
"""Regenerates the SVG fixtures the score-geometry tests read.

Renders page 1 of each real bundled piece with the real Verovio, using the
same options and identifier seed the app uses, and writes beside it a map of
note id -> written step and octave taken from the same render's MEI.

    python3 scripts/score-fixtures.py [libverovio.so] [verovio/data]
"""

import ctypes
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
LIB = sys.argv[1] if len(sys.argv) > 1 else str(ROOT / "../verovio-build/libverovio.so")
DATA = sys.argv[2] if len(sys.argv) > 2 else str(ROOT / "../verovio/data")
OUT = ROOT / "app/src/components/__fixtures__"
PIECES = ["minuet-in-g", "prelude-in-c"]
# Keep in step with Layout::as_options in crates/hogwash-engraving-verovio.
OPTIONS = (
    '{"pageWidth":1400,"scale":100,"adjustPageHeight":true,"breaks":"auto",'
    '"header":"none","footer":"none","svgViewBox":true}'
)
ID_SEED = 1

lib = ctypes.CDLL(LIB)
lib.vrvToolkit_constructorResourcePath.restype = ctypes.c_void_p
lib.vrvToolkit_constructorResourcePath.argtypes = [ctypes.c_char_p]
lib.vrvToolkit_setOptions.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
lib.vrvToolkit_resetXmlIdSeed.argtypes = [ctypes.c_void_p, ctypes.c_int]
lib.vrvToolkit_loadData.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
lib.vrvToolkit_loadData.restype = ctypes.c_bool
lib.vrvToolkit_renderToSVG.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_bool]
lib.vrvToolkit_renderToSVG.restype = ctypes.c_char_p
lib.vrvToolkit_getMEI.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
lib.vrvToolkit_getMEI.restype = ctypes.c_char_p

OUT.mkdir(parents=True, exist_ok=True)

for piece in PIECES:
    toolkit = ctypes.c_void_p(lib.vrvToolkit_constructorResourcePath(DATA.encode()))
    lib.vrvToolkit_setOptions(toolkit, OPTIONS.encode())
    lib.vrvToolkit_resetXmlIdSeed(toolkit, ID_SEED)
    source = (ROOT / f"app/src-tauri/library/{piece}.musicxml").read_bytes()
    if not lib.vrvToolkit_loadData(toolkit, source):
        sys.exit(f"Verovio refused {piece}")

    svg = lib.vrvToolkit_renderToSVG(toolkit, 1, False).decode()
    mei = lib.vrvToolkit_getMEI(toolkit, b'{"noLayout": true}').decode()

    notes = {}
    for match in re.finditer(r"<note\b([^>]*)>", mei):
        attributes = dict(re.findall(r'([\w:.]+)="([^"]*)"', match.group(1)))
        if "xml:id" in attributes and "pname" in attributes and "oct" in attributes:
            notes[attributes["xml:id"]] = {
                "step": attributes["pname"].upper(),
                "octave": int(attributes["oct"]),
            }

    drawn = set(re.findall(r'<g id="([^"]+)" class="note"', svg))
    page_notes = {id: spelling for id, spelling in notes.items() if id in drawn}

    (OUT / f"{piece}.page1.svg").write_text(svg)
    (OUT / f"{piece}.page1.notes.json").write_text(json.dumps(page_notes, indent=0, sort_keys=True) + "\n")
    print(f"{piece}: {len(svg)} bytes, {len(page_notes)} notes on page 1")
