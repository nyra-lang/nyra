#!/usr/bin/env python3
"""Restore English source HTML from Arabic doc pages using doc-i18n-blocks.BLOCKS."""
from __future__ import annotations

import re
import sys
from importlib.machinery import SourceFileLoader
from pathlib import Path

WEBDOCS = Path(__file__).resolve().parent.parent
SKIP_GLOB = ("beginner-*.html",)

TRANSLATABLE = re.compile(
    r"<(h1|h2|h3|h4|p|li|th|td|summary|a)\b([^>]*)>(.*?)</\1>",
    re.IGNORECASE | re.DOTALL,
)
SKIP_CLASS = re.compile(
    r"class=\"[^\"]*(?:code-tab|lesson-nav-hub|lesson-nav-prev|lesson-nav-next)[^\"]*\"",
    re.IGNORECASE,
)
ALLOW_CLASS = re.compile(
    r"class=\"[^\"]*(?:doc-ex-prose|example-output-label|lesson-nav-hub|lesson-nav-prev|lesson-nav-next)[^\"]*\"",
    re.IGNORECASE,
)

MASK_PATTERNS = [
  (re.compile(r"<script\b[^>]*>.*?</script>", re.I | re.S), "SCRIPT"),
  (re.compile(r"<pre\b[^>]*>.*?</pre>", re.I | re.S), "PRE"),
  (re.compile(r"<!--\s*BUILTIN_CODE_TABS_START\s*-->.*?<!--\s*BUILTIN_CODE_TABS_END\s*-->", re.I | re.S), "TABS"),
  (re.compile(r'<div\s+class="code-tabs"[^>]*>.*?</div>', re.I | re.S), "CODETABS"),
]


def load_blocks() -> dict[str, str]:
    mod = SourceFileLoader(
        "docs_i18n",
        str(WEBDOCS / "scripts" / "doc-i18n-blocks.py"),
    ).load_module()
    return dict(getattr(mod, "BLOCKS", {}))


def reverse_map(blocks: dict[str, str]) -> list[tuple[str, str]]:
    """AR innerHTML -> EN innerHTML, longest AR first."""
    pairs = [(ar, en) for en, ar in blocks.items() if en and ar and en != ar]
    pairs.sort(key=lambda x: len(x[0]), reverse=True)
    return pairs


def mask_protected(html: str) -> tuple[str, list[tuple[int, int, str]]]:
    spans: list[tuple[int, int, str]] = []

    def repl(m: re.Match[str], tag: str) -> str:
        spans.append((m.start(), m.end(), tag))
        return f"\x00{tag}:{len(spans)-1}\x00"

    for pat, tag in MASK_PATTERNS:
        html = pat.sub(lambda m, t=tag: repl(m, t), html)
    return html, spans


def unmask(html: str, spans: list[tuple[int, int, str]], original: str) -> str:
    for i, (start, end, _tag) in enumerate(spans):
        token = f"\x00{_tag}:{i}\x00"
        html = html.replace(token, original[start:end], 1)
    return html


def is_translatable_tag(tag: str, attrs: str) -> bool:
    if tag in ("h1", "h2", "h3", "h4", "p", "li", "th", "td", "summary"):
        return True
    if tag == "a" and ALLOW_CLASS.search(attrs):
        return True
    if ALLOW_CLASS.search(attrs):
        return True
    return False


def replace_in_main(html: str, ar_to_en: list[tuple[str, str]], intro_cut: bool) -> str:
    main_m = re.search(r"(<main\b[^>]*>)(.*?)(</main>)", html, re.I | re.S)
    if not main_m:
        return html
    open_tag, main_body, close_tag = main_m.group(1), main_m.group(2), main_m.group(3)

    if intro_cut:
        cut = main_body.find("<!-- BUILTIN_CODE_TABS_START -->")
        if cut >= 0:
            head, tail = main_body[:cut], main_body[cut:]
            head = replace_translatable(head, ar_to_en)
            main_body = head + tail
        else:
            main_body = replace_translatable(main_body, ar_to_en)
    else:
        main_body = replace_translatable(main_body, ar_to_en)

    return html[: main_m.start()] + open_tag + main_body + close_tag + html[main_m.end() :]


def replace_translatable(fragment: str, ar_to_en: list[tuple[str, str]]) -> str:
    masked, spans = mask_protected(fragment)
    orig_masked = masked

    def sub_el(m: re.Match[str]) -> str:
        tag, attrs, inner = m.group(1), m.group(2), m.group(3)
        if not is_translatable_tag(tag, attrs):
            return m.group(0)
        if SKIP_CLASS.search(attrs) and tag == "a" and not ALLOW_CLASS.search(attrs):
            return m.group(0)
        new_inner = inner
        for ar, en in ar_to_en:
            if ar in new_inner:
                new_inner = new_inner.replace(ar, en)
        if new_inner == inner:
            return m.group(0)
        return f"<{tag}{attrs}>{new_inner}</{tag}>"

    masked = TRANSLATABLE.sub(sub_el, masked)
    return unmask(masked, spans, orig_masked)


def set_lang_ltr(html: str) -> str:
    html = re.sub(
        r"<html\b([^>]*)\blang=\"ar\"([^>]*)>",
        lambda m: f'<html{m.group(1)}lang="en"{m.group(2)}>',
        html,
        count=1,
        flags=re.I,
    )
    html = re.sub(
        r"<html\b([^>]*)\bdir=\"rtl\"([^>]*)>",
        lambda m: f'<html{m.group(1)}dir="ltr"{m.group(2)}>',
        html,
        count=1,
        flags=re.I,
    )
    if 'lang="en"' not in html[:500]:
        html = re.sub(r"<html\b", '<html lang="en" dir="ltr"', html, count=1, flags=re.I)
    return html


def should_process(path: Path) -> bool:
    name = path.name
    if not name.endswith(".html"):
        return False
    if name.startswith("beginner-") and name != "beginner-track.html":
        return False
    if name == "beginner-track.html":
        return False
    return True


def main() -> int:
    blocks = load_blocks()
    ar_to_en = reverse_map(blocks)
    intro_only = {"methods.html", "stdlib.html", "changelog.html"}

    converted: list[str] = []
    for path in sorted(WEBDOCS.glob("*.html")):
        if not should_process(path):
            continue
        html = path.read_text(encoding="utf-8")
        if 'lang="ar"' not in html[:400] and not re.search(r"[\u0600-\u06FF]", html):
            continue
        new_html = replace_in_main(html, ar_to_en, path.name in intro_only)
        new_html = set_lang_ltr(new_html)
        if new_html != html:
            path.write_text(new_html, encoding="utf-8")
            converted.append(path.name)
            print(f"converted {path.name}")

    print(f"done: {len(converted)} file(s), {len(blocks)} block(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
