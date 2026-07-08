#!/usr/bin/env python3
"""Replace sidebar <nav> in all webDocs HTML with expanded nav from generate-pages."""
import re
from pathlib import Path

WEBDOCS = Path(__file__).resolve().parent.parent
NAV = (WEBDOCS / "scripts" / "generate-pages.py").read_text(encoding="utf-8")
# extract NAV constant
start = NAV.index("NAV = '''") + len("NAV = '''")
end = NAV.index("'''", start)
NAV_HTML = NAV[start:end]

def nav_for(active: str) -> str:
    nav = NAV_HTML
    if active in nav:
        nav = re.sub(
            rf'<a href="{re.escape(active)}"',
            f'<a class="active" href="{active}"',
            nav,
            count=1,
        )
    return nav


def main():
    pattern = re.compile(r"<nav>[\s\S]*?</nav>", re.MULTILINE)
    for html in WEBDOCS.glob("*.html"):
        text = html.read_text(encoding="utf-8")
        active = html.name
        new_nav = nav_for(active).strip()
        if not new_nav.startswith('<nav>'):
            new_nav = '<nav>\n' + new_nav + '\n</nav>'
        new_text, n = pattern.subn(new_nav, text, count=1)
        if n:
            html.write_text(new_text, encoding="utf-8")
            print(f"synced {html.name}")


if __name__ == "__main__":
    main()
