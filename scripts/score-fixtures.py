#!/usr/bin/env python3
"""Regenerates the SVG fixtures the score-geometry tests read.

Renders page 1 of each real bundled piece with the real Verovio, using the
same options and identifier seed the app uses, and writes beside it a map of
note id -> the step, octave and MIDI pitch it sounds, from the same render's
MEI. Also writes the Minuet with an 8va line over bars 1-2 of the right hand
(the same edit as the engraving test's `minuet_under_an_octave_line`).

    python3 scripts/score-fixtures.py [libverovio.so] [verovio/data]
"""

import ctypes
import json
import pathlib
import re
import sys
import xml.etree.ElementTree as ET

ROOT = pathlib.Path(__file__).resolve().parent.parent
LIB = sys.argv[1] if len(sys.argv) > 1 else str(ROOT / "../verovio-build/libverovio.so")
DATA = sys.argv[2] if len(sys.argv) > 2 else str(ROOT / "../verovio/data")
OUT = ROOT / "app/src/components/__fixtures__"
MEI = "{http://www.music-encoding.org/ns/mei}"
XML_ID = "{http://www.w3.org/XML/1998/namespace}id"
SEMITONES = {"c": 0, "d": 2, "e": 4, "f": 5, "g": 7, "a": 9, "b": 11}
ALTER = {"s": 1, "f": -1, "ss": 2, "x": 2, "ff": -2, "n": 0}


def under_an_octave_line(source):
    def shift(kind):
        return (f'<direction><direction-type><octave-shift type="{kind}" size="8" number="1"/>'
                "</direction-type><staff>1</staff></direction>")

    def before_first_note_of(text, bar):
        start = text.index(f'<measure number="{bar}"')
        return text.index("<note", start)

    at = before_first_note_of(source, "3")
    source = source[:at] + shift("stop") + source[at:]
    at = before_first_note_of(source, "1")
    return source[:at] + shift("down") + source[at:]


def library(name):
    return (ROOT / f"app/src-tauri/library/{name}.musicxml").read_text()


PIECES = {
    "minuet-in-g": library("minuet-in-g"),
    "prelude-in-c": library("prelude-in-c"),
    "minuet-in-g-8va": under_an_octave_line(library("minuet-in-g")),
}
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

for piece, source in PIECES.items():
    toolkit = ctypes.c_void_p(lib.vrvToolkit_constructorResourcePath(DATA.encode()))
    lib.vrvToolkit_setOptions(toolkit, OPTIONS.encode())
    lib.vrvToolkit_resetXmlIdSeed(toolkit, ID_SEED)
    if not lib.vrvToolkit_loadData(toolkit, source.encode()):
        sys.exit(f"Verovio refused {piece}")

    svg = lib.vrvToolkit_renderToSVG(toolkit, 1, False).decode()
    mei = lib.vrvToolkit_getMEI(toolkit, b'{"noLayout": true}').decode()

    notes = {}
    for note in ET.fromstring(mei).iter(f"{MEI}note"):
        if "pname" not in note.attrib or "oct" not in note.attrib:
            continue
        # Sounding, not drawn: @oct.ges under an octave line, @accid.ges from
        # the key signature, as the app's own reader does.
        octave = int(note.get("oct.ges", note.get("oct")))
        accids = [note] + list(note.iter(f"{MEI}accid"))
        accid = next((a.get("accid.ges") or a.get("accid") for a in accids if a.get("accid.ges") or a.get("accid")), "n")
        notes[note.get(XML_ID)] = {
            "step": note.get("pname").upper(),
            "octave": octave,
            "pitch": (octave + 1) * 12 + SEMITONES[note.get("pname")] + ALTER[accid],
        }

    drawn = set(re.findall(r'<g id="([^"]+)" class="note"', svg))
    page_notes = {id: spelling for id, spelling in notes.items() if id in drawn}

    (OUT / f"{piece}.page1.svg").write_text(svg)
    (OUT / f"{piece}.page1.notes.json").write_text(json.dumps(page_notes, indent=0, sort_keys=True) + "\n")
    print(f"{piece}: {len(svg)} bytes, {len(page_notes)} notes on page 1")
