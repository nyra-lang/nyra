/**
 * Lunr full-text search for Nyra webDocs — all pages, sections, and sidebar nav.
 */
(function () {
  var index = null;
  var docsById = {};
  var modal = null;
  var input = null;
  var resultsEl = null;
  var countEl = null;
  var activeIdx = -1;
  var MAX_RESULTS = 30;

  var strings = {
    placeholder: 'Search all docs… (Ctrl+K)',
    hint: '↑↓ navigate · Enter open · Esc close',
    emptyShort: 'Type to search all pages and sections',
    loading: 'Index loading…',
    noResults: 'No results — try another word',
    results: function (n) {
      return n === 1 ? '1 result' : n + ' results';
    },
    nav: 'Navigation',
    ariaLabel: 'Search documentation',
  };

  function t(key, arg) {
    if (window.NyraSite && window.NyraSite.t) {
      var val = window.NyraSite.t('search.' + key);
      if (val !== undefined) {
        if (key === 'results' && arg !== undefined) {
          if (arg === 1) {
            var one = window.NyraSite.t('search.resultsOne');
            if (one) return one;
          }
          return val.replace('{n}', String(arg));
        }
        return val;
      }
    }
    var s = strings[key];
    if (typeof s === 'function') return s(arg);
    return s || key;
  }

  function escapeHtml(text) {
    return String(text)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function escapeRegExp(text) {
    return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  }

  function highlight(text, q) {
    var plain = escapeHtml(text);
    var terms = q
      .trim()
      .split(/\s+/)
      .filter(Boolean)
      .map(function (term) {
        return term.replace(/[^\w\u0600-\u06FF-]/g, '');
      })
      .filter(function (term) {
        return term.length > 0;
      });
    if (!terms.length) return plain;
    var re = new RegExp('(' + terms.map(escapeRegExp).join('|') + ')', 'gi');
    return plain.replace(re, '<mark>$1</mark>');
  }

  function snippet(text, q, len) {
    len = len || 140;
    var lower = text.toLowerCase();
    var terms = q.trim().toLowerCase().split(/\s+/).filter(Boolean);
    var qi = -1;
    for (var i = 0; i < terms.length; i += 1) {
      var idx = lower.indexOf(terms[i]);
      if (idx >= 0 && (qi < 0 || idx < qi)) qi = idx;
    }
    if (qi < 0) {
      var s = text.slice(0, len);
      return highlight(s + (text.length > len ? '…' : ''), q);
    }
    var start = Math.max(0, qi - 50);
    var chunk = text.slice(start, start + len);
    return (
      (start > 0 ? '…' : '') +
      highlight(chunk, q) +
      (start + len < text.length ? '…' : '')
    );
  }

  function buildLunrQuery(q) {
    return q
      .trim()
      .split(/\s+/)
      .filter(Boolean)
      .map(function (term) {
        term = term.replace(/[^\w\u0600-\u06FF-]/g, '');
        if (!term) return '';
        return term.length >= 2 ? term + '*' : term;
      })
      .filter(Boolean)
      .join(' ');
  }

  function pagePath(url) {
    var hash = url.indexOf('#');
    if (hash < 0) return url;
    var base = url.slice(0, hash);
    var anchor = url.slice(hash + 1);
    return anchor === 'intro' ? base : base + ' § ' + anchor.replace(/-/g, ' ');
  }

  function createModal() {
    if (document.getElementById('search-modal')) return;
    modal = document.createElement('div');
    modal.id = 'search-modal';
    modal.className = 'search-modal';
    modal.hidden = true;
    modal.innerHTML =
      '<div class="search-dialog" role="dialog" aria-modal="true" aria-label="' +
      escapeHtml(strings.ariaLabel) +
      '">' +
      '<div class="search-input-wrap">' +
      '<span class="search-icon" aria-hidden="true">⌕</span>' +
      '<input type="search" id="search-input" autocomplete="off" spellcheck="false" data-i18n-placeholder="search.placeholder" placeholder="' +
      escapeHtml(strings.placeholder) +
      '" />' +
      '<kbd class="search-kbd">Esc</kbd></div>' +
      '<p id="search-count" class="search-count" hidden></p>' +
      '<ul id="search-results" class="search-results"></ul>' +
      '<p class="search-hint" data-i18n="search.hint">' +
      escapeHtml(strings.hint) +
      '</p></div>';
    document.body.appendChild(modal);
    input = document.getElementById('search-input');
    resultsEl = document.getElementById('search-results');
    countEl = document.getElementById('search-count');
    modal.addEventListener('click', function (e) {
      if (e.target === modal) closeSearch();
    });
    input.addEventListener('input', function () {
      runSearch(input.value);
    });
    input.addEventListener('keydown', function (e) {
      var items = resultsEl.querySelectorAll('.search-result');
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        activeIdx = Math.min(activeIdx + 1, items.length - 1);
        highlightActive(items);
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        activeIdx = Math.max(activeIdx - 1, 0);
        highlightActive(items);
      } else if (e.key === 'Enter' && activeIdx >= 0 && items[activeIdx]) {
        e.preventDefault();
        items[activeIdx].click();
      } else if (e.key === 'Escape') {
        closeSearch();
      }
    });
  }

  function highlightActive(items) {
    items.forEach(function (el, i) {
      el.classList.toggle('active', i === activeIdx);
    });
    if (items[activeIdx]) items[activeIdx].scrollIntoView({ block: 'nearest' });
  }

  function substringSearch(q, limit) {
    var terms = q
      .trim()
      .toLowerCase()
      .split(/\s+/)
      .filter(Boolean);
    if (!terms.length) return [];
    var hits = [];
    for (var id in docsById) {
      if (!Object.prototype.hasOwnProperty.call(docsById, id)) continue;
      var doc = docsById[id];
      var hay = (
        doc.title +
        ' ' +
        doc.heading +
        ' ' +
        doc.pageTitle +
        ' ' +
        doc.body +
        ' ' +
        doc.section
      ).toLowerCase();
      var ok = terms.every(function (term) {
        return hay.indexOf(term) >= 0;
      });
      if (ok) {
        hits.push({
          ref: id,
          score: doc.kind === 'nav' ? 3 : doc.heading ? 2 : 1,
        });
      }
    }
    hits.sort(function (a, b) {
      return b.score - a.score;
    });
    return hits.slice(0, limit);
  }

  function mergeHits(primary, extra, limit) {
    var seen = {};
    var merged = [];
    primary.forEach(function (hit) {
      if (seen[hit.ref]) return;
      seen[hit.ref] = true;
      merged.push(hit);
    });
    extra.forEach(function (hit) {
      if (seen[hit.ref] || merged.length >= limit) return;
      seen[hit.ref] = true;
      merged.push(hit);
    });
    return merged.slice(0, limit);
  }

  function runSearch(q) {
    activeIdx = -1;
    resultsEl.innerHTML = '';
    countEl.hidden = true;

    if (!q || !q.trim()) {
      resultsEl.innerHTML =
        '<li class="search-empty">' + escapeHtml(t('emptyShort')) + '</li>';
      return;
    }

    if (!index) {
      resultsEl.innerHTML =
        '<li class="search-empty">' + escapeHtml(t('loading')) + '</li>';
      return;
    }

    var query = buildLunrQuery(q);
    var hits = [];
    if (index) {
      try {
        hits = query ? index.search(query) : index.search(q.trim());
      } catch (err) {
        try {
          hits = index.search(q.trim());
        } catch (ignored) {
          hits = [];
        }
      }
    }
    hits = mergeHits(hits, substringSearch(q, MAX_RESULTS), MAX_RESULTS);

    if (!hits.length) {
      resultsEl.innerHTML =
        '<li class="search-empty">' + escapeHtml(t('noResults')) + '</li>';
      return;
    }

    countEl.textContent = t('results', hits.length);
    countEl.hidden = false;

    hits.forEach(function (hit) {
      var doc = docsById[hit.ref];
      if (!doc) return;

      var li = document.createElement('li');
      li.className = 'search-result' + (doc.kind === 'nav' ? ' search-result-nav' : '');

      var titleLine = doc.heading || doc.pageTitle || doc.title;
      var metaLine =
        doc.kind === 'nav'
          ? t('nav') + ' · ' + doc.url
          : pagePath(doc.url);

      li.innerHTML =
        '<span class="search-result-title">' +
        highlight(titleLine, q) +
        '</span>' +
        '<span class="search-result-meta">' +
        '<span class="search-result-section">' +
        escapeHtml(doc.section) +
        '</span>' +
        '<span class="search-result-path">' +
        escapeHtml(metaLine) +
        '</span></span>' +
        '<span class="search-result-snippet">' +
        snippet(doc.body, q) +
        '</span>';

      li.addEventListener('click', function () {
        window.location.href = doc.url;
      });
      resultsEl.appendChild(li);
    });

    activeIdx = 0;
    highlightActive(resultsEl.querySelectorAll('.search-result'));
  }

  function openSearch() {
    createModal();
    modal.hidden = false;
    document.body.classList.add('search-open');
    input.value = '';
    countEl.hidden = true;
    resultsEl.innerHTML =
      '<li class="search-empty">' + escapeHtml(t('emptyShort')) + '</li>';
    setTimeout(function () {
      input.focus();
    }, 50);
  }

  function closeSearch() {
    if (!modal) return;
    modal.hidden = true;
    document.body.classList.remove('search-open');
  }

  function loadIndex() {
    return fetch('search-index.json')
      .then(function (r) {
        return r.json();
      })
      .then(function (data) {
        data.docs.forEach(function (d) {
          docsById[d.id] = d;
        });
        index = lunr(function () {
          this.ref('id');
          this.field('title', { boost: 12 });
          this.field('heading', { boost: 10 });
          this.field('pageTitle', { boost: 8 });
          this.field('section', { boost: 6 });
          this.field('body');
          data.docs.forEach(
            function (doc) {
              this.add(doc);
            }.bind(this)
          );
        });
      })
      .catch(function () {
        console.warn('search-index.json not found');
      });
  }

  function applySearchI18n() {
    if (!modal) return;
    if (input) input.placeholder = t('placeholder');
    var hint = modal.querySelector('.search-hint');
    if (hint) hint.textContent = t('hint');
    var dialog = modal.querySelector('.search-dialog');
    if (dialog) dialog.setAttribute('aria-label', t('ariaLabel'));
  }

  function init() {
    createModal();
    applySearchI18n();
    loadIndex();
    var btn = document.getElementById('search-open');
    if (btn) btn.addEventListener('click', openSearch);
    var sidebar = document.getElementById('sidebar-search');
    if (sidebar) {
      sidebar.addEventListener('click', function (e) {
        e.preventDefault();
        openSearch();
      });
    }
    document.addEventListener('keydown', function (e) {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        openSearch();
      }
    });
  }

  window.NyraSearch = {
    applyI18n: applySearchI18n,
  };

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
