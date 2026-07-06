#!/usr/bin/env python3
"""Inject Lunr search assets and search button into all webDocs HTML pages."""
import re
from pathlib import Path

WEBDOCS = Path(__file__).resolve().parent.parent

SEARCH_BTN = '''      <div class="toolbar-group" role="group" aria-label="Search">
        <button type="button" class="toolbar-btn search-btn" id="search-open" data-i18n-title="ui.searchOpen" title="Search (Ctrl+K)">
          <span aria-hidden="true">⌕</span>
          <kbd>Ctrl+K</kbd>
        </button>
      </div>
'''

SIDEBAR_SEARCH = '''      <li><a href="#" id="sidebar-search" data-i18n="nav.search">Search</a></li>
'''

def patch_file(path: Path) -> bool:
    text = path.read_text(encoding='utf-8')
    orig = text

    if 'search.css' not in text:
        text = text.replace(
            '<link rel="stylesheet" href="css/style.css">',
            '<link rel="stylesheet" href="css/style.css">\n  <link rel="stylesheet" href="css/search.css">',
        )

    if 'id="search-open"' not in text and 'site-toolbar' in text:
        text = text.replace(
            '<div class="site-toolbar">',
            '<div class="site-toolbar">\n' + SEARCH_BTN,
            1,
        )

    if 'vendor/lunr.min.js' not in text:
        text = text.replace(
            '<script src="js/site.js"></script>',
            '<script src="vendor/lunr.min.js"></script>\n    <script src="js/search.js"></script>\n    <script src="js/site.js"></script>',
        )

    # Sidebar search link under Start section
    if 'nav.search' not in text and '<li><a class="active" href="index.html"' in text or 'nav.overview' in text:
        text = re.sub(
            r'(<li><a[^>]*href="install\.html"[^>]*>.*?</li>\s*)',
            r'\1' + SIDEBAR_SEARCH,
            text,
            count=1,
        )
    elif 'nav.search' not in text:
        text = re.sub(
            r'(<li><a href="install\.html"[^>]*>.*?</li>\s*)',
            r'\1' + SIDEBAR_SEARCH,
            text,
            count=1,
        )

    if text != orig:
        path.write_text(text, encoding='utf-8')
        return True
    return False


def main():
    count = 0
    for html in WEBDOCS.glob('*.html'):
        if patch_file(html):
            print(f'patched {html.name}')
            count += 1
    print(f'done: {count} files')


if __name__ == '__main__':
    main()
