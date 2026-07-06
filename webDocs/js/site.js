/**
 * Nyra docs — theme (light/dark) + i18n (en/ar), sets html[dir] ltr/rtl.
 * Nav and header chrome are static HTML; this script only updates translated UI.
 */
(function () {
  var STORAGE_LANG = 'nyra-docs-lang';
  var STORAGE_THEME = 'nyra-docs-theme';

  /* Apply saved prefs before paint to avoid flash */
  (function applyEarlyPrefs() {
    var theme = localStorage.getItem(STORAGE_THEME);
    var lang = localStorage.getItem(STORAGE_LANG);
    if (theme !== 'light' && theme !== 'dark') theme = 'dark';
    if (lang !== 'ar' && lang !== 'en') lang = 'en';
    document.documentElement.setAttribute('data-theme', theme);
    document.documentElement.lang = lang;
    document.documentElement.dir = lang === 'ar' ? 'rtl' : 'ltr';
  })();
  var DEFAULT_LANG = 'en';
  var DEFAULT_THEME = 'dark';

  var state = {
    lang: DEFAULT_LANG,
    theme: DEFAULT_THEME,
    strings: {},
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

    updateLangToggleUi();
    localStorage.setItem(STORAGE_LANG, lang);
    if (window.NyraSearch && window.NyraSearch.applyI18n) {
      window.NyraSearch.applyI18n();
    }
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
    loadLocale(lang)
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

  function init() {
    var lang = localStorage.getItem(STORAGE_LANG) || DEFAULT_LANG;
    var theme = localStorage.getItem(STORAGE_THEME) || DEFAULT_THEME;
    if (lang !== 'en' && lang !== 'ar') lang = DEFAULT_LANG;
    if (theme !== 'light' && theme !== 'dark') theme = DEFAULT_THEME;

    bindToolbar();
    bindMobileNav();
    applyTheme(theme);
    ensureCodeTabs();
    highlightAllCodeBlocks();
    initCodeTabs();

    loadLocale(lang)
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
