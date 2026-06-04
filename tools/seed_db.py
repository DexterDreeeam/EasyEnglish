#!/usr/bin/env python3
"""Seed the mini dictionary fixture used by storage/dictionary unit tests.

Schema is intentionally tiny and stable. Re-running this script must produce
a byte-identical SQLite file when the WORDS list below is unchanged, so test
fixtures are reproducible across machines:

    python tools/seed_db.py [out_path]

Default out_path: tests/fixtures/mini_dict.sqlite (the canonical fixture).
"""
from __future__ import annotations

import json
import os
import sqlite3
import sys
from pathlib import Path

# Small, hand-picked subset. Expand if real iterations need broader coverage,
# but keep the list deterministic — golden tests downstream depend on it.
WORDS: list[tuple[str, str, list[str]]] = [
    ("apple",   "/ˈæp.əl/",     ["a round fruit with red or green skin",
                                 "the tree on which apples grow"]),
    ("apply",   "/əˈplaɪ/",     ["make a formal request",
                                 "put something to use"]),
    ("ample",   "/ˈæm.pəl/",    ["enough or more than enough"]),
    ("banana",  "/bəˈnɑː.nə/",  ["a long curved fruit with a yellow skin"]),
    ("band",    "/bænd/",       ["a small group of musicians",
                                 "a flat narrow piece of material"]),
    ("book",    "/bʊk/",        ["a set of printed pages bound together",
                                 "arrange for a place at a later time"]),
    ("boot",    "/buːt/",       ["a strong shoe covering the foot and ankle",
                                 "start a computer"]),
    ("cat",     "/kæt/",        ["a small domesticated carnivorous mammal"]),
    ("car",     "/kɑːr/",       ["a road vehicle with four wheels"]),
    ("code",    "/koʊd/",       ["a system of words or symbols",
                                 "instructions that a computer follows"]),
    ("dog",     "/dɒɡ/",        ["a domesticated carnivorous mammal"]),
    ("door",    "/dɔːr/",       ["a hinged barrier closing an entrance"]),
    ("easy",    "/ˈiː.zi/",     ["achieved without great effort"]),
    ("east",    "/iːst/",       ["the direction toward the sunrise"]),
    ("english", "/ˈɪŋ.ɡlɪʃ/",   ["relating to England or its people",
                                 "the language of England"]),
    ("fast",    "/fæst/",       ["moving or capable of moving at high speed"]),
    ("file",    "/faɪl/",       ["a collection of data stored on a computer",
                                 "a folder or box for keeping papers"]),
    ("flash",   "/flæʃ/",       ["a sudden brief burst of light"]),
    ("good",    "/ɡʊd/",        ["to be desired or approved of"]),
    ("great",   "/ɡreɪt/",      ["of an extent or amount above the normal"]),
    ("hello",   "/həˈloʊ/",     ["used as a greeting"]),
    ("house",   "/haʊs/",       ["a building for human habitation"]),
    ("ice",     "/aɪs/",        ["frozen water, a brittle transparent solid"]),
    ("idea",    "/aɪˈdɪə/",     ["a thought or suggestion"]),
    ("join",    "/dʒɔɪn/",      ["link or connect"]),
    ("jump",    "/dʒʌmp/",      ["push oneself off a surface into the air"]),
    ("key",     "/kiː/",        ["a small piece of shaped metal for a lock",
                                 "crucially important"]),
    ("kind",    "/kaɪnd/",      ["having a friendly nature",
                                 "a class or category"]),
    ("learn",   "/lɜːrn/",      ["gain knowledge or skill"]),
    ("light",   "/laɪt/",       ["the natural agent that stimulates sight",
                                 "of little weight"]),
    ("map",     "/mæp/",        ["a diagrammatic representation of an area"]),
    ("music",   "/ˈmjuːzɪk/",   ["vocal or instrumental sounds combined"]),
    ("night",   "/naɪt/",       ["the time from sunset to sunrise"]),
    ("note",    "/noʊt/",       ["a brief written record",
                                 "a single tone of definite pitch"]),
    ("open",    "/ˈoʊpən/",     ["allowing access",
                                 "move so as to permit access"]),
    ("orange",  "/ˈɒrɪndʒ/",    ["a round juicy citrus fruit",
                                 "a reddish-yellow color"]),
    ("paper",   "/ˈpeɪpər/",    ["material for writing, printing, or wrapping"]),
    ("phone",   "/foʊn/",       ["a device for talking to people far away"]),
    ("query",   "/ˈkwɪər.i/",   ["a question, especially expressing doubt"]),
    ("quick",   "/kwɪk/",       ["moving fast or doing something fast"]),
    ("read",    "/riːd/",       ["look at and understand written words"]),
    ("right",   "/raɪt/",       ["morally good, justified",
                                 "the opposite side from the left"]),
    ("see",     "/siː/",        ["perceive with the eyes"]),
    ("sun",     "/sʌn/",        ["the star around which the earth orbits"]),
    ("teach",   "/tiːtʃ/",      ["impart knowledge or skill"]),
    ("time",    "/taɪm/",       ["the indefinite continued progress of existence"]),
    ("under",   "/ˈʌndər/",     ["extending or directly below"]),
    ("use",     "/juːz/",       ["take, hold, or deploy as a means"]),
    ("very",    "/ˈvɛri/",      ["used for emphasis"]),
    ("water",   "/ˈwɔːtər/",    ["a colorless transparent liquid"]),
    ("word",    "/wɜːrd/",      ["a single distinct meaningful element of speech"]),
    ("write",   "/raɪt/",       ["mark coherent letters or symbols on a surface"]),
    ("year",    "/jɪər/",       ["the time taken by the earth to orbit the sun"]),
    ("zero",    "/ˈzɪroʊ/",     ["the number 0"]),
]

SCHEMA = """\
PRAGMA user_version = 1;
CREATE TABLE IF NOT EXISTS entries (
    headword    TEXT PRIMARY KEY COLLATE NOCASE,
    phonetic    TEXT NOT NULL DEFAULT '',
    definitions TEXT NOT NULL  -- JSON array of strings
);
CREATE INDEX IF NOT EXISTS idx_entries_headword_nocase
    ON entries(headword COLLATE NOCASE);
"""


def main(argv: list[str]) -> int:
    out = Path(argv[1]) if len(argv) > 1 else Path("tests/fixtures/mini_dict.sqlite")
    out.parent.mkdir(parents=True, exist_ok=True)
    if out.exists():
        out.unlink()

    conn = sqlite3.connect(out)
    try:
        conn.executescript(SCHEMA)
        conn.executemany(
            "INSERT INTO entries(headword, phonetic, definitions) VALUES(?, ?, ?);",
            [(w, p, json.dumps(d, ensure_ascii=False)) for w, p, d in WORDS],
        )
        conn.commit()
    finally:
        conn.close()

    print(f"Wrote {len(WORDS)} entries to {out} ({out.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
