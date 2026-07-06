/**
 * Add optional explicit types to Nyra snippets (mirrors scripts/gen-typed-examples.py).
 */
(function (global) {
  function arrayLenLiteral(line) {
    var m = line.match(/=\s*\[([^\]]*)\]/);
    if (!m) return null;
    var inner = m[1].trim();
    if (!inner) return 0;
    return inner.split(',').filter(function (p) {
      return p.trim();
    }).length;
  }

  function inferElemType(inner) {
    inner = inner.trim();
    if (!inner) return 'i32';
    var first = inner.split(',')[0].trim();
    if (/^-?\d+$/.test(first)) return 'i32';
    if (/^-?\d+\.\d+$/.test(first) || first.indexOf('.') !== -1) return 'f64';
    return 'i32';
  }

  function typeLetLine(line) {
    var stripped = line.trim();
    if (stripped.split('=')[0].indexOf(': ') !== -1) return line;

    var m;

    m = line.match(/^(\s*)let\s+mut\s+(\w+)\s*=\s*(\d+)\s*$/);
    if (m) return m[1] + 'let mut ' + m[2] + ': i32 = ' + m[3] + '\n';

    m = line.match(/^(\s*)let\s+(\w+)\s*=\s*(-?\d+)\s*$/);
    if (m) return m[1] + 'let ' + m[2] + ': i32 = ' + m[3] + '\n';

    m = line.match(/^(\s*)let\s+(\w+)\s*=\s*(input\(.+\))\s*$/);
    if (m) return m[1] + 'let ' + m[2] + ': string = ' + m[3] + '\n';

    m = line.match(/^(\s*)let\s+(\w+)\s*=\s*"[^"]*"\s*$/);
    if (m) return m[1] + 'let ' + m[2] + ': string = ' + m[3] + '\n';

    if (line.indexOf('.split(') !== -1 && /^\s*let\s+/.test(line)) return line;

    if (/=\s*\[/.test(line) && line.indexOf('split(') === -1 && line.indexOf('sort()') === -1) {
      m = line.match(/^(\s*)let\s+(\w+)\s*=\s*(\[.+\])\s*$/);
      if (m) {
        var n = arrayLenLiteral(line);
        if (n !== null) {
          var elem = inferElemType(m[3].slice(1, -1));
          return m[1] + 'let ' + m[2] + ': [' + elem + '; ' + n + '] = ' + m[3] + '\n';
        }
      }
    }

    if (line.indexOf('.sort()') !== -1) {
      m = line.match(/^(\s*)let\s+(\w+)\s*=\s*(\w+)\.sort\(\)\s*$/);
      if (m) return m[1] + 'let ' + m[2] + ': [i32; 5] = ' + m[3] + '.sort()\n';
    }

    if (line.indexOf('.split(') !== -1) return line;

    if (line.indexOf('date()') !== -1) {
      m = line.match(/^(\s*)let\s+(\w+)\s*=\s*date\(\)\s*$/);
      if (m) return m[1] + 'let ' + m[2] + ': Date = date()\n';
    }

    if (line.indexOf('Vec_i32_new()') !== -1) {
      m = line.match(/^(\s*)let\s+(\w+)\s*=\s*Vec_i32_new\(\)\s*$/);
      if (m) return m[1] + 'let ' + m[2] + ': ptr = Vec_i32_new()\n';
    }

    if (
      line.indexOf('Array_map') !== -1 ||
      line.indexOf('Array_filter') !== -1 ||
      line.indexOf('Array_reduce') !== -1
    ) {
      m = line.match(/^(\s*)let\s+(\w+)\s*=\s*(.+)\s*$/);
      if (m) {
        if (m[3].indexOf('Array_reduce') !== -1 || m[3].indexOf('Array_find') !== -1) {
          return m[1] + 'let ' + m[2] + ': i32 = ' + m[3] + '\n';
        }
        return m[1] + 'let ' + m[2] + ': ptr = ' + m[3] + '\n';
      }
    }

    if (line.indexOf('JSON_') !== -1) {
      m = line.match(/^(\s*)let\s+(\w+)\s*=\s*(.+)\s*$/);
      if (m) return m[1] + 'let ' + m[2] + ': string = ' + m[3] + '\n';
    }

    if (/^\s*let\s+/.test(line) && line.indexOf(' clone ') !== -1) {
      m = line.match(/^(\s*)let\s+(\w+)\s*=\s*clone\s+(\w+)\s*$/);
      if (m) return m[1] + 'let ' + m[2] + ': string = clone ' + m[3] + '\n';
    }

    return line;
  }

  function fixSortArrayTypes(text) {
    var lines = text.split('\n');
    var arrTypes = {};
    var out = [];
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i];
      var endsNl = line.length > 0 || i < lines.length - 1;
      var raw = line;
      if (i < lines.length - 1) line = line + '\n';

      var m = line.match(/^\s*let\s+(\w+):\s*(\[[^\]]+\])\s*=/);
      if (m) arrTypes[m[1]] = m[2];

      var m2 = line.match(/^(\s*)let\s+(\w+):\s*\[i32;\s*5\]\s*=\s*(\w+)\.sort\(\)/);
      if (m2) {
        var ty = arrTypes[m2[3]] || '[i32; 5]';
        out.push(m2[1] + 'let ' + m2[2] + ': ' + ty + ' = ' + m2[3] + '.sort()');
        continue;
      }

      if (/^\s*let /.test(raw) || /^\s*let mut /.test(raw)) {
        out.push(typeLetLine(line).replace(/\n$/, ''));
      } else {
        out.push(raw);
      }
    }
    return out.join('\n');
  }

  function transformSource(text) {
    var hadTrailingNl = /\n$/.test(text);
    text = text.replace(/fn main\(\)\s*\{/g, 'fn main() -> void {');
    text = text.replace(/fn main\(\)\s*->\s*void\s*->\s*void/g, 'fn main() -> void');

    var lines = text.split('\n');
    var out = [];
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i];
      var isLast = i === lines.length - 1;
      var withNl = isLast && !hadTrailingNl ? line : line + '\n';
      if (/^\s*let /.test(line) || /^\s*let mut /.test(line)) {
        out.push(typeLetLine(withNl));
      } else {
        out.push(isLast && !hadTrailingNl ? line : withNl);
      }
    }
    var joined = out.join('');
    if (!hadTrailingNl && joined.endsWith('\n')) joined = joined.slice(0, -1);
    return fixSortArrayTypes(joined);
  }

  global.NyraTypedTransform = { transformSource: transformSource };
})(typeof window !== 'undefined' ? window : globalThis);
