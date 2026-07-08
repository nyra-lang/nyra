#!/usr/bin/env python3
"""Restore English as HTML source; rebuild ar-content.json from current Arabic pages."""
from __future__ import annotations

import json
import re
import subprocess
import sys
from html import unescape
from pathlib import Path

WEBDOCS = Path(__file__).resolve().parent.parent
AR_CONTENT = WEBDOCS / "locales" / "ar-content.json"
SCRIPTS = WEBDOCS / "scripts"

TRANSLATE_RE = re.compile(
    r"<(h2|h3|h4|p|li|th|td|summary|a)([^>]*class=\"[^\"]*(?:doc-ex-prose|lesson-nav|example-output-label|builtin-ex-title)[^\"]*\"[^>]*)>"
    r"(.*?)</\1>",
    re.S,
)
# Also plain h2/h3 inside doc-ex, lesson-nav links, lead without i18n
BLOCK_TAGS = re.compile(
    r"<(h2|h3|h4|p|li|th|td|summary)([^>]*)>(.*?)</\1>",
    re.S,
)
LESSON_NAV = re.compile(
    r'<a class="lesson-nav-(prev|next|hub)"[^>]*>(.*?)</a>',
    re.S,
)
ARABIC = re.compile(r"[\u0600-\u06FF]")
SKIP = re.compile(r"<pre|<code|code-tabs|BUILTIN_CODE|NYRA_SNIPPET", re.I)


def normalize_key(s: str) -> str:
    return re.sub(r"\s+", " ", unescape(s)).strip()


def has_arabic(s: str) -> bool:
    return bool(ARABIC.search(s))


def strip_tags(s: str) -> str:
    return re.sub(r"<[^>]+>", "", s)


def extract_main(path: Path) -> str:
    text = path.read_text(encoding="utf-8")
    m = re.search(r"<main class=\"content\">(.*)</main>", text, re.S)
    return m.group(1) if m else ""


def extract_blocks(html: str) -> list[tuple[str, str]]:
    """Return list of (tag_key, innerHTML) for translatable leaves."""
    out: list[tuple[str, str]] = []
    main = html

    for m in LESSON_NAV.finditer(main):
        inner = m.group(2).strip()
        if inner and not SKIP.search(inner):
            out.append(("nav", inner))

    for m in BLOCK_TAGS.finditer(main):
        attrs, inner = m.group(2), m.group(3).strip()
        if not inner or SKIP.search(inner):
            continue
        if m.group(1) in ("h2", "h3", "h4") and m.group(0).count("<") > inner.count("<") + 2:
            continue
        if "data-i18n" in attrs:
            continue
        if m.group(1) == "p" and "doc-ex-prose" not in attrs and "lead" not in attrs:
            if "example-output" in attrs:
                continue
            if not has_arabic(inner):
                continue
        if not has_arabic(inner) and "doc-ex-prose" not in attrs:
            continue
        out.append((m.group(1), inner))

    return out


def pair_by_code(ar_html: str, en_html: str) -> dict[str, str]:
    """Map English innerHTML -> Arabic innerHTML using matching code snippets as anchors."""
    pairs: dict[str, str] = {}

    ar_secs = re.findall(
        r"<section class=\"doc-ex\">(.*?)</section>", ar_html, re.S
    )
    en_secs = re.findall(
        r"<section class=\"doc-ex\">(.*?)</section>", en_html, re.S
    )
    if len(ar_secs) == len(en_secs) and ar_secs:
        for a, e in zip(ar_secs, en_secs):
            for tag in ("h2", "h3", "p"):
                am = re.search(
                    rf"<{tag}[^>]*>(.*?)</{tag}>", a, re.S
                )
                em = re.search(
                    rf"<{tag}[^>]*class=\"doc-ex-prose\"[^>]*>(.*?)</p>", e, re.S
                ) if tag == "p" else re.search(
                    rf"<{tag}[^>]*>(.*?)</{tag}>", e, re.S
                )
                if tag == "p":
                    am = re.search(r'class="doc-ex-prose"[^>]*>(.*?)</p>', a, re.S)
                if am and em:
                    ar_in, en_in = am.group(1).strip(), em.group(1).strip()
                    if has_arabic(ar_in) and not has_arabic(en_in):
                        pairs[normalize_key(en_in)] = ar_in
                    ar_h = re.search(r"<h2[^>]*>(.*?)</h2>", a, re.S)
                    en_h = re.search(r"<h2[^>]*>(.*?)</h2>", e, re.S)
                    if ar_h and en_h:
                        ai, ei = ar_h.group(1).strip(), en_h.group(1).strip()
                        if has_arabic(ai) and not has_arabic(ei):
                            pairs[normalize_key(ei)] = ai
        return pairs

    # Fallback: zip extracted blocks in order
    ar_blocks = extract_blocks(ar_html)
    en_blocks = extract_blocks(en_html)
    n = min(len(ar_blocks), len(en_blocks))
    for i in range(n):
        _, ar_in = ar_blocks[i]
        _, en_in = en_blocks[i]
        if has_arabic(ar_in) and not has_arabic(en_in):
            pairs[normalize_key(en_in)] = ar_in
    return pairs


