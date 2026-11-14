/* QuickNotes mind maps: lays out the map parsed by src/mindmap.rs and draws
 * it as SVG. QM.update(map) redraws, QM.exportSvg() returns a standalone SVG.
 * Layouts: "both" (branches left and right of the centre), "right", "down".
 * Interactive pages zoom (Ctrl+wheel, +/−/0), pan (wheel, drag), fold
 * branches (knob) and report double-clicked nodes ({type: "line"}). */
(function () {
  'use strict';
  var NS = 'http://www.w3.org/2000/svg';
  var XLINK = 'http://www.w3.org/1999/xlink';
  var S = window.QM_STRINGS || {};
  var SANS = 'Cantarell, Inter, sans-serif';
  var MONO = "'DejaVu Sans Mono', monospace";

  var THEMES = {
    pastel: { bg: '#fbfaf7', text: '#3d3846', root: '#5e5c64', link: '#1a5fb4',
      palette: ['#f4a6a6', '#f9c99a', '#f2d675', '#b3dba0', '#9fd3d1', '#a4c2f4', '#c3b1e1', '#efb1d1'] },
    vivid: { bg: '#ffffff', text: '#241f31', root: '#241f31', link: '#1a5fb4',
      palette: ['#e01b24', '#ff7800', '#e5a50a', '#2ec27e', '#00a2c7', '#3584e4', '#9141ac', '#d6337e'] },
    mono: { bg: '#ffffff', text: '#1e1e1e', root: '#1e1e1e', link: '#1e1e1e',
      palette: ['#5e5c64', '#3d3846', '#77767b', '#241f31'] },
    dark: { bg: '#1e1e2e', text: '#e6e6ef', root: '#f5f5ff', link: '#89b4fa',
      palette: ['#f38ba8', '#fab387', '#f9e2af', '#a6e3a1', '#94e2d5', '#89b4fa', '#cba6f7', '#f5c2e7'] },
    ocean: { bg: '#f2f8fb', text: '#0b3c49', root: '#0b3c49', link: '#0077b6',
      palette: ['#0077b6', '#00a6c4', '#2a9d8f', '#457b9d', '#1d3557', '#3a86ff', '#5e8fb8', '#168aad'] },
    forest: { bg: '#f6f8f1', text: '#283618', root: '#283618', link: '#2d6a4f',
      palette: ['#606c38', '#bc6c25', '#2d6a4f', '#c08a3e', '#40916c', '#8a5a44', '#52796f', '#7f8c4a'] }
  };

  function currentTheme(name) {
    if (THEMES[name]) return THEMES[name];
    var root = document.documentElement;
    var cs = getComputedStyle(root);
    var dark = root.dataset.theme === 'dark';
    var v = function (n, d) { return cs.getPropertyValue(n).trim() || d; };
    return {
      bg: v('--bg', dark ? '#1e1e1e' : '#ffffff'),
      text: v('--fg', dark ? '#e6e6e6' : '#1f2328'),
      root: dark ? '#deddda' : '#3d3846',
      link: v('--link', dark ? '#78aeed' : '#1a5fb4'),
      palette: dark
        ? ['#78aeed', '#ffa348', '#8ff0a4', '#f66151', '#dc8add', '#f9f06b', '#99c1f1', '#cdab8f']
        : ['#3584e4', '#e66100', '#26a269', '#c01c28', '#9141ac', '#c88800', '#1c71d8', '#865e3c']
    };
  }

  // Black or white text on a fill colour.
  function contrast(color) {
    var c = document.createElement('canvas').getContext('2d');
    c.fillStyle = '#000'; c.fillStyle = color;
    var hex = c.fillStyle;
    if (hex[0] !== '#') return '#ffffff';
    var r = parseInt(hex.substr(1, 2), 16), g = parseInt(hex.substr(3, 2), 16), b = parseInt(hex.substr(5, 2), 16);
    var lum = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
    return lum > 0.62 ? '#1e1e1e' : '#ffffff';
  }

  // `color` faded towards `bg` (instead of opacity, which makes PDFs embed
  // the fonts once more for every transparent group).
  function fade(color, bg, amount) {
    function rgb(c) {
      var x = document.createElement('canvas').getContext('2d');
      x.fillStyle = '#000'; x.fillStyle = c;
      var h = x.fillStyle;
      return h[0] === '#' ? [1, 3, 5].map(function (i) { return parseInt(h.substr(i, 2), 16); }) : null;
    }
    var a = rgb(color), b = rgb(bg);
    if (!a || !b) return color;
    return 'rgb(' + a.map(function (v, i) { return Math.round(v * amount + b[i] * (1 - amount)); }).join(',') + ')';
  }

  function el(tag, attrs, parent) {
    var e = document.createElementNS(NS, tag);
    for (var k in attrs) if (attrs[k] !== undefined && attrs[k] !== null) e.setAttribute(k, attrs[k]);
    if (parent) parent.appendChild(e);
    return e;
  }

  // ----- text measurement and wrapping ---------------------------------------

  var meter = null, cache = {};
  function measure(text, font) {
    var key = font.size + '|' + font.weight + '|' + font.style + '|' + font.family + '|' + text;
    if (cache[key] !== undefined) return cache[key];
    if (!meter) {
      var svg = el('svg', { width: 0, height: 0, style: 'position:absolute;left:-9999px;top:0' });
      document.body.appendChild(svg);
      meter = el('text', { 'xml:space': 'preserve', style: 'white-space:pre' }, svg);
    }
    meter.setAttribute('font-size', font.size);
    meter.setAttribute('font-weight', font.weight);
    meter.setAttribute('font-style', font.style);
    meter.setAttribute('font-family', font.family);
    meter.textContent = text;
    var w = meter.getComputedTextLength();
    cache[key] = w;
    return w;
  }

  function runFont(run, base) {
    return {
      size: base.size,
      weight: run.b ? 700 : base.weight,
      style: run.i ? 'italic' : 'normal',
      family: run.c ? MONO : base.family
    };
  }

  // Lines of {run, t} pieces no wider than maxW.
  function wrap(runs, base, maxW) {
    var lines = [[]], width = [0];
    function add(run, t) {
      var line = lines[lines.length - 1], last = line[line.length - 1];
      if (last && last.run === run) last.t += t; else line.push({ run: run, t: t });
      width[width.length - 1] += measure(t, runFont(run, base));
    }
    runs.forEach(function (run) {
      run.t.split(/(\n| +)/).forEach(function (tok) {
        if (!tok) return;
        if (tok === '\n') { lines.push([]); width.push(0); return; }
        var w = measure(tok, runFont(run, base));
        var line = lines[lines.length - 1];
        if (/^ +$/.test(tok)) { if (line.length) add(run, ' '); return; }
        if (line.length && width[width.length - 1] + w > maxW) {
          // Drop the trailing space, start a new line.
          var last = line[line.length - 1];
          if (/ $/.test(last.t)) {
            width[width.length - 1] -= measure(' ', runFont(last.run, base));
            last.t = last.t.replace(/ $/, '');
          }
          lines.push([]); width.push(0);
        }
        add(run, tok);
      });
    });
    var widths = lines.map(function (line) {
      return line.reduce(function (s, p) { return s + measure(p.t, runFont(p.run, base)); }, 0);
    });
    return { lines: lines, widths: widths, width: Math.max.apply(null, widths.concat([0])) };
  }

  // ----- state ---------------------------------------------------------------

  var state = {
    map: null, interactive: false, theme: null,
    folded: {},          // node key → folded (overrides the source)
    view: { x: 0, y: 0, k: 1 }, moved: false,
    svg: null, vp: null, content: null
  };

  function post(msg) {
    if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.qn) {
      window.webkit.messageHandlers.qn.postMessage(JSON.stringify(msg));
    }
  }

  function plain(node) { return node.runs.map(function (r) { return r.t; }).join(''); }

  // ----- layout --------------------------------------------------------------

  function style(depth, layout, family) {
    if (depth === 0) return { size: 20, weight: 700, padX: 16, padY: 10, maxW: 320, box: 'fill', radius: 14 };
    // Only regular and bold: every weight is another font in a PDF.
    if (depth === 1) return { size: 16, weight: 700, padX: 12, padY: 7, maxW: 260, box: 'fill', radius: 9 };
    if (layout === 'down') return { size: 14, weight: 400, padX: 9, padY: 5, maxW: 220, box: 'outline', radius: 7 };
    return { size: 14, weight: 400, padX: 4, padY: 4, maxW: 280, box: 'line', radius: 0 };
  }

  // Builds the visible tree with sizes and colours.
  function build(node, depth, key, color, layout, family, palette, index) {
    var st = style(depth, layout, family);
    var own = node.color || (depth === 1 ? palette[index % palette.length] : color);
    var runs = node.runs.slice();
    if (!runs.length) runs = [{ t: ' ' }];
    var base = { size: st.size, weight: st.weight, style: 'normal', family: family };
    // Tasks: a drawn checkbox before the text (like the to-do lists).
    var boxW = node.task !== undefined ? Math.round(st.size * 0.9) + 6 : 0;
    var text = wrap(runs, base, st.maxW - boxW);
    var lineH = Math.round(st.size * 1.35);
    var k = key + '/' + plain(node);
    var folded = state.folded[k] !== undefined ? state.folded[k] : !!node.folded;
    var n = {
      src: node, depth: depth, key: k, color: own || '#888', st: st, base: base, text: text, lineH: lineH, boxW: boxW,
      w: Math.ceil(text.width + boxW + 2 * st.padX), h: Math.ceil(text.lines.length * lineH + 2 * st.padY),
      folded: depth > 0 && folded && node.children.length > 0, kids: []
    };
    if (!n.folded) {
      n.kids = node.children.map(function (c, i) {
        return build(c, depth + 1, k, own, layout, family, palette, i);
      });
    }
    return n;
  }

  function hgap(depth) { return depth === 1 ? 64 : 34; }
  function vgap(depth) { return depth === 1 ? 22 : 10; }

  function subH(n) {
    if (n.sh !== undefined) return n.sh;
    var s = 0;
    n.kids.forEach(function (c, i) { s += subH(c) + (i ? vgap(c.depth) : 0); });
    n.sh = Math.max(n.h, s);
    return n.sh;
  }

  function placeH(n, x, cy, dir) {
    n.dir = dir;
    n.x = dir > 0 ? x : x - n.w;
    n.y = cy - n.h / 2;
    placeKids(n, n.kids, dir, cy);
  }

  function placeKids(n, kids, dir, cy) {
    if (!kids.length) return;
    var total = 0;
    kids.forEach(function (c, i) { total += subH(c) + (i ? vgap(c.depth) : 0); });
    var y = cy - total / 2;
    var gap = hgap(kids[0].depth);
    kids.forEach(function (c) {
      var h = subH(c);
      placeH(c, dir > 0 ? n.x + n.w + gap : n.x - gap, y + h / 2, dir);
      y += h + vgap(c.depth);
    });
  }

  function subW(n) {
    if (n.sw !== undefined) return n.sw;
    var s = 0;
    n.kids.forEach(function (c, i) { s += subW(c) + (i ? 24 : 0); });
    n.sw = Math.max(n.w, s);
    return n.sw;
  }

  function placeD(n, cx, y) {
    n.dir = 0;
    n.x = cx - n.w / 2;
    n.y = y;
    var total = 0;
    n.kids.forEach(function (c, i) { total += subW(c) + (i ? 24 : 0); });
    var x = cx - total / 2;
    n.kids.forEach(function (c) {
      var w = subW(c);
      placeD(c, x + w / 2, y + n.h + 48);
      x += w + 24;
    });
  }

  function layout(root, mode) {
    if (mode === 'down') { placeD(root, 0, 0); return; }
    root.x = -root.w / 2; root.y = -root.h / 2; root.dir = 1;
    var kids = root.kids;
    var right = kids, left = [];
    if (mode === 'both' && kids.length > 1) {
      // First branches right, the rest left, balanced by height.
      var total = kids.reduce(function (s, c) { return s + subH(c); }, 0), acc = 0, split = kids.length;
      for (var i = 0; i < kids.length; i++) {
        if (i > 0 && acc + subH(kids[i]) / 2 > total / 2) { split = i; break; }
        acc += subH(kids[i]);
      }
      right = kids.slice(0, split);
      left = kids.slice(split);
    }
    placeKids(root, right, 1, 0);
    placeKids(root, left, -1, 0);
  }

  // Where connectors leave a node (towards dir) and arrive at it.
  function outAnchor(n, dir) {
    if (dir === 0) return { x: n.x + n.w / 2, y: n.y + n.h };
    var y = n.st.box === 'line' ? n.y + n.h : n.y + n.h / 2;
    return { x: dir > 0 ? n.x + n.w : n.x, y: y };
  }
  function inAnchor(n) {
    if (n.dir === 0) return { x: n.x + n.w / 2, y: n.y };
    var y = n.st.box === 'line' ? n.y + n.h : n.y + n.h / 2;
    return { x: n.dir > 0 ? n.x : n.x + n.w, y: y };
  }

  // ----- drawing -------------------------------------------------------------

  function drawConnectors(n, g) {
    n.kids.forEach(function (c) {
      var a = outAnchor(n, n.depth === 0 && c.dir !== 0 ? c.dir : n.dir), b = inAnchor(c);
      var d;
      if (c.dir === 0) {
        var my = (a.y + b.y) / 2;
        d = 'M' + a.x + ',' + a.y + ' C' + a.x + ',' + my + ' ' + b.x + ',' + my + ' ' + b.x + ',' + b.y;
      } else {
        var mx = (a.x + b.x) / 2;
        d = 'M' + a.x + ',' + a.y + ' C' + mx + ',' + a.y + ' ' + mx + ',' + b.y + ' ' + b.x + ',' + b.y;
      }
      el('path', {
        d: d, fill: 'none', stroke: c.color, 'stroke-linecap': 'round',
        'stroke-width': c.depth === 1 ? 3 : c.depth === 2 ? 2 : 1.5
      }, g);
      drawConnectors(c, g);
    });
  }

  function drawText(n, g, color, under) {
    var center = n.st.box !== 'line';
    var ascent = n.st.size * 0.95;
    var link = state.theme.link;
    var task = n.src.task;
    var done = task === true;
    var left = n.x + n.st.padX + n.boxW;
    if (task !== undefined) drawCheckbox(n, g, color, under, center ? n.x + (n.w - n.boxW - n.text.widths[0]) / 2 : n.x + n.st.padX);
    n.text.lines.forEach(function (line, i) {
      var x = center ? n.x + n.boxW + (n.w - n.boxW - n.text.widths[i]) / 2 : left;
      var y = n.y + n.st.padY + ascent + i * n.lineH + (n.lineH - n.st.size * 1.2) / 2;
      var t = el('text', {
        x: x, y: y, 'font-size': n.st.size, 'font-family': n.base.family, 'font-weight': n.base.weight,
        fill: done ? fade(color, under, 0.6) : color, 'xml:space': 'preserve', style: 'white-space:pre'
      }, g);
      line.forEach(function (p) {
        var r = p.run, parent = t;
        if (r.href) {
          parent = el('a', { href: r.href }, t);
          parent.setAttributeNS(XLINK, 'xlink:href', r.href);
        }
        var deco = [];
        if (r.href) deco.push('underline');
        if (r.s || done) deco.push('line-through');
        var span = el('tspan', {
          'font-weight': r.b ? 700 : null,
          'font-style': r.i ? 'italic' : null,
          'font-family': r.c ? MONO : null,
          'text-decoration': deco.length ? deco.join(' ') : null,
          fill: r.href && n.st.box !== 'fill' ? link : null
        }, parent);
        span.textContent = p.t;
      });
    });
  }

  // A checkbox like the to-do lists of the Markdown preview; a click
  // toggles the task in the source.
  function drawCheckbox(n, g, color, under, x) {
    var s = Math.round(n.st.size * 0.9);
    var y = n.y + n.st.padY + (n.lineH - s) / 2;
    var cg = el('g', { 'class': 'check' }, g);
    var fill = n.st.box === 'fill' || n.depth === 0 ? color : n.color;
    // An empty box has no fill: this makes all of it clickable.
    if (state.interactive) el('rect', { x: x - 2, y: y - 2, width: s + 4, height: s + 4, fill: 'transparent' }, cg);
    el('rect', {
      x: x + 0.75, y: y + 0.75, width: s - 1.5, height: s - 1.5, rx: 3,
      fill: n.src.task ? fill : 'none', stroke: n.src.task ? fill : fade(color, under, 0.7), 'stroke-width': 1.5
    }, cg);
    if (n.src.task) {
      var c = contrast(fill);
      el('path', {
        d: 'M' + (x + s * 0.25) + ',' + (y + s * 0.52) + ' L' + (x + s * 0.43) + ',' + (y + s * 0.7) + ' L' + (x + s * 0.76) + ',' + (y + s * 0.3),
        fill: 'none', stroke: c, 'stroke-width': 1.8, 'stroke-linecap': 'round', 'stroke-linejoin': 'round'
      }, cg);
    }
    if (state.interactive) {
      cg.addEventListener('click', function (e) {
        e.stopPropagation();
        post({ type: 'task', line: n.src.line });
      });
      cg.addEventListener('dblclick', function (e) { e.stopPropagation(); });
    }
  }

  // Interactive: a "+" next to the node (shown on hover) opens an input for
  // a new branch below it.
  function drawAdd(n, ng, bg) {
    var dir = n.depth === 0 ? 1 : n.dir;
    var hasKnob = n.depth > 0 && n.src.children.length > 0;
    var a = n.depth === 0 ? { x: n.x + n.w, y: n.y + n.h / 2 } : outAnchor(n, dir);
    var off = hasKnob ? 26 : 13;
    var cx = dir === 0 ? a.x : a.x + dir * off, cy = dir === 0 ? a.y + off : a.y;
    // Keeps the hover alive between node and button.
    var x1 = Math.min(n.x, cx - 10), x2 = Math.max(n.x + n.w, cx + 10);
    var y1 = Math.min(n.y, cy - 10), y2 = Math.max(n.y + n.h, cy + 10);
    ng.insertBefore(el('rect', { x: x1, y: y1, width: x2 - x1, height: y2 - y1, fill: 'transparent' }), ng.firstChild);
    var ag = el('g', { 'class': 'add' }, ng);
    el('circle', { cx: cx, cy: cy, r: 8, fill: n.color, stroke: bg, 'stroke-width': 1.5 }, ag);
    el('path', {
      d: 'M' + (cx - 4) + ',' + cy + ' H' + (cx + 4) + ' M' + cx + ',' + (cy - 4) + ' V' + (cy + 4),
      stroke: contrast(n.color), 'stroke-width': 1.8, 'stroke-linecap': 'round'
    }, ag);
    var title = el('title', {}, ag);
    title.textContent = S.add || '';
    ag.addEventListener('click', function (e) {
      e.stopPropagation();
      if (n.folded) state.folded[n.key] = false;
      openInput(n, cx, cy, dir);
    });
    ag.addEventListener('dblclick', function (e) { e.stopPropagation(); });
  }

  var input = null;
  function closeInput() {
    if (input) { input.remove(); input = null; }
  }
  function openInput(n, cx, cy, dir) {
    closeInput();
    var v = state.view;
    var sx = v.x + cx * v.k, sy = v.y + cy * v.k;
    input = document.createElement('input');
    input.className = 'add-input';
    input.placeholder = S.newBranch || '';
    input.style.borderColor = n.color;
    document.body.appendChild(input);
    var w = input.offsetWidth, h = input.offsetHeight;
    var left = dir < 0 ? sx - 14 - w : dir > 0 ? sx + 14 : sx - w / 2;
    var top = dir === 0 ? sy + 14 : sy - h / 2;
    // No room beside the button: below it.
    if (left < 4 || left + w > window.innerWidth - 4) { left = sx - w / 2; top = sy + 14; }
    input.style.left = Math.max(4, Math.min(left, window.innerWidth - w - 4)) + 'px';
    input.style.top = Math.max(4, Math.min(top, window.innerHeight - h - 4)) + 'px';
    var line = n.src.line;
    input.addEventListener('keydown', function (e) {
      e.stopPropagation();
      if (e.key === 'Escape') { closeInput(); return; }
      if (e.key === 'Enter' && input.value.trim()) {
        post({ type: 'add', line: line, text: input.value });
        // Stays open for the next branch.
        input.value = '';
      }
    });
    input.addEventListener('blur', function () { setTimeout(closeInput, 100); });
    input.focus();
  }

  function drawNode(n, g, bg) {
    var ng = el('g', { 'class': 'node' }, g);
    var textColor = state.theme.text;
    if (n.depth === 0) {
      var fill = state.theme.root;
      el('rect', { x: n.x, y: n.y, width: n.w, height: n.h, rx: n.st.radius, fill: fill }, ng);
      textColor = contrast(fill);
    } else if (n.st.box === 'fill') {
      el('rect', { x: n.x, y: n.y, width: n.w, height: n.h, rx: n.st.radius, fill: n.color }, ng);
      textColor = contrast(n.color);
    } else if (n.st.box === 'outline') {
      el('rect', { x: n.x, y: n.y, width: n.w, height: n.h, rx: n.st.radius, fill: bg, stroke: n.color, 'stroke-width': 1.5 }, ng);
    } else {
      el('line', {
        x1: n.x, y1: n.y + n.h, x2: n.x + n.w, y2: n.y + n.h, stroke: n.color,
        'stroke-width': n.depth === 2 ? 2 : 1.5, 'stroke-linecap': 'round'
      }, ng);
    }
    var under = n.depth === 0 ? state.theme.root : n.st.box === 'fill' ? n.color : bg;
    drawText(n, ng, textColor, under);
    if (state.interactive) drawAdd(n, ng, bg);
    if (state.interactive) {
      ng.addEventListener('dblclick', function (e) {
        e.preventDefault();
        post({ type: 'line', line: n.src.line });
      });
      var tip = el('title', {}, ng);
      tip.textContent = S.jump || '';
    }
    // Fold knob: on the side the branches leave.
    var count = n.src.children.length;
    if (n.depth > 0 && count && (state.interactive || n.folded)) {
      var a = outAnchor(n, n.dir);
      var dx = n.dir === 0 ? 0 : n.dir * 8, dy = n.dir === 0 ? 8 : 0;
      var kg = el('g', { 'class': 'knob' }, g);
      el('circle', { cx: a.x + dx, cy: a.y + dy, r: n.folded ? 8 : 5, fill: bg, stroke: n.color, 'stroke-width': 1.5 }, kg);
      if (n.folded) {
        var t = el('text', {
          x: a.x + dx, y: a.y + dy + 3.5, 'text-anchor': 'middle', 'font-size': 10,
          'font-family': n.base.family, fill: n.color, 'font-weight': 700
        }, kg);
        t.textContent = count > 9 ? '9+' : String(count);
      }
      if (state.interactive) {
        var title = el('title', {}, kg);
        title.textContent = S.fold || '';
        kg.addEventListener('click', function (e) {
          e.stopPropagation();
          state.folded[n.key] = !n.folded;
          render();
        });
      }
    }
    n.kids.forEach(function (c) { drawNode(c, g, bg); });
  }

  function render() {
    var map = state.map;
    if (!map) return;
    var theme = currentTheme(map.theme);
    if (map.colors && map.colors.length) theme = Object.assign({}, theme, { palette: map.colors });
    state.theme = theme;
    var family = map.font ? "'" + map.font.replace(/['"]/g, '') + "', " + SANS : SANS;
    document.body.style.background = theme.bg;
    var root = build(map.root, 0, '', theme.root, map.layout, family, theme.palette, 0);
    layout(root, map.layout);
    var content = el('g', { 'font-family': family });
    drawConnectors(root, el('g', {}, content));
    drawNode(root, el('g', {}, content), theme.bg);
    state.vp.textContent = '';
    state.vp.appendChild(content);
    state.content = content;
    state.family = family;
    if (!state.moved) fit();
    else applyView();
  }

  // ----- view ----------------------------------------------------------------

  function applyView() {
    var v = state.view;
    state.vp.setAttribute('transform', 'translate(' + v.x + ',' + v.y + ') scale(' + v.k + ')');
  }

  function fit() {
    if (!state.content) return;
    var b = state.content.getBBox();
    var W = state.svg.clientWidth || 800, H = state.svg.clientHeight || 600, m = 30;
    var k = Math.min((W - 2 * m) / Math.max(b.width, 1), (H - 2 * m) / Math.max(b.height, 1), 1.4);
    k = Math.max(k, 0.1);
    state.view = { k: k, x: W / 2 - (b.x + b.width / 2) * k, y: H / 2 - (b.y + b.height / 2) * k };
    applyView();
  }

  function zoom(factor, cx, cy) {
    var v = state.view;
    var k = Math.min(Math.max(v.k * factor, 0.1), 6);
    if (cx === undefined) { cx = state.svg.clientWidth / 2; cy = state.svg.clientHeight / 2; }
    v.x = cx - (cx - v.x) * (k / v.k);
    v.y = cy - (cy - v.y) * (k / v.k);
    v.k = k;
    state.moved = true;
    applyView();
  }

  function setupInteraction() {
    var svg = state.svg;
    svg.addEventListener('wheel', function (e) {
      e.preventDefault();
      if (e.ctrlKey) {
        var r = svg.getBoundingClientRect();
        zoom(Math.exp(-e.deltaY * 0.0025), e.clientX - r.left, e.clientY - r.top);
      } else {
        state.view.x -= e.deltaX;
        state.view.y -= e.deltaY;
        state.moved = true;
        applyView();
      }
    }, { passive: false });
    var drag = null;
    svg.addEventListener('pointerdown', function (e) {
      if (e.button !== 0 || e.target.closest('.knob, .check, .add, a')) return;
      drag = { x: e.clientX, y: e.clientY, vx: state.view.x, vy: state.view.y, id: e.pointerId, active: false };
    });
    svg.addEventListener('pointermove', function (e) {
      if (!drag) return;
      if (!(e.buttons & 1)) { end(); return; }
      // Capture only once it is a drag: capturing on press would take
      // clicks and double-clicks away from the nodes.
      if (!drag.active) {
        if (Math.abs(e.clientX - drag.x) + Math.abs(e.clientY - drag.y) <= 3) return;
        drag.active = true;
        state.moved = true;
        svg.setPointerCapture(drag.id);
        svg.classList.add('dragging');
      }
      state.view.x = drag.vx + e.clientX - drag.x;
      state.view.y = drag.vy + e.clientY - drag.y;
      applyView();
    });
    function end() { drag = null; svg.classList.remove('dragging'); }
    svg.addEventListener('pointerup', end);
    svg.addEventListener('pointercancel', end);
    document.addEventListener('keydown', function (e) {
      if (e.ctrlKey || e.altKey || e.metaKey || e.target.tagName === 'INPUT') return;
      if (e.key === '+' || e.key === '=') zoom(1.2);
      else if (e.key === '-') zoom(1 / 1.2);
      else if (e.key === '0') { state.moved = false; fit(); }
    });
    window.addEventListener('resize', function () { if (!state.moved) fit(); });

    var bar = document.createElement('div');
    bar.className = 'toolbar';
    [['−', S.zoomOut, function () { zoom(1 / 1.2); }],
     ['+', S.zoomIn, function () { zoom(1.2); }],
     ['⤢', S.fit, function () { state.moved = false; fit(); }]].forEach(function (b) {
      var btn = document.createElement('button');
      btn.textContent = b[0];
      btn.title = b[1] || '';
      btn.addEventListener('click', b[2]);
      bar.appendChild(btn);
    });
    document.body.appendChild(bar);
  }

  // ----- API -----------------------------------------------------------------

  window.QM = {
    init: function (opts) {
      state.interactive = !!(opts && opts.interactive);
      var host = document.getElementById('map');
      state.svg = el('svg', { width: '100%', height: '100%', 'class': 'canvas' }, host);
      state.vp = el('g', {}, state.svg);
      if (state.interactive) setupInteraction();
    },
    update: function (map) {
      var old = state.map;
      // Another map (e.g. other theme or layout): fit again.
      if (old && (old.layout !== map.layout)) state.moved = false;
      state.map = map;
      render();
    },
    exportSvg: function () {
      if (!state.content) return null;
      var b = state.content.getBBox(), m = 24;
      var w = Math.ceil(b.width + 2 * m), h = Math.ceil(b.height + 2 * m);
      var svg = document.createElementNS(NS, 'svg');
      svg.setAttribute('xmlns', NS);
      svg.setAttribute('xmlns:xlink', XLINK);
      svg.setAttribute('width', w);
      svg.setAttribute('height', h);
      svg.setAttribute('viewBox', (b.x - m) + ' ' + (b.y - m) + ' ' + w + ' ' + h);
      svg.setAttribute('font-family', state.family);
      el('rect', { x: b.x - m, y: b.y - m, width: w, height: h, fill: state.theme.bg }, svg);
      svg.appendChild(state.content.cloneNode(true));
      return { svg: new XMLSerializer().serializeToString(svg), width: w, height: h, background: state.theme.bg };
    }
  };

  window.setTheme = function (t, vars) {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.cssText = vars || '';
    render();
  };
})();
