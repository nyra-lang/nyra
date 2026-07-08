#!/usr/bin/env python3
"""Restore English HTML source; merge ar-content.json blocks from Arabic snapshots."""
from __future__ import annotations

import json
import re
import subprocess
import sys
from html import unescape
from pathlib import Path

WEBDOCS = Path(__file__).resolve().parent.parent
AR_CONTENT = WEBDOCS / "locales" / "ar-content.json"
REPO = WEBDOCS.parent

ARABIC = re.compile(r"[\u0600-\u06FF]")
BLOCK_TAGS = re.compile(r"<(h2|h3|h4|p|li|th|td|summary)([^>]*)>(.*?)</\1>", re.S)
LESSON_NAV = re.compile(r'<a class="lesson-nav-(prev|next|hub)"[^>]*>(.*?)</a>', re.S)
SKIP = re.compile(r"<pre|<code|code-tabs|BUILTIN_CODE|NYRA_SNIPPET", re.I)
INTRO_ONLY = {"methods.html", "stdlib.html"}


def normalize_key(s: str) -> str:
    return re.sub(r"\s+", " ", unescape(s)).strip()


def has_arabic(s: str) -> bool:
    return bool(ARABIC.search(s))


def extract_main(path: Path) -> str:
    text = path.read_text(encoding="utf-8")
    m = re.search(r"<main class=\"content\">(.*)</main>", text, re.S)
    return m.group(1) if m else ""


def extract_blocks(html: str) -> list[str]:
    out: list[str] = []
    for m in LESSON_NAV.finditer(html):
        inner = m.group(2).strip()
        if inner and not SKIP.search(inner):
            out.append(inner)
    for m in BLOCK_TAGS.finditer(html):
        attrs, inner = m.group(2), m.group(3).strip()
        if not inner or SKIP.search(inner) or "data-i18n" in attrs:
            continue
        if m.group(1) == "p" and "doc-ex-prose" not in attrs and "lead" not in attrs:
            if "example-output" in attrs:
                continue
            continue
        out.append(inner)
    return out


def pair_sections(ar_html: str, en_html: str) -> dict[str, str]:
    pairs: dict[str, str] = {}
    ar_secs = re.findall(r"<section class=\"doc-ex\">(.*?)</section>", ar_html, re.S)
    en_secs = re.findall(r"<section class=\"doc-ex\">(.*?)</section>", en_html, re.S)
    if ar_secs and len(ar_secs) == len(en_secs):
        for a, e in zip(ar_secs, en_secs):
            for tag in ("h2", "h3", "p"):
                if tag == "p":
                    am = re.search(r'class="doc-ex-prose"[^>]*>(.*?)</p>', a, re.S)
                    em = re.search(r'class="doc-ex-prose"[^>]*>(.*?)</p>', e, re.S)
                else:
                    am = re.search(rf"<{tag}[^>]*>(.*?)</{tag}>", a, re.S)
                    em = re.search(rf"<{tag}[^>]*>(.*?)</{tag}>", e, re.S)
                if am and em:
                    ar_in, en_in = am.group(1).strip(), em.group(1).strip()
                    if has_arabic(ar_in) and not has_arabic(en_in):
                        pairs[normalize_key(en_in)] = ar_in
    ar_blocks = extract_blocks(ar_html)
    en_blocks = extract_blocks(en_html)
    for ar_in, en_in in zip(ar_blocks, en_blocks):
        if has_arabic(ar_in) and not has_arabic(en_in):
            pairs[normalize_key(en_in)] = ar_in
    return pairs


def fix_html_lang(text: str) -> str:
    text = text.replace('<html lang="ar" dir="rtl"', '<html lang="en" dir="ltr"')
    if 'lang="en"' not in text[:500]:
        text = re.sub(r"<html\b", '<html lang="en" dir="ltr"', text, count=1)
    return text


def load_en_pages() -> dict:
    return json.loads((WEBDOCS / "locales" / "en.json").read_text(encoding="utf-8"))["pages"]


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
    if "metaTitle" in meta:
        text = re.sub(
            r"<title>.*?</title>",
            f"<title>{meta['metaTitle']}</title>",
            text,
            count=1,
            flags=re.S,
        )
    path.write_text(text, encoding="utf-8")


