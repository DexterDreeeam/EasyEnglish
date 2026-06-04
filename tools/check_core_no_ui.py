#!/usr/bin/env python3
"""Architectural guard: src/core/** must NOT include Qt UI headers.

Run from repo root:  python tools/check_core_no_ui.py
Exit code 0 = clean, 1 = violation found.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

FORBIDDEN = re.compile(
    r'^\s*#\s*include\s*[<"](Qt(?:Widgets|Gui|Quick|Qml|QuickWidgets)/?|'
    r'Q(?:Widget|MainWindow|Application|Label|Push|LineEdit|TextBrowser|Layout|Dialog))',
    re.MULTILINE,
)

ROOT = Path(__file__).resolve().parents[1]
CORE = ROOT / "src" / "core"


def main() -> int:
    if not CORE.is_dir():
        print(f"[check_core_no_ui] {CORE} not found", file=sys.stderr)
        return 0  # nothing to check yet

    violations: list[tuple[Path, int, str]] = []
    for path in CORE.rglob("*"):
        if path.suffix.lower() not in {".h", ".hpp", ".cpp", ".cc", ".cxx"}:
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        for match in FORBIDDEN.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            violations.append((path.relative_to(ROOT), line, match.group(0).strip()))

    if violations:
        print("[check_core_no_ui] FAILED: src/core/** must not include Qt UI headers")
        for rel, line, snippet in violations:
            print(f"  {rel}:{line}: {snippet}")
        return 1

    print("[check_core_no_ui] OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
