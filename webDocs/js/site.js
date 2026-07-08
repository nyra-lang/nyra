/**
 * Nyra docs — theme (light/dark) + i18n (en/ar), sets html[dir] ltr/rtl.
 * Nav and header chrome are static HTML; this script only updates translated UI.
 */
(function () {
  var STORAGE_LANG = 'nyra-docs-lang';
  var STORAGE_THEME = 'nyra-docs-theme';
  var STORAGE_CONTRIB_BANNER = 'nyra-docs-contrib-banner-dismissed';
  var CONTRIB_GUIDE_URL =
    'https://github.com/nyra-lang/nyra/blob/main/CONTRIBUTING.md';

  /* Apply saved prefs before paint to avoid flash */
  (function applyEarlyPrefs() {
    var theme = localStorage.getItem(STORAGE_THEME);
    var lang = localStorage.getItem(STORAGE_LANG);
    if (theme !== 'light' && theme !== 'dark') theme = 'dark';
    if (lang !== 'ar' && lang !== 'en') lang = 'en';
    document.documentElement.setAttribute('data-theme', theme);
    document.documentElement.lang = lang;
    document.documentElement.dir = lang === 'ar' ? 'rtl' : 'ltr';
    try {
      localStorage.removeItem(STORAGE_CONTRIB_BANNER);
      if (sessionStorage.getItem(STORAGE_CONTRIB_BANNER) !== '1') {
        document.documentElement.classList.add('contrib-banner-pending');
      }
    } catch (e) {
      document.documentElement.classList.add('contrib-banner-pending');
    }
  })();
  var DEFAULT_LANG = 'en';
  var DEFAULT_THEME = 'dark';

  var state = {
    lang: DEFAULT_LANG,
    theme: DEFAULT_THEME,
    strings: {},
    content: {},
  };

  function getNested(obj, path) {
    var parts = path.split('.');
    var cur = obj;
    for (var i = 0; i < parts.length; i++) {
      if (cur == null || typeof cur !== 'object') return undefined;
      cur = cur[parts[i]];
    }
    return cur;
  }

  function t(key) {
    var val = getNested(state.strings, key);
    return typeof val === 'string' ? val : undefined;
  }

  function loadLocale(lang) {
    return fetch('locales/' + lang + '.json')
      .then(function (r) {
        if (!r.ok) throw new Error('locale ' + lang);
        return r.json();
      })
      .then(function (data) {
        state.strings = data;
        state.lang = lang;
      });
  }

  /* Body-prose dictionary: English HTML is source of truth; ar-content.json maps EN → AR. */
  function loadContentLocale(lang) {
    if (lang === 'en') {
      state.content = {};
      return Promise.resolve();
    }
    return fetch('locales/' + lang + '-content.json')
      .then(function (r) {
        if (!r.ok) throw new Error('content locale ' + lang);
        return r.json();
      })
      .then(function (data) {
        state.content = data || {};
      })
      .catch(function () {
        state.content = {};
      });
  }

  function applyTheme(theme) {
    state.theme = theme;
    document.documentElement.setAttribute('data-theme', theme);
    var meta = document.querySelector('meta[name="theme-color"]');
    if (meta) {
      meta.setAttribute('content', theme === 'light' ? '#f0f4f8' : '#06090d');
    }
    var colorScheme = document.querySelector('meta[name="color-scheme"]');
    if (colorScheme) {
      colorScheme.setAttribute('content', theme === 'light' ? 'light dark' : 'dark light');
    }
    updateThemeToggleUi();
    localStorage.setItem(STORAGE_THEME, theme);
  }

  /* ——— Smart body-prose translation ———
     Translates whole text blocks (headings, paragraphs, list items, cells…) using
     a dictionary keyed by the original English innerHTML. Code blocks, inline
     <code> (method/type names), and elements already handled by data-i18n are
     never touched, so identifiers like print() or i32 stay verbatim. */
  var TRANSLATE_BLOCKS =
    'h1,h2,h3,h4,h5,h6,p,li,figcaption,blockquote,th,td,dt,dd,summary,' +
    '.section-desc,.lesson-meta,.example-output-label,.builtin-ex-title,' +
    '.doc-ex-prose,.bento-kicker,.pillar-label,.callout>strong,.lesson-nav-hub,' +
    '.lesson-nav-prev,.lesson-nav-next,.code-tab';
  var NO_TRANSLATE_ANCESTORS =
    'pre,code,.file-tree,script,style,svg,.no-translate,[data-no-translate]';
  var NESTED_BLOCK = 'p,ul,ol,li,div,table,pre,section,h1,h2,h3,h4,h5,h6,blockquote,dl';

  function normalizeKey(s) {
    return s.replace(/\s+/g, ' ').trim();
  }

  function translateBlocks(lang) {
    var root = document.querySelector('main') || document.body;
    if (!root) return;
    var blocks = state.content && state.content.blocks ? state.content.blocks : null;
    var els = root.querySelectorAll(TRANSLATE_BLOCKS);

    els.forEach(function (el) {
      if (el.hasAttribute('data-i18n') || el.hasAttribute('data-i18n-html')) return;
      if (el.closest(NO_TRANSLATE_ANCESTORS)) return;
      // Skip wrappers that contain their own translatable blocks; translate leaves.
      if (el.querySelector(NESTED_BLOCK)) return;

      if (el.dataset.nyraOrig === undefined) {
        el.dataset.nyraOrig = el.innerHTML;
      }
      var original = el.dataset.nyraOrig;

      if (lang === 'ar' && blocks) {
        var val = blocks[normalizeKey(original)];
        if (typeof val === 'string') {
          if (el.innerHTML !== val) el.innerHTML = val;
          el.dataset.nyraTr = '1';
          return;
        }
      }
      // No translation (or EN): restore the authored English once.
      if (el.dataset.nyraTr === '1') {
        el.innerHTML = original;
        delete el.dataset.nyraTr;
      }
    });
  }

  function applyLang(lang) {
    var isAr = lang === 'ar';
    document.documentElement.lang = lang;
    document.documentElement.dir = isAr ? 'rtl' : 'ltr';

    document.querySelectorAll('[data-i18n]').forEach(function (el) {
      var key = el.getAttribute('data-i18n');
      var text = t(key);
      if (text !== undefined) el.textContent = text;
    });

    document.querySelectorAll('[data-i18n-html]').forEach(function (el) {
      var key = el.getAttribute('data-i18n-html');
      var html = t(key);
      if (html !== undefined) el.innerHTML = html;
    });

    document.querySelectorAll('[data-i18n-placeholder]').forEach(function (el) {
      var key = el.getAttribute('data-i18n-placeholder');
      var text = t(key);
      if (text !== undefined) el.setAttribute('placeholder', text);
    });

    document.querySelectorAll('[data-i18n-title]').forEach(function (el) {
      var key = el.getAttribute('data-i18n-title');
      var text = t(key);
      if (text !== undefined) el.setAttribute('title', text);
    });

    document.querySelectorAll('[data-i18n-aria-label]').forEach(function (el) {
      var key = el.getAttribute('data-i18n-aria-label');
      var text = t(key);
      if (text !== undefined) el.setAttribute('aria-label', text);
    });

    var page = document.body.getAttribute('data-page');
    if (page) {
      var titleKey = 'pages.' + page + '.metaTitle';
      var title = t(titleKey);
      if (title) document.title = title;
    }

    translateBlocks(lang);

    updateLangToggleUi();
    localStorage.setItem(STORAGE_LANG, lang);
    if (window.NyraSearch && window.NyraSearch.applyI18n) {
      window.NyraSearch.applyI18n();
    }
  }

  function isContribBannerDismissed() {
    try {
      return sessionStorage.getItem(STORAGE_CONTRIB_BANNER) === '1';
    } catch (e) {
      return false;
    }
  }

  function setContribBannerDismissed() {
    try {
      sessionStorage.setItem(STORAGE_CONTRIB_BANNER, '1');
      // Clear old permanent dismiss key if present from earlier builds.
      localStorage.removeItem(STORAGE_CONTRIB_BANNER);
    } catch (e) {
      /* ignore quota / private mode */
    }
  }

  function initContribBanner() {
    document.documentElement.classList.remove('contrib-banner-pending');
    if (isContribBannerDismissed()) {
      document.body.classList.remove('has-contrib-banner');
      return;
    }
    if (document.getElementById('contrib-banner')) {
      document.body.classList.add('has-contrib-banner');
      return;
    }

    var banner = document.createElement('aside');
    banner.id = 'contrib-banner';
    banner.className = 'contrib-banner';
    banner.setAttribute('role', 'region');
    banner.setAttribute('aria-label', 'Contributors');

    var inner = document.createElement('div');
    inner.className = 'contrib-banner-inner';

    var text = document.createElement('span');
    text.className = 'contrib-banner-text';
    text.setAttribute('data-i18n', 'common.contribBannerText');
    text.textContent = 'We’re looking for contributors — help build Nyra.';

    var cta = document.createElement('a');
    cta.className = 'contrib-banner-cta';
    cta.href = CONTRIB_GUIDE_URL;
    cta.target = '_blank';
    cta.rel = 'noopener noreferrer';
    cta.setAttribute('data-i18n', 'common.contribBannerCta');
    cta.textContent = 'Contribute';

    var dismiss = document.createElement('button');
    dismiss.type = 'button';
    dismiss.className = 'contrib-banner-dismiss';
    dismiss.setAttribute('data-i18n-aria-label', 'common.contribBannerDismiss');
    dismiss.setAttribute('aria-label', 'Dismiss contributors banner');
    dismiss.textContent = '×';
    dismiss.addEventListener('click', function () {
      setContribBannerDismissed();
      banner.hidden = true;
      document.body.classList.remove('has-contrib-banner');
      document.documentElement.classList.remove('contrib-banner-pending');
    });

    inner.appendChild(text);
    inner.appendChild(cta);
    banner.appendChild(inner);
    banner.appendChild(dismiss);
    document.body.insertBefore(banner, document.body.firstChild);
    document.body.classList.add('has-contrib-banner');
  }

  function el(tag, className, attrs) {
    var node = document.createElement(tag);
    if (className) node.className = className;
    if (attrs) {
      Object.keys(attrs).forEach(function (key) {
        if (key === 'text') node.textContent = attrs[key];
        else if (key === 'html') node.innerHTML = attrs[key];
        else node.setAttribute(key, attrs[key]);
      });
    }
    return node;
  }

  function footerLink(href, i18nKey, fallback, external) {
    var a = el('a', null, {
      href: href,
      'data-i18n': i18nKey,
      text: fallback,
    });
    if (external) {
      a.target = '_blank';
      a.rel = 'noopener noreferrer';
    }
    return a;
  }

  function initDocsFooter() {
    if (document.getElementById('docs-footer')) return;

    var layout = document.querySelector('.layout');
    if (!layout || !layout.parentNode) return;

    var footer = el('footer', 'docs-footer', {
      id: 'docs-footer',
      role: 'contentinfo',
      'data-i18n-aria-label': 'common.footerAria',
      'aria-label': 'Site footer',
    });

    var inner = el('div', 'docs-footer-inner');
    var grid = el('div', 'docs-footer-grid');

    var brand = el('div', 'docs-footer-brand docs-footer-col');
    var logo = el('a', 'docs-footer-logo', { href: 'index.html' });
    var logoImg = el('img', null, {
      src: './assets/Nyrabgremoved.png',
      alt: 'Nyra',
      width: '28',
      height: '28',
    });
    var logoText = el('span', null, {
      'data-i18n': 'common.footerBrand',
      text: 'Nyra',
    });
    logo.appendChild(logoImg);
    logo.appendChild(logoText);
    var tagline = el('p', 'docs-footer-tagline', {
      'data-i18n': 'common.footerTagline',
      text: 'Fast · Safe · Minimal — systems language with LLVM under the hood.',
    });
    brand.appendChild(logo);
    brand.appendChild(tagline);

    var explore = el('div', 'docs-footer-col');
    explore.appendChild(
      el('h3', null, { 'data-i18n': 'common.footerExplore', text: 'Explore' })
    );
    var exploreList = el('ul');
    [
      ['index.html', 'common.footerHome', 'Home'],
      ['install.html', 'common.footerInstall', 'Install'],
      ['learning-path.html', 'common.footerLearning', 'Learning path'],
      ['stdlib.html', 'common.footerStdlib', 'Standard library'],
      ['roadmap.html', 'common.footerRoadmap', 'Roadmap'],
      ['sitemap.html', 'common.footerSitemap', 'Sitemap'],
    ].forEach(function (item) {
      var li = el('li');
      li.appendChild(footerLink(item[0], item[1], item[2], false));
      exploreList.appendChild(li);
    });
    explore.appendChild(exploreList);

    var community = el('div', 'docs-footer-col');
    community.appendChild(
      el('h3', null, { 'data-i18n': 'common.footerCommunity', text: 'Community' })
    );
    var communityList = el('ul');
    [
      [
        'https://github.com/nyra-lang/nyra',
        'common.footerGithub',
        'GitHub',
        true,
      ],
      [CONTRIB_GUIDE_URL, 'common.footerContribute', 'Contribute', true],
      ['changelog.html', 'common.footerChangelog', 'Changelog', false],
      ['ai-skill.html', 'nav.aiSkill', 'AI skill file', false],
    ].forEach(function (item) {
      var li = el('li');
      li.appendChild(footerLink(item[0], item[1], item[2], item[3]));
      communityList.appendChild(li);
    });
    community.appendChild(communityList);

    grid.appendChild(brand);
    grid.appendChild(explore);
    grid.appendChild(community);

    var meta = el('div', 'docs-footer-meta');
    meta.appendChild(
      el('span', null, {
        'data-i18n': 'common.footerLicense',
        text: 'Nyra · Proprietary · All Rights Reserved ·',
      })
    );
    meta.appendChild(
      footerLink(
        'https://github.com/nyra-lang/nyra',
        'common.footerGithub',
        'GitHub',
        true
      )
    );

    inner.appendChild(grid);
    inner.appendChild(meta);
    footer.appendChild(inner);

    // Full-bleed under sidebar + content: sit as a sibling after .layout, not inside <main>.
    layout.parentNode.insertBefore(footer, layout.nextSibling);
    document.body.classList.add('has-docs-footer');
  }

  function bindMobileNav() {
    var navCheck = document.getElementById('nav-check');
    if (!navCheck || navCheck.dataset.bound) return;
    navCheck.dataset.bound = '1';

    var toggle = document.querySelector('.nav-toggle');

    function updateNavAria() {
      if (!toggle) return;
      var open = navCheck.checked;
      var key = open ? 'ui.menuClose' : 'ui.menuOpen';
      var label = t(key);
      if (label) toggle.setAttribute('aria-label', label);
      toggle.setAttribute('aria-expanded', open ? 'true' : 'false');
    }

    navCheck.addEventListener('change', updateNavAria);
    updateNavAria();

    function closeMobileNav() {
      if (!window.matchMedia('(max-width: 768px)').matches) return;
      navCheck.checked = false;
      updateNavAria();
    }

    document.querySelectorAll('.sidebar a[href]').forEach(function (link) {
      link.addEventListener('click', function (e) {
        if (link.getAttribute('href') === '#') return;
        closeMobileNav();
      });
    });
  }

  function bindToolbar() {
    var themeBtn = document.getElementById('theme-toggle');
    if (themeBtn && !themeBtn.dataset.bound) {
      themeBtn.dataset.bound = '1';
      themeBtn.addEventListener('click', function () {
        applyTheme(state.theme === 'dark' ? 'light' : 'dark');
      });
    }
    var langEn = document.getElementById('lang-en');
    if (langEn && !langEn.dataset.bound) {
      langEn.dataset.bound = '1';
      langEn.addEventListener('click', function () {
        switchLang('en');
      });
    }
    var langAr = document.getElementById('lang-ar');
    if (langAr && !langAr.dataset.bound) {
      langAr.dataset.bound = '1';
      langAr.addEventListener('click', function () {
        switchLang('ar');
      });
    }
  }

  function updateThemeToggleUi() {
    var btn = document.getElementById('theme-toggle');
    if (!btn) return;
    btn.setAttribute('aria-pressed', state.theme === 'dark' ? 'true' : 'false');
  }

  function updateLangToggleUi() {
    document.querySelectorAll('.lang-btn').forEach(function (btn) {
      var active = btn.getAttribute('data-lang') === state.lang;
      btn.classList.toggle('active', active);
      btn.setAttribute('aria-pressed', active ? 'true' : 'false');
    });
  }

  function switchLang(lang) {
    if (lang === state.lang) return;
    Promise.all([loadLocale(lang), loadContentLocale(lang)])
      .then(function () {
        applyLang(lang);
        ensureCodeTabs();
        highlightAllCodeBlocks();
        initCodeTabs();
      })
      .catch(function () {
        console.warn('Failed to load locale:', lang);
      });
  }

  function isNyraSnippet(text) {
    var t = text.trim();
    if (!t) return false;

    if (/^(nyra |cargo |rustc |wasmtime |clang |llvm-|\$ |#>|# )/m.test(t)) return false;
    if (/^(curl |mkdir |cd |export |source |xattr |python3? |bash |apt |brew |xcode-select)/m.test(t)) {
      return false;
    }
    if (/^~?\//m.test(t) && t.indexOf('fn ') === -1 && t.indexOf('let ') === -1) return false;
    if (/^(error|warning|help):/m.test(t)) return false;
    if (/^\s*define\s+/.test(t) || /^\s*%/.test(t)) return false;
    if (/^\s*\{[\s\S]*"[\w]+"\s*:/.test(t)) return false;

    if (/\bfn\s+/.test(t)) return true;
    if (/\blet\s+(mut\s+)?\w+/.test(t)) return true;
    if (/\bstruct\s+\w+/.test(t)) return true;
    if (/\benum\s+\w+/.test(t)) return true;
    if (/\bimport\s+"/.test(t)) return true;
    if (/\bextern\s+fn\b/.test(t)) return true;
    if (/\bimpl\s+/.test(t)) return true;
    if (/\bmatch\s+/.test(t)) return true;
    if (/\bfor\s+\w+\s+in\b/.test(t)) return true;
    if (/\bwhile\s+/.test(t)) return true;
    if (/\bconst\s+\w+/.test(t)) return true;
    return false;
  }

  function makeCodePanel(text) {
    var panel = document.createElement('div');
    panel.className = 'code-panel';
    var pre = document.createElement('pre');
    var code = document.createElement('code');
    code.textContent = text;
    pre.appendChild(code);
    panel.appendChild(pre);
    return panel;
  }

  function wrapNyraCodeExamples() {
    var transform =
      (window.NyraTypedTransform && window.NyraTypedTransform.transformSource) ||
      function (s) {
        return s;
      };

    var pres = document.querySelectorAll('main pre, .content pre');
    pres.forEach(function (pre) {
      if (pre.closest('.code-tabs')) return;
      if (pre.closest('[data-code-tabs]')) return;
      if (pre.parentElement && pre.parentElement.closest('.code-tabs')) return;
      if (pre.closest('header, footer, aside, .sidebar, .site-header')) return;
      if (pre.dataset.nyraTabsWrapped === '1') return;

      var code = pre.querySelector('code');
      if (!code) return;

      var plain = code.textContent;
      if (!isNyraSnippet(plain)) return;

      var typed = transform(plain);

      var root = document.createElement('div');
      root.className = 'code-tabs';
      root.setAttribute('data-code-tabs', '');

      var bar = document.createElement('div');
      bar.className = 'code-tabs-bar';
      bar.setAttribute('role', 'tablist');

      var easyBtn = document.createElement('button');
      easyBtn.type = 'button';
      easyBtn.className = 'code-tab active';
      easyBtn.setAttribute('role', 'tab');
      easyBtn.setAttribute('data-tab', 'easy');
      easyBtn.setAttribute('aria-selected', 'true');
      easyBtn.textContent = 'Without types';

      var typedBtn = document.createElement('button');
      typedBtn.type = 'button';
      typedBtn.className = 'code-tab';
      typedBtn.setAttribute('role', 'tab');
      typedBtn.setAttribute('data-tab', 'typed');
      typedBtn.setAttribute('aria-selected', 'false');
      typedBtn.textContent = 'With types';

      bar.appendChild(easyBtn);
      bar.appendChild(typedBtn);

      var easyPanel = makeCodePanel(plain);
      easyPanel.setAttribute('data-panel', 'easy');
      easyPanel.setAttribute('role', 'tabpanel');
      easyPanel.classList.add('active');

      var typedPanel = makeCodePanel(typed);
      typedPanel.setAttribute('data-panel', 'typed');
      typedPanel.setAttribute('role', 'tabpanel');
      typedPanel.setAttribute('hidden', '');

      root.appendChild(bar);
      root.appendChild(easyPanel);
      root.appendChild(typedPanel);

      pre.dataset.nyraTabsWrapped = '1';
      pre.parentNode.replaceChild(root, pre);
    });
  }

  function ensureCodeTabs() {
    if (document.body.dataset.nyraCodeWrapped === '1') return;
    wrapNyraCodeExamples();
    document.body.dataset.nyraCodeWrapped = '1';
  }

  function highlightAllCodeBlocks() {
    // Tokenize only visible code blocks; keep existing file labels intact.
    var codeEls = document.querySelectorAll('pre code');
    if (!codeEls || codeEls.length === 0) return;

    var keywords = new Set([
      'fn',
      'let',
      'mut',
      'const',
      'struct',
      'enum',
      'impl',
      'trait',
      'for',
      'in',
      'while',
      'if',
      'else',
      'match',
      'return',
      'spawn',
      'defer',
      'async',
      'await',
      'extern',
      'export',
      'import',
      'module',
      'use',
      'link',
      'test',
      'unsafe',
      'no_std',
      'pub',
      'private',
      'move',
      'save',
      'true',
      'false',
    ]);

    var types = new Set([
      'i32',
      'i64',
      'u32',
      'u64',
      'f32',
      'f64',
      'bool',
      'string',
      'void',
      'ptr',
      'usize',
      'isize',
      'char',
    ]);

    var numberRe = /^((0x[0-9a-fA-F_]+)|(\d[\d_]*))(\.\d[\d_]*)?$/;
    var pascalTypeRe = /^[A-Z][A-Za-z0-9]*$/; // types/struct/enum names in this docs

    // Order matters: comments and strings first, then numbers/idents, then operators.
    var tokenRe =
      /\/\*[\s\S]*?\*\/|\/\/[^\n]*|"([^"\\]|\\.)*"|'([^'\\]|\\.)*'|-->|=>|->|::|\.\.|\b(?:0x[0-9A-Fa-f_]+|\d[\d_]*)\b|\b[A-Za-z_][A-Za-z0-9_]*\b|[()[\]{}.,;:+\-*/%=&|!<>?]+|\s+/g;

    function classifyToken(token, text, startIndex) {
      if (/^\s+$/.test(token)) return null;

      if (token.indexOf('//') === 0 || token.indexOf('/*') === 0) return 'tok-comment';

      if (
        (token.charAt(0) === '"' && token.charAt(token.length - 1) === '"') ||
        (token.charAt(0) === "'" && token.charAt(token.length - 1) === "'")
      ) {
        return 'tok-string';
      }

      if (token === 'true' || token === 'false') return 'tok-boolean';
      if (numberRe.test(token)) return 'tok-number';

      // Diagnostics output: `error:` / `warning:` / `help:`
      if (token === 'error' || token === 'warning' || token === 'help') {
        if (text.charAt(startIndex + token.length) === ':') return 'tok-error';
      }

      if (keywords.has(token)) return 'tok-keyword';

      if (types.has(token) || pascalTypeRe.test(token)) return 'tok-type';

      // Function calls: identifier followed by '(' after optional whitespace.
      if (/^[A-Za-z_][A-Za-z0-9_]*$/.test(token)) {
        var i = startIndex + token.length;
        while (i < text.length && /\s/.test(text.charAt(i))) i++;
        if (text.charAt(i) === '(') return 'tok-fn';
      }

      // Operators/punctuation
      if (/^(\-\->|=>|->|::|\.\.)$/.test(token)) return 'tok-operator';
      if (/^[()[\]{}.,;:+\-*/%=&|!<>?]+$/.test(token)) return 'tok-operator';

      return null;
    }

    function isInsideFileLabel(textNode) {
      var parent = textNode.parentElement;
      return !!(parent && parent.classList && parent.classList.contains('file-label'));
    }

    codeEls.forEach(function (codeEl) {
      if (codeEl.dataset.nyraSyntaxHighlighted === '1') return;
      codeEl.dataset.nyraSyntaxHighlighted = '1';

      // Collect first; replacing nodes while walking can confuse the traversal.
      var walker = document.createTreeWalker(codeEl, NodeFilter.SHOW_TEXT, null);
      var textNodes = [];
      while (walker.nextNode()) {
        var node = walker.currentNode;
        if (!node.nodeValue) continue;
        if (node.nodeValue.length === 0) continue;
        if (isInsideFileLabel(node)) continue;
        textNodes.push(node);
      }

      textNodes.forEach(function (textNode) {
        var text = textNode.nodeValue;
        if (!text || text.length === 0) return;

        var frag = document.createDocumentFragment();
        var lastIndex = 0;
        tokenRe.lastIndex = 0;
        var m;
        while ((m = tokenRe.exec(text)) !== null) {
          var start = m.index;
          if (start > lastIndex) {
            frag.appendChild(document.createTextNode(text.slice(lastIndex, start)));
          }

          var token = m[0];
          var cls = classifyToken(token, text, start);
          if (cls) {
            var span = document.createElement('span');
            span.className = cls;
            span.textContent = token;
            frag.appendChild(span);
          } else {
            frag.appendChild(document.createTextNode(token));
          }

          lastIndex = tokenRe.lastIndex;
        }

        if (lastIndex < text.length) {
          frag.appendChild(document.createTextNode(text.slice(lastIndex)));
        }

        textNode.parentNode.replaceChild(frag, textNode);
      });
    });
  }

  /* ——— Hero: animated Nyra code typing ———
     Renders a small editor window that types Nyra snippets with live syntax
     highlighting and a blinking caret, then cycles to the next snippet. */
  function initHeroTyping() {
    var out = document.getElementById('hero-typed');
    if (!out || out.dataset.bound === '1') return;
    out.dataset.bound = '1';

    var SNIPPETS = [
      'fn main() -> void {\n' +
        '    let name = "Nyra"\n' +
        '    let mut count = 0\n' +
        '    for i in 0..3 {\n' +
        '        count = count + i\n' +
        '        print("hello, " + name)\n' +
        '    }\n' +
        '}',
      'struct Panther {\n' +
        '    name: string\n' +
        '    speed: i32\n' +
        '}\n\n' +
        'fn run(p: Panther) -> void {\n' +
        '    print(p.name)\n' +
        '}',
      'async fn fetch(url: string) -> i32 {\n' +
        '    let body = await get(url)\n' +
        '    return body.len()\n' +
        '}',
    ];

    var KEYWORDS = {
      fn: 1, let: 1, mut: 1, const: 1, struct: 1, enum: 1, impl: 1, trait: 1,
      for: 1, in: 1, while: 1, if: 1, else: 1, match: 1, return: 1, spawn: 1,
      defer: 1, async: 1, await: 1, extern: 1, export: 1, import: 1, module: 1,
      use: 1, pub: 1, move: 1, true: 1, false: 1,
    };
    var TYPES = {
      i32: 1, i64: 1, u32: 1, u64: 1, f32: 1, f64: 1, bool: 1, string: 1,
      void: 1, ptr: 1, usize: 1, isize: 1, char: 1,
    };

    var tokenRe =
      /\/\/[^\n]*|"([^"\\]|\\.)*"|->|::|\.\.|\b(?:0x[0-9A-Fa-f_]+|\d[\d_]*)\b|\b[A-Za-z_][A-Za-z0-9_]*\b|[()[\]{}.,;:+\-*/%=&|!<>?]+|\s+|[^\s]/g;

    function tokenize(src) {
      var tokens = [];
      var m;
      tokenRe.lastIndex = 0;
      while ((m = tokenRe.exec(src)) !== null) {
        var tok = m[0];
        var cls = '';
        if (/^\s+$/.test(tok)) cls = '';
        else if (tok.indexOf('//') === 0) cls = 'tok-comment';
        else if (tok.charAt(0) === '"') cls = 'tok-string';
        else if (tok === 'true' || tok === 'false') cls = 'tok-boolean';
        else if (/^(?:0x[0-9A-Fa-f_]+|\d[\d_]*)$/.test(tok)) cls = 'tok-number';
        else if (KEYWORDS[tok]) cls = 'tok-keyword';
        else if (TYPES[tok] || /^[A-Z][A-Za-z0-9]*$/.test(tok)) cls = 'tok-type';
        else if (/^[A-Za-z_][A-Za-z0-9_]*$/.test(tok)) {
          var rest = src.slice(m.index + tok.length);
          if (/^\s*\(/.test(rest)) cls = 'tok-fn';
        } else if (/^(->|::|\.\.)$/.test(tok) || /^[()[\]{}.,;:+\-*/%=&|!<>?]+$/.test(tok)) {
          cls = 'tok-operator';
        }
        tokens.push({ text: tok, cls: cls });
      }
      return tokens;
    }

    function escapeHtml(s) {
      return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    }

    function renderUpTo(tokens, chars) {
      var html = '';
      var used = 0;
      for (var i = 0; i < tokens.length && used < chars; i++) {
        var t = tokens[i];
        var take = Math.min(t.text.length, chars - used);
        var piece = escapeHtml(t.text.slice(0, take));
        html += t.cls ? '<span class="' + t.cls + '">' + piece + '</span>' : piece;
        used += take;
      }
      return html;
    }

    var reduced =
      window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    if (reduced) {
      var toks = tokenize(SNIPPETS[0]);
      out.innerHTML = renderUpTo(toks, SNIPPETS[0].length);
      return;
    }

    var snippetIndex = 0;
    var tokens = tokenize(SNIPPETS[0]);
    var full = SNIPPETS[0];
    var pos = 0;
    var typing = true;

    function tick() {
      out.innerHTML = renderUpTo(tokens, pos);
      var delay;
      if (typing) {
        if (pos < full.length) {
          pos++;
          var ch = full.charAt(pos - 1);
          delay = ch === '\n' ? 90 : 18 + Math.random() * 40;
        } else {
          typing = false;
          delay = 2200;
        }
      } else {
        if (pos > 0) {
          pos -= 3;
          if (pos < 0) pos = 0;
          delay = 12;
        } else {
          typing = true;
          snippetIndex = (snippetIndex + 1) % SNIPPETS.length;
          full = SNIPPETS[snippetIndex];
          tokens = tokenize(full);
          delay = 350;
        }
      }
      window.setTimeout(tick, delay);
    }
    tick();
  }

  function init() {
    var lang = localStorage.getItem(STORAGE_LANG) || DEFAULT_LANG;
    var theme = localStorage.getItem(STORAGE_THEME) || DEFAULT_THEME;
    if (lang !== 'en' && lang !== 'ar') lang = DEFAULT_LANG;
    if (theme !== 'light' && theme !== 'dark') theme = DEFAULT_THEME;

    bindToolbar();
    bindMobileNav();
    initContribBanner();
    initDocsFooter();
    applyTheme(theme);
    ensureCodeTabs();
    highlightAllCodeBlocks();
    initCodeTabs();
    initHeroTyping();

    Promise.all([loadLocale(lang), loadContentLocale(lang)])
      .then(function () {
        applyLang(lang);
        bindMobileNav();
        ensureCodeTabs();
        highlightAllCodeBlocks();
        initCodeTabs();
      })
      .catch(function () {
        state.lang = DEFAULT_LANG;
        applyLang(DEFAULT_LANG);
        bindMobileNav();
        ensureCodeTabs();
        highlightAllCodeBlocks();
        initCodeTabs();
      });
  }

  function initCodeTabs() {
    document.querySelectorAll('[data-code-tabs]').forEach(function (root) {
      if (root.dataset.tabsBound === '1') return;
      root.dataset.tabsBound = '1';
      var tabs = root.querySelectorAll('.code-tab');
      var panels = root.querySelectorAll('.code-panel');
      tabs.forEach(function (tab) {
        tab.addEventListener('click', function () {
          var name = tab.getAttribute('data-tab');
          tabs.forEach(function (t) {
            var on = t === tab;
            t.classList.toggle('active', on);
            t.setAttribute('aria-selected', on ? 'true' : 'false');
          });
          panels.forEach(function (p) {
            var on = p.getAttribute('data-panel') === name;
            p.classList.toggle('active', on);
            if (on) p.removeAttribute('hidden');
            else p.setAttribute('hidden', '');
          });
        });
      });
    });
  }

  window.NyraSite = {
    t: t,
    getLang: function () {
      return state.lang;
    },
    getTheme: function () {
      return state.theme;
    },
    applyLang: applyLang,
    applyTheme: applyTheme,
  };

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