def restore_non_learn_from_git() -> None:
    for p in sorted(WEBDOCS.glob("*.html")):
        name = p.name
        if name.startswith("learn-") or name.startswith("beginner-"):
            continue
        if name in INTRO_ONLY:
            continue
        if name == "language-basics.html":
            continue
        subprocess.run(
            ["git", "show", f"HEAD:webDocs/{name}"],
            capture_output=True,
            text=True,
            cwd=REPO,
            check=True,
        )
        en_html = subprocess.run(
            ["git", "show", f"HEAD:webDocs/{name}"],
            capture_output=True,
            text=True,
            cwd=REPO,
            check=True,
        ).stdout
        p.write_text(en_html, encoding="utf-8")
        print(f"restored {name} from git")


def convert_language_basics() -> None:
    path = WEBDOCS / "language-basics.html"
    pages = load_en_pages()
    meta = pages["language-basics"]
    text = path.read_text(encoding="utf-8")
    text = fix_html_lang(text)
    text = re.sub(
        r'(<h1[^>]*data-i18n="pages\.language-basics\.h1"[^>]*>)(.*?)(</h1>)',
        rf"\1{meta['h1']}\3",
        text,
        count=1,
        flags=re.S,
    )
    text = re.sub(
        r'(<p class="lead"[^>]*data-i18n-html="pages\.language-basics\.lead"[^>]*>)(.*?)(</p>)',
        rf"\1{meta['lead']}\3",
        text,
        count=1,
        flags=re.S,
    )
    text = re.sub(r"<title>.*?</title>", f"<title>{meta['metaTitle']}</title>", text, count=1)
    replacements = [
        ("<h2>let و let mut</h2>", "<h2>let and let mut</h2>"),
        (
            '<p class="doc-ex-prose"><code>let</code> ثابت. لتغيير القيمة استخدم <code>let mut</code>.</p>',
            '<p class="doc-ex-prose"><code>let</code> is immutable. Use <code>let mut</code> to change the value.</p>',
        ),
        (
            '<p class="doc-ex-prose"><code>print</code> يكتب إلى الطرفية مع سطر جديد.</p>',
            '<p class="doc-ex-prose"><code>print</code> writes to the terminal with a newline after each call.</p>',
        ),
        (
            '<p class="doc-ex-prose">فرّع التنفيذ بشرط منطقي.</p>',
            '<p class="doc-ex-prose">Branch execution on a boolean condition.</p>',
        ),
        (
            "<h2>for و while</h2>",
            "<h2>for and while</h2>",
        ),
        (
            '<p class="doc-ex-prose"><code>for</code> على نطاق رقمي؛ <code>while</code> طالما الشرط صحيح.</p>',
            '<p class="doc-ex-prose"><code>for</code> over a numeric range; <code>while</code> while the condition is true.</p>',
        ),
        (
            '<p class="doc-ex-prose">عرّف دالة بمعاملات واستدعِها من <code>main</code>.</p>',
            '<p class="doc-ex-prose">Define a function with parameters and call it from <code>main</code>.</p>',
        ),
    ]
    for old, new in replacements:
        text = text.replace(old, new)
    path.write_text(text, encoding="utf-8")
    print("converted language-basics.html")


def fix_intro_pages() -> None:
    pages = load_en_pages()
    for name in INTRO_ONLY:
        path = WEBDOCS / name
        text = fix_html_lang(path.read_text(encoding="utf-8"))
        path.write_text(text, encoding="utf-8")
        apply_en_h1_lead(path, pages)
        print(f"fixed lang/h1/lead on {name}")


def main() -> int:
    ar_snap: dict[str, str] = {}
    for p in WEBDOCS.glob("learn-*.html"):
        ar_snap[p.name] = extract_main(p)
    lb_snap = extract_main(WEBDOCS / "language-basics.html")
    intro_snaps = {n: extract_main(WEBDOCS / n) for n in INTRO_ONLY}

    gen = WEBDOCS / "scripts" / "generate-learn-track.py"
    subprocess.run([sys.executable, str(gen)], check=True, cwd=REPO)

    all_blocks: dict[str, str] = {}
    for name, ar_main in ar_snap.items():
        en_main = extract_main(WEBDOCS / name)
        all_blocks.update(pair_sections(ar_main, en_main))

    restore_non_learn_from_git()
    convert_language_basics()
    all_blocks.update(pair_sections(lb_snap, extract_main(WEBDOCS / "language-basics.html")))
    fix_intro_pages()

    data = json.loads(AR_CONTENT.read_text(encoding="utf-8"))
    existing = data.get("blocks", {})
    existing.update(all_blocks)
    data["blocks"] = dict(sorted(existing.items(), key=lambda kv: kv[0].lower()))
    AR_CONTENT.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"ar-content blocks: {len(data['blocks'])}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
