#!/usr/bin/env python3
"""Build doc-i18n-blocks.py from embedded EN/AR pairs."""
from __future__ import annotations

import json
from pathlib import Path

OUT = Path(__file__).resolve().parent / "doc-i18n-blocks.py"
DATA = Path(__file__).resolve().parent / "doc-i18n-pairs.json"


def main() -> None:
    pairs: list[list[str]] = json.loads(DATA.read_text(encoding="utf-8"))
    blocks: dict[str, str] = {}
    for en, ar in pairs:
        if en and ar and en != ar:
            blocks[en] = ar
    lines = [
        '#!/usr/bin/env python3',
        '"""English innerHTML -> Arabic innerHTML for doc pages (not learn-*)."""',
        "from __future__ import annotations",
        "",
        "BLOCKS: dict[str, str] = {",
    ]
    for en in sorted(blocks, key=lambda s: s.lower()):
        ar = blocks[en]
        lines.append(f"    {json.dumps(en, ensure_ascii=False)}: {json.dumps(ar, ensure_ascii=False)},")
    lines.append("}")
    lines.append("")
    OUT.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {OUT} ({len(blocks)} blocks)")


if __name__ == "__main__":
    main()
