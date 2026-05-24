#!/usr/bin/env python3
"""Count tokens/chars/lines for captured benchmark output files.

Usage:
    python3 bench/count_tokens_fetch.py bench-output/*.txt

Requires:
    pip install tiktoken
"""

from __future__ import annotations

import sys
from pathlib import Path

try:
    import tiktoken
except ImportError:  # pragma: no cover - user-facing script
    print("error: missing dependency: tiktoken", file=sys.stderr)
    print("install with: python3 -m pip install tiktoken", file=sys.stderr)
    raise SystemExit(2)

ENCODING = "cl100k_base"


def main() -> int:
    paths = [Path(arg) for arg in sys.argv[1:]]
    if not paths:
        print("usage: python3 bench/count_tokens_fetch.py bench-output/*.txt", file=sys.stderr)
        return 2

    enc = tiktoken.get_encoding(ENCODING)
    rows = []
    for path in paths:
        if not path.exists() or not path.is_file():
            continue
        text = path.read_text(errors="replace")
        rows.append((path.stem, len(enc.encode(text)), len(text), text.count("\n")))

    if not rows:
        print("error: no readable input files", file=sys.stderr)
        return 1

    name_w = max(len(name) for name, *_ in rows + [("Output", 0, 0, 0)]) + 2
    print(f"Tokenizer: {ENCODING}\n")
    print(f"{'Output'.ljust(name_w)}{'Tokens':>8}{'Chars':>8}{'Lines':>7}")
    print("-" * (name_w + 23))
    for name, toks, chars, lines in rows:
        print(f"{name.ljust(name_w)}{toks:>8}{chars:>8}{lines:>7}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