def fix_html_lang(text: str) -> str:
    text = text.replace('<html lang="ar" dir="rtl"', '<html lang="en" dir="ltr"')
    return text


def apply_en_to_file(path: Path, blocks: dict[str, str]) -> bool:
    """Replace Arabic translatable innerHTML with English keys from blocks (reverse)."""
    reverse = {normalize_key(v): k for k, v in blocks.items()}
    text = path.read_text(encoding="utf-8")
    orig = text
    text = fix_html_lang(text)

    def repl_nav(m: re.Match) -> str:
        inner = m.group(2).strip()
        key = normalize_key(inner)
        if key in reverse:
            inner = reverse[key]
        return m.group(0).replace(m.group(2), inner)

    text = LESSON_NAV.sub(repl_nav, text)

    def repl_block(m: re.Match) -> str:
        tag, attrs, inner = m.group(1), m.group(2), m.group(3)
        if "data-i18n" in attrs or SKIP.search(m.group(0)):
            return m.group(0)
        if tag == "p" and "doc-ex-prose" not in attrs and "lead" not in attrs:
            return m.group(0)
        key = normalize_key(inner)
        if key in reverse:
            return f"<{tag}{attrs}>{reverse[key]}</{tag}>"
        return m.group(0)

    text = BLOCK_TAGS.sub(repl_block, text)
    if text != orig:
        path.write_text(text, encoding="utf-8")
        return True
    return False


def load_en_pages() -> dict:
    en_path = WEBDOCS / "locales" / "en.json"
    return json.loads(en_path.read_text(encoding="utf-8")).get("pages", {})


def apply_en_h1_lead(path: Path, pages: dict) -> None:
    text = path.read_text(encoding="utf-8")
    m = re.search(r'data-page="([^"]+)"', text)
    if not m:
        return
    page = m.group(1)
    meta = pages.get(page, {})
    if not meta:
        return
    if "h1" in meta:
        text = re.sub(
            r'(<h1[^>]*data-i18n="pages\.[^"]+\.h1"[^>]*>)(.*?)(</h1>)',
            rf"\1{meta['h1']}\3",
            text,
            count=1,
            flags=re.S,
        )
    if "lead" in meta:
        text = re.sub(
            r'(<p class="lead"[^>]*data-i18n-html="pages\.[^"]+\.lead"[^>]*>)(.*?)(</p>)',
            rf"\1{meta['lead']}\3",
            text,
            count=1,
            flags=re.S,
        )
    path.write_text(text, encoding="utf-8")


def main() -> int:
    # 1) Snapshot Arabic learn pages before regen
    ar_snap: dict[str, str] = {}
    for p in WEBDOCS.glob("learn-*.html"):
        ar_snap[p.name] = extract_main(p)

    # 2) Regenerate learn track in English
    gen = SCRIPTS / "generate-learn-track.py"
    subprocess.run([sys.executable, str(gen)], check=True, cwd=WEBDOCS.parent)

    all_blocks: dict[str, str] = {}

    # 3) Pair learn pages
    for name, ar_main in ar_snap.items():
        p = WEBDOCS / name
        if not p.exists():
            continue
        en_main = extract_main(p)
        pairs = pair_by_code(ar_main, en_main)
        all_blocks.update(pairs)

    # 4) Pair other doc-ex pages (AR snapshot was current files — read from git stash impossible)
    #    For non-learn: current files still Arabic; we'll convert using accumulated pairs + manual pass
    for p in sorted(WEBDOCS.glob("*.html")):
        if p.name.startswith("learn-") or p.name.startswith("beginner-"):
            continue
        if 'http-equiv="refresh"' in p.read_text(encoding="utf-8")[:400]:
            continue
        main = extract_main(p)
        if not has_arabic(main):
            continue
        # Try git HEAD English main
        try:
            en_git = subprocess.run(
                ["git", "show", f"HEAD:webDocs/{p.name}"],
                capture_output=True,
                text=True,
                cwd=WEBDOCS.parent,
                check=True,
            ).stdout
            en_main = extract_main(Path("/dev/stdin")) if False else ""
            m = re.search(r"<main class=\"content\">(.*)</main>", en_git, re.S)
            if m and has_arabic(main):
                pairs = pair_by_code(main, m.group(1))
                all_blocks.update(pairs)
        except subprocess.CalledProcessError:
            pass

    # 5) Merge into ar-content.json
    data = json.loads(AR_CONTENT.read_text(encoding="utf-8"))
    existing = data.get("blocks", {})
    existing.update(all_blocks)
    data["blocks"] = existing
    AR_CONTENT.write_text(
        json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print(f"ar-content blocks: {len(existing)}")

    # 6) Convert all HTML to English using reverse map
    pages = load_en_pages()
    changed = 0
    for p in sorted(WEBDOCS.glob("*.html")):
        if 'http-equiv="refresh"' in p.read_text(encoding="utf-8")[:400]:
            text = fix_html_lang(p.read_text(encoding="utf-8"))
            p.write_text(text, encoding="utf-8")
            continue
        if apply_en_to_file(p, all_blocks):
            changed += 1
        apply_en_h1_lead(p, pages)
        text = fix_html_lang(p.read_text(encoding="utf-8"))
        p.write_text(text, encoding="utf-8")

    print(f"updated files: {changed}, lang=en on all pages")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
