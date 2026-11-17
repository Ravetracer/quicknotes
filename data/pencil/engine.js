// QuickNotes – engine for Pencil stencil collections (live shapes).
//
// A clean-room implementation written for QuickNotes from the published
// structure of Pencil collections (Definition.xml: typed properties, behaviours
// written as JavaScript expressions, an SVG template per shape, shortcuts and
// collection scripts). It runs in a hidden web view without network access and
// without a message bridge: Rust calls the functions of `PE` with one JSON
// string and gets a JSON string back.
//
//   PE.loadPackage({pkg, files})            -> {ok, id, name, fontCss}
//   PE.render({pkg, s, props, w, h, oldW, oldH}) -> {ok, svg, w, h, props, schema, edits, handles, box, actions}
//   PE.action({pkg, s, props, w, h, action}) -> {ok, props, w, h}
//
// Expressions of a collection are evaluated like Pencil does: `$name` stands
// for the shape property `properties.name`, `$$name` for a collection
// property, and the value of the last statement is the result. That needs
// sloppy-mode `eval` inside `with`, which is why this file is not strict.

var PE_NS = {
  svg: 'http://www.w3.org/2000/svg',
  xlink: 'http://www.w3.org/1999/xlink',
  p: 'http://www.evolus.vn/Namespace/Pencil',
  html: 'http://www.w3.org/1999/xhtml'
};
var PencilNamespaces = { svg: PE_NS.svg, xlink: PE_NS.xlink, p: PE_NS.p, html: PE_NS.html, xul: 'http://www.mozilla.org/keymaster/gatekeeper/there.is.only.xul' };

/// Evaluates `__code` with the names of `__scope` in reach; the result is the
/// completion value of the code.
function __peEval(__scope, __code) {
  with (__scope) {
    return eval(__code);
  }
}

// ===== value types ===========================================================

function pe_num(v, d) { var n = parseFloat(v); return isFinite(n) ? n : (d || 0); }
function pe_round(n) { return Math.round(n * 1000) / 1000; }

function Dimension(w, h) { this.w = pe_num(w); this.h = pe_num(h); }
Dimension.fromString = function (s) {
  var p = String(s || '').split(',');
  return new Dimension(pe_num(p[0]), pe_num(p[1]));
};
Dimension.prototype.toString = function () { return pe_round(this.w) + ',' + pe_round(this.h); };
Dimension.prototype.narrowed = function (dx, dy) {
  if (dy === undefined) dy = dx;
  return new Dimension(this.w - 2 * dx, this.h - 2 * dy);
};

function Bound(x, y, w, h) { this.x = pe_num(x); this.y = pe_num(y); this.w = pe_num(w); this.h = pe_num(h); }
Bound.fromString = function (s) {
  var p = String(s || '').split(',');
  return new Bound(p[0], p[1], p[2], p[3]);
};
Bound.fromBox = function (box, px, py) {
  px = px || 0;
  py = py === undefined ? px : py;
  return new Bound(px, py, box.w - 2 * px, box.h - 2 * py);
};
Bound.prototype.toString = function () { return [this.x, this.y, this.w, this.h].map(pe_round).join(','); };
Bound.prototype.narrowed = function (dx, dy) {
  if (dy === undefined) dy = dx;
  return new Bound(this.x + dx, this.y + dy, this.w - 2 * dx, this.h - 2 * dy);
};
Bound.prototype.shifted = function (dx, dy) { return new Bound(this.x + (dx || 0), this.y + (dy || 0), this.w, this.h); };

function Handle(x, y) { this.x = pe_num(x); this.y = pe_num(y); }
Handle.REG_EX = /^([\-0-9\.]+)\,([\-0-9\.]+)$/;
Handle.fromString = function (s) {
  var m = Handle.REG_EX.exec(String(s || '').trim());
  return m ? new Handle(Math.round(parseFloat(m[1])), Math.round(parseFloat(m[2]))) : new Handle(0, 0);
};
Handle.prototype.toString = function () { return Math.round(this.x) + ',' + Math.round(this.y); };
Handle.prototype.applyMinX = function (v) { if (typeof v === 'number' && this.x < v) this.x = v; };
Handle.prototype.applyMaxX = function (v) { if (typeof v === 'number' && this.x > v) this.x = v; };
Handle.prototype.applyMinY = function (v) { if (typeof v === 'number' && this.y < v) this.y = v; };
Handle.prototype.applyMaxY = function (v) { if (typeof v === 'number' && this.y > v) this.y = v; };
Handle.prototype.applyExpressionX = function (v) { if (typeof v === 'number') this.x = v; };
Handle.prototype.applyExpressionY = function (v) { if (typeof v === 'number') this.y = v; };

function Point(x, y) { this.x = pe_num(x); this.y = pe_num(y); }
Point.fromString = function (s) { var p = String(s || '').split(','); return new Point(p[0], p[1]); };
Point.prototype.toString = function () { return pe_round(this.x) + ',' + pe_round(this.y); };
Point.prototype.translate = function (dx, dy) { return new Point(this.x + dx, this.y + dy); };

// ----- colours

var PE_NAMED = {
  black: '000000', white: 'ffffff', red: 'ff0000', green: '008000', blue: '0000ff', gray: '808080', grey: '808080',
  yellow: 'ffff00', orange: 'ffa500', purple: '800080', silver: 'c0c0c0', navy: '000080', teal: '008080',
  maroon: '800000', lime: '00ff00', aqua: '00ffff', fuchsia: 'ff00ff', olive: '808000'
};

function Color(r, g, b, a) {
  this.r = r === undefined ? 0 : r;
  this.g = g === undefined ? 0 : g;
  this.b = b === undefined ? 0 : b;
  this.a = a === undefined ? 1 : a;
}
Color.fromString = function (s) {
  s = String(s === undefined || s === null ? '' : s).trim();
  var c = new Color(0, 0, 0, 1), m;
  var low = s.toLowerCase();
  if (low === 'transparent' || low === 'none' || low === '') return new Color(0, 0, 0, 0);
  if (PE_NAMED[low]) s = '#' + PE_NAMED[low];
  if ((m = /^#([0-9a-f]{3,4})$/i.exec(s))) {
    var x = m[1];
    c.r = parseInt(x[0] + x[0], 16); c.g = parseInt(x[1] + x[1], 16); c.b = parseInt(x[2] + x[2], 16);
    if (x.length === 4) c.a = parseInt(x[3] + x[3], 16) / 255;
  } else if ((m = /^#([0-9a-f]{6})([0-9a-f]{2})?$/i.exec(s))) {
    c.r = parseInt(m[1].substr(0, 2), 16); c.g = parseInt(m[1].substr(2, 2), 16); c.b = parseInt(m[1].substr(4, 2), 16);
    if (m[2]) c.a = parseInt(m[2], 16) / 255;
  } else if ((m = /^rgba?\(([^)]*)\)$/i.exec(s))) {
    var p = m[1].split(/[ ,\/]+/).filter(Boolean);
    c.r = Math.round(pe_num(p[0])); c.g = Math.round(pe_num(p[1])); c.b = Math.round(pe_num(p[2]));
    if (p.length > 3) c.a = p[3].indexOf('%') > 0 ? pe_num(p[3]) / 100 : pe_num(p[3], 1);
  }
  return c;
};
Color.fromHSV = function (h, s, v) {
  h = ((h % 360) + 360) % 360; s = Math.max(0, Math.min(1, s)); v = Math.max(0, Math.min(1, v));
  var k = function (n) { var t = (n + h / 60) % 6; return v - v * s * Math.max(0, Math.min(t, 4 - t, 1)); };
  return new Color(Math.round(k(5) * 255), Math.round(k(3) * 255), Math.round(k(1) * 255), 1);
};
function pe_hex2(n) { n = Math.max(0, Math.min(255, Math.round(n))); return (n < 16 ? '0' : '') + n.toString(16).toUpperCase(); }
Color.prototype.toString = function () { return '#' + pe_hex2(this.r) + pe_hex2(this.g) + pe_hex2(this.b) + pe_hex2(this.a * 255); };
Color.prototype.toRGBString = function () { return 'rgb(' + Math.round(this.r) + ', ' + Math.round(this.g) + ', ' + Math.round(this.b) + ')'; };
Color.prototype.toRGBAString = function () {
  return 'rgba(' + Math.round(this.r) + ', ' + Math.round(this.g) + ', ' + Math.round(this.b) + ', ' + pe_round(this.a) + ')';
};
Color.prototype.getHSV = function () {
  var r = this.r / 255, g = this.g / 255, b = this.b / 255;
  var max = Math.max(r, g, b), min = Math.min(r, g, b), d = max - min, h = 0;
  if (d) {
    if (max === r) h = 60 * (((g - b) / d) % 6);
    else if (max === g) h = 60 * ((b - r) / d + 2);
    else h = 60 * ((r - g) / d + 4);
  }
  if (h < 0) h += 360;
  return [h, max ? d / max : 0, max];
};
Color.prototype.shaded = function (p) {
  var hsv = this.getHSV();
  var c = Color.fromHSV(hsv[0], hsv[1], hsv[2] * (1 - (p || 0)));
  c.a = this.a;
  return c;
};
Color.prototype.hollowed = function (p) { return new Color(this.r, this.g, this.b, this.a * (1 - (p || 0))); };
Color.prototype.transparent = function () { return new Color(this.r, this.g, this.b, 0); };
Color.prototype.inverse = function () { return new Color(255 - this.r, 255 - this.g, 255 - this.b, this.a); };

// ----- fonts

function Font(family, weight, style, size, decor, lineHeight) {
  this.family = family || 'sans-serif';
  this.weight = weight || 'normal';
  this.style = style || 'normal';
  this.size = size || '12px';
  this.decor = decor || 'none';
  this.lineHeight = lineHeight || 0;
}
Font.REG_EX = /^([^\|]+)\|([^\|]+)\|([^\|]+)\|([0-9]+[a-z]+)$/i;
Font.fromString = function (s) {
  var p = String(s || '').split('|');
  if (p.length < 4) return new Font();
  return new Font(p[0], p[1], p[2], p[3], p[4] || 'none', p[5] ? parseFloat(p[5]) || 0 : 0);
};
Font.parsePixelHeight = function (size) { return parseInt(size, 10) || 0; };
Font.prototype.getPixelHeight = function () { return Font.parsePixelHeight(this.size); };
Font.prototype.toString = function () { return [this.family, this.weight, this.style, this.size, this.decor, this.lineHeight].join('|'); };
Font.prototype.toCSSFontString = function () {
  return [this.style, this.weight, this.size, this.family].join(' ');
};
Font.prototype.getFamilies = function () {
  return this.family.split(',').map(function (f) { return f.trim().replace(/^['"]|['"]$/g, ''); });
};
Font.prototype.isUnderlined = function () { return this.decor === 'underline'; };
Font.prototype.copy = function () { return Font.fromString(this.toString()); };
Font.prototype.bold = function (yes) { var f = this.copy(); f.weight = (yes === undefined || yes) ? 'bold' : 'normal'; return f; };
Font.prototype.italic = function (yes) { var f = this.copy(); f.style = (yes === undefined || yes) ? 'italic' : 'normal'; return f; };
Font.prototype.getWeightAsNumber = function () {
  if (this.weight === 'bold') return 700;
  if (this.weight === 'normal') return 400;
  return parseInt(this.weight, 10) || 400;
};
Font.prototype.bolder = function (n) {
  var w = Math.max(100, Math.min(900, this.getWeightAsNumber() + (n || 0)));
  w -= w % 100;
  var f = this.copy();
  f.weight = w === 400 ? 'normal' : w === 700 ? 'bold' : String(w);
  return f;
};
Font.prototype.resized = function (d) {
  var f = this.copy(), px = this.getPixelHeight(), m;
  if (typeof d === 'string' && (m = /^(.+)%$/.exec(d))) f.size = Math.round(px * (1 + parseFloat(m[1]) / 100)) + 'px';
  else if (typeof d === 'number') f.size = Math.round(px * (1 + d)) + 'px';
  return f;
};

// ----- strokes, shadows

function StrokeStyle(w, array) {
  this.w = typeof w === 'number' ? w : (w === undefined ? 1 : pe_num(w, 1));
  this.array = array === undefined ? null : array;
}
StrokeStyle.REG_EX = /^([0-9]+)\|([0-9 \,]*)$/;
StrokeStyle.fromString = function (s) {
  var p = String(s || '').split('|');
  var st = new StrokeStyle(Math.round(pe_num(p[0], 1)), (p[1] || '').trim());
  if (st.array === '0') st.array = '';
  return st;
};
StrokeStyle.prototype.toString = function () { return this.w + '|' + (this.array || ''); };
StrokeStyle.prototype.condensed = function (r) { return new StrokeStyle(Math.round(this.w * (1 + (r || 0))), this.array); };
StrokeStyle.prototype.styled = function (array) { return new StrokeStyle(this.w, array); };

function ShadowStyle(dx, dy, size, opacity, color) {
  this.dx = dx || 0; this.dy = dy || 0; this.size = size === undefined ? 3 : size;
  this.opacity = opacity === undefined ? 1 : opacity; this.color = color || '#000000';
}
ShadowStyle.fromString = function (s) {
  var p = String(s || '').split('|');
  if (p.length < 3) return new ShadowStyle();
  return new ShadowStyle(parseInt(p[0], 10) || 0, parseInt(p[1], 10) || 0, pe_num(p[2]), p[3] ? pe_num(p[3], 1) : 1, p[4] || '#000000');
};
ShadowStyle.prototype.toString = function () { return [this.dx, this.dy, this.size, this.opacity, this.color].join('|'); };
ShadowStyle.prototype.toCSSString = function (color) {
  var css = this.dx + 'px ' + this.dy + 'px ' + Math.abs(this.size) + 'px ' + (color ? color.toRGBAString() : this.color);
  return this.size < 0 ? 'inset ' + css : css;
};

// ----- small value types

function Alignment(h, v) { this.h = pe_num(h); this.v = pe_num(v); }
Alignment.fromString = function (s) { var p = String(s || '').split(','); return new Alignment(parseInt(p[0], 10) || 0, parseInt(p[1], 10) || 0); };
Alignment.prototype.toString = function () { return this.h + ',' + this.v; };

function Bool(v) { this.value = !!v; }
Bool.fromString = function (s) { return new Bool(String(s).trim() === 'true'); };
Bool.prototype.toString = function () { return this.value ? 'true' : 'false'; };
Bool.prototype.negative = function () { return new Bool(!this.value); };

function Num(v) { this.value = pe_num(v); }
Num.fromString = function (s) { return new Num(pe_num(s)); };
Num.prototype.toString = function () { return String(this.value); };

function Enum(v) { this.value = v === undefined || v === null ? '' : String(v); }
Enum.fromString = function (s) { return new Enum(String(s === undefined ? '' : s)); };
Enum.prototype.toString = function () { return this.value; };

function PlainText(v) { this.value = v === undefined || v === null ? '' : String(v); }
PlainText.fromString = function (s) { return new PlainText(s); };
PlainText.prototype.toString = function () { return this.value; };
PlainText.prototype.toUpper = function () { return new PlainText(this.value.toUpperCase()); };

function RichText(v) { this.value = v === undefined || v === null ? '' : String(v); this.html = this.value; }
RichText.fromString = function (s) { return new RichText(s); };
RichText.prototype.toString = function () { return this.value; };

// ----- images

function ImageData(w, h, data, xCells, yCells) {
  this.w = pe_num(w); this.h = pe_num(h);
  this.data = data === undefined || data === null ? '' : String(data);
  this.xCells = xCells || null;
  this.yCells = yCells || null;
}
ImageData.parseCells = function (s) {
  return String(s || '').split(/\s+/).map(function (b) {
    var m = /^(\d+)-(\d+)$/.exec(b.trim());
    return m ? { from: parseInt(m[1], 10), to: parseInt(m[2], 10) } : null;
  }).filter(Boolean);
};
ImageData.cellString = function (cells) { return (cells || []).map(function (c) { return c.from + '-' + c.to; }).join(' '); };
ImageData.fromString = function (s) {
  s = String(s === undefined || s === null ? '' : s);
  var m = /^([0-9]+),([0-9]+),([0-9\- ]*),([0-9\- ]*),([^\0]+)$/.exec(s);
  if (m && /\d-\d/.test(m[3] + m[4])) return new ImageData(+m[1], +m[2], m[5], ImageData.parseCells(m[3]), ImageData.parseCells(m[4]));
  m = /^([0-9]+),([0-9]+),([^\0]*)$/.exec(s);
  if (m) return new ImageData(+m[1], +m[2], m[3]);
  return new ImageData(0, 0, s);
};
ImageData.prototype.toString = function () {
  if (this.xCells && this.xCells.length || this.yCells && this.yCells.length) {
    return [this.w, this.h, ImageData.cellString(this.xCells), ImageData.cellString(this.yCells), this.data].join(',');
  }
  return this.w + ',' + this.h + ',' + this.data;
};
ImageData.idToRefString = function (id) { return 'ref://' + id; };
ImageData.refStringToUrl = function (ref) { return PE.resolveUrl(ref); };
ImageData.prompt = function (cb) { if (cb) cb(null); };
ImageData.prototype.getDataAsXML = function () { return PE.resolveText(this.data); };
ImageData.prototype.isEmpty = function () { return !(this.w > 0) || PE.isEmptyImage(this.data); };

var PE_TYPES = {
  Dimension: Dimension, Bound: Bound, Handle: Handle, Point: Point, Color: Color, Font: Font,
  StrokeStyle: StrokeStyle, ShadowStyle: ShadowStyle, Alignment: Alignment, Bool: Bool, Num: Num,
  Enum: Enum, PlainText: PlainText, RichText: RichText, ImageData: ImageData
};

// ===== expression helpers (globals stencils use) ==============================

var PE_last = { x: 0, y: 0 };
function pe_path(cmd, nums) { return cmd + ' ' + nums.join(' '); }
function M(x, y) { PE_last = { x: x, y: y }; return pe_path('M', [x, y]); }
function L(x, y) { PE_last = { x: x, y: y }; return pe_path('L', [x, y]); }
function C(x1, y1, x2, y2, x, y) { PE_last = { x: x, y: y }; return pe_path('C', [x1, y1, x2, y2, x, y]); }
function c(x1, y1, x2, y2, x, y) { return pe_path('c', [x1, y1, x2, y2, x, y]); }
function S(x2, y2, x, y) { PE_last = { x: x, y: y }; return pe_path('S', [x2, y2, x, y]); }
function s(x2, y2, x, y) { return pe_path('s', [x2, y2, x, y]); }
function Q(x1, y1, x, y) { PE_last = { x: x, y: y }; return pe_path('Q', [x1, y1, x, y]); }
function q(x1, y1, x, y) { return pe_path('q', [x1, y1, x, y]); }
function T(x, y) { PE_last = { x: x, y: y }; return pe_path('T', [x, y]); }
function A(rx, ry, r, f1, f2, x, y) { PE_last = { x: x, y: y }; return pe_path('A', [rx, ry, r, f1, f2, x, y]); }
function a(rx, ry, r, f1, f2, x, y) { return pe_path('a', [rx, ry, r, f1, f2, x, y]); }
var z = 'z';
function sk(x1, y1, x2, y2, d, noMove) { PE_last = { x: x2, y: y2 }; return (noMove ? '' : 'M ' + x1 + ' ' + y1 + ' ') + 'L ' + x2 + ' ' + y2; }
function skTo(x, y) { PE_last = { x: x, y: y }; return 'L ' + x + ' ' + y; }

function translate(x, y) { return 'translate(' + (x || 0) + ',' + (y || 0) + ')'; }
function scale(x, y) { return 'scale(' + x + ',' + (y === undefined ? x : y) + ')'; }
function rotate(r, x, y) { return 'rotate(' + r + (x !== undefined ? ',' + x + ',' + (y || 0) : '') + ')'; }
function skewX(v) { return 'skewX(' + v + ')'; }
function skewY(v) { return 'skewY(' + v + ')'; }

function debug() {}
var Console = { dumpError: function () {} };
var tick = function () {};
var clipboard = { readImage: function () { return null; }, readText: function () { return ''; }, writeText: function () {} };
var Canvas = { domParser: new DOMParser() };
var Pencil = {
  controller: {
    nativeImageToRefSync: function () { return null; },
    svgImageToRefSync: function () { return null; },
    activeCanvas: null
  },
  behaviors: {},
  activeCanvas: null
};
var CollectionResourceBrowserDialog = {
  TYPE_BITMAP: 'bitmap', TYPE_SVG: 'svg',
  RETURN_IMAGEDATA: 'imagedata', RETURN_CONTENT: 'content', RETURN_DOMCONTENT: 'domcontent',
  lastCollectionId: null,
  open: function () {},
  _getParsedSvgSize: function () { return { w: 24, h: 24 }; }
};
var CollectionManager = {
  findCollection: function (id) {
    for (var k in PE.packages) if (PE.packages[k].collection.id === id) return PE.packages[k].collection;
    return null;
  }
};
function SnappingData(name, pos, type, vertical, id) {
  this.name = name; this.pos = pos; this.type = type; this.vertical = vertical; this.id = id;
}
SnappingData.prototype.makeLocal = function () { return this; };

var Svg = {
  setStyle: function (n, name, v) { pe_style(n, name, v); },
  removeStyle: function (n, name) { pe_style(n, name, null); },
  setWidth: function (n, v) { n.setAttribute('width', v); },
  setHeight: function (n, v) { n.setAttribute('height', v); },
  setX: function (n, v) { n.setAttribute('x', v); },
  setY: function (n, v) { n.setAttribute('y', v); }
};

function pe_style(node, name, value) {
  if (!node || !node.style) return;
  try {
    if (value === null || value === undefined || value === '') node.style.removeProperty(name);
    else node.style.setProperty(name, String(value));
  } catch (e) { /* ignored */ }
}

var Dom = {
  resolver: function (prefix) { return PE_NS[prefix] || null; },
  parseDocument: function (xml) { return new DOMParser().parseFromString(String(xml), 'text/xml'); },
  parseToNode: function (xml, doc) {
    var d = Dom.parseDocument(xml);
    if (!d || !d.documentElement || d.getElementsByTagName('parsererror').length) return null;
    return (doc || document).importNode(d.documentElement, true);
  },
  getList: function (xpath, node) {
    var out = [];
    try {
      var doc = node.nodeType === 9 ? node : node.ownerDocument;
      var r = doc.evaluate(xpath, node, Dom.resolver, 7, null);
      for (var i = 0; i < r.snapshotLength; i++) out.push(r.snapshotItem(i));
    } catch (e) { /* ignored */ }
    return out;
  },
  getSingle: function (xpath, node) { return Dom.getList(xpath, node)[0] || null; },
  workOn: function (xpath, node, fn) { Dom.getList(xpath, node).forEach(function (n) { try { fn(n); } catch (e) { /* ignored */ } }); },
  empty: function (n) { while (n && n.firstChild) n.removeChild(n.firstChild); },
  newDOMElement: function (spec, doc) {
    doc = doc || (F._target && F._target.ownerDocument) || document;
    var e = spec._uri ? doc.createElementNS(spec._uri, spec._name) : doc.createElementNS(PE_NS.svg, spec._name);
    for (var k in spec) {
      if (k.charAt(0) === '_' || spec[k] === null || spec[k] === undefined) continue;
      try { e.setAttribute(k, spec[k]); } catch (er) { /* ignored */ }
    }
    if (spec._text) e.appendChild(doc.createTextNode(spec._text));
    if (spec._children && spec._children.length) e.appendChild(Dom.newDOMFragment(spec._children, doc));
    return e;
  },
  newDOMFragment: function (specs, doc) {
    doc = doc || (F._target && F._target.ownerDocument) || document;
    var f = doc.createDocumentFragment();
    (specs || []).forEach(function (s) { if (s) f.appendChild(Dom.newDOMElement(s, doc)); });
    return f;
  },
  serializeNode: function (n) { return new XMLSerializer().serializeToString(n); },
  appendAfter: function (n, ref) { ref.parentNode.insertBefore(n, ref.nextSibling); }
};

var F = {
  _target: null,
  getObjectBoundingBox: function (name) {
    var n = PE.findNamed(name);
    if (!n) return null;
    var b = pe_bbox(n);
    return { x: b.x, y: b.y, w: b.width, h: b.height, width: b.width, height: b.height };
  },
  textSize: function (text, font) {
    var f = font || new Font();
    var m = PE.measure(String(text && text.value !== undefined ? text.value : text), pe_fontCss(f.style, f.weight, f.getPixelHeight(), f.family));
    return { w: m, h: f.getPixelHeight() * 1.2, width: m, height: f.getPixelHeight() * 1.2 };
  },
  richTextSize: function (html, font, width) {
    var f = font || new Font();
    var lay = PE.layoutRuns(PE.htmlToBlocks(String(html && html.value !== undefined ? html.value : html)),
      { family: f.family, size: f.getPixelHeight(), weight: f.weight, style: f.style, lh: f.lineHeight }, width || 100000);
    return { w: lay.w, h: lay.h, width: lay.w, height: lay.h };
  },
  newDOMElement: function (spec) { return Dom.newDOMElement(spec); },
  newDOMFragment: function (specs) { return Dom.newDOMFragment(specs); },
  lineLength: function (p1, p2) { return Math.sqrt((p1.x - p2.x) * (p1.x - p2.x) + (p1.y - p2.y) * (p1.y - p2.y)); }
};
var functions = F;

var Group = {
  calculateLayout: function (pos0, size0, groupSize0, posPolicy, sizePolicy, newGroupSize, size) {
    var out = {};
    var rest = groupSize0 - pos0 - size0;
    if (sizePolicy === 'relative') out.size = Math.round(size0 * newGroupSize / groupSize0);
    else if (sizePolicy === 'start-end') out.size = newGroupSize - rest - pos0;
    else out.size = size;
    if (posPolicy === 'start') out.pos = pos0;
    else if (posPolicy === 'middle') out.pos = Math.round((newGroupSize - out.size) / 2);
    else if (posPolicy === 'relative') out.pos = Math.round(pos0 * newGroupSize / groupSize0);
    else out.pos = newGroupSize - rest - out.size;
    return out;
  }
};

function pe_bbox(n) {
  try {
    if (n.style && n.style.display === 'none') return { x: 0, y: 0, width: 0, height: 0 };
    var b = n.getBBox();
    return { x: b.x, y: b.y, width: b.width, height: b.height };
  } catch (e) {
    return { x: 0, y: 0, width: 0, height: 0 };
  }
}

function pe_fontCss(style, weight, size, family) {
  return (style || 'normal') + ' ' + (weight || 'normal') + ' ' + (size || 12) + 'px ' + (family || 'sans-serif');
}

// ===== behaviours =============================================================

var PE_BEHAVIORS = {
  Attr: function (name, value, ns) {
    if (value !== null && value !== undefined) {
      if (ns) this.setAttributeNS(ns, name, value); else this.setAttribute(name, value);
    } else if (ns) this.removeAttributeNS(ns, name);
    else this.removeAttribute(name);
  },
  Box: function (d) {
    this.setAttribute('width', Math.max(0, d.w));
    this.setAttribute('height', Math.max(0, d.h));
  },
  Bound: function (b) {
    if (this.localName === 'g') {
      this.setAttribute('transform', translate(b.x, b.y));
      return;
    }
    this.setAttribute('x', b.x);
    this.setAttribute('y', b.y);
    this.setAttribute('width', Math.max(0, b.w));
    this.setAttribute('height', Math.max(0, b.h));
  },
  Radius: function (rx, ry) {
    this.setAttribute('rx', rx);
    this.setAttribute('ry', ry === undefined ? rx : ry);
  },
  Fill: function (color) {
    if (!color) return;
    pe_style(this, 'fill', color.toRGBString());
    pe_style(this, 'fill-opacity', color.a);
  },
  StrokeColor: function (color) {
    if (!color) return;
    pe_style(this, 'stroke', color.toRGBString());
    pe_style(this, 'stroke-opacity', color.a);
  },
  StrokeStyle: function (st) {
    if (!st) return;
    pe_style(this, 'stroke-width', st.w);
    pe_style(this, 'stroke-dasharray', st.array ? st.array : null);
  },
  Color: function (color) {
    if (this.localName === 'text') {
      if (!color) return;
      pe_style(this, 'fill', color.toRGBString());
      pe_style(this, 'fill-opacity', color.a);
    } else {
      pe_style(this, 'color', color ? color.toRGBAString() : null);
    }
  },
  Font: function (f) {
    if (!f) return;
    pe_style(this, 'font-family', f.family);
    pe_style(this, 'font-size', f.size);
    pe_style(this, 'font-weight', f.weight);
    pe_style(this, 'font-style', f.style);
    pe_style(this, 'text-decoration', f.decor && f.decor !== 'none' ? f.decor : null);
    this.__peFont = f;
  },
  Visibility: function (v) {
    var on = v && v.constructor === Bool ? v.value : !!v;
    pe_style(this, 'visibility', on ? 'visible' : 'hidden');
    pe_style(this, 'display', on ? null : 'none');
  },
  CustomStyle: function (name, value) { pe_style(this, name, value); },
  ApplyFilter: function (v) {
    var on = v && v.constructor === Bool ? v.value : !!v;
    var f = this.getAttributeNS(PE_NS.p, 'filter');
    if (on && f) this.setAttribute('filter', f);
    else this.removeAttribute('filter');
  },
  Transform: function (t) {
    if (t === null || t === undefined) return;
    this.setAttribute('transform', t.join ? t.join(' ') : String(t));
  },
  Scale: function (x, y) { this.setAttribute('transform', scale(x, y)); },
  D: function (d) {
    if (d === null || d === undefined) return;
    this.setAttribute('d', d.join ? d.join(' ') : String(d));
  },
  EllipseFit: function (box) {
    this.setAttribute('cx', box.w / 2); this.setAttribute('cy', box.h / 2);
    this.setAttribute('rx', box.w / 2); this.setAttribute('ry', box.h / 2);
  },
  InnerText: function (t) {
    Dom.empty(this);
    this.appendChild(this.ownerDocument.createTextNode(t && t.value !== undefined ? t.value : String(t)));
  },
  DomContent: function (x) {
    Dom.empty(this);
    if (x === null || x === undefined) return;
    var node = null;
    if (x.nodeType) node = x;
    else node = Dom.parseToNode(x.value !== undefined ? x.value : String(x), this.ownerDocument);
    if (!node) return;
    if (node.ownerDocument !== this.ownerDocument) node = this.ownerDocument.importNode(node, true);
    this.appendChild(node);
  },
  Image: function (img) {
    if (!img) return;
    var url = img.w > 0 ? (PE.resolveUrl(img.data) || '') : '';
    if (url) {
      this.setAttribute('href', url);
      this.setAttributeNS(PE_NS.xlink, 'xlink:href', url);
    } else {
      this.removeAttribute('href');
      this.removeAttributeNS(PE_NS.xlink, 'href');
    }
    this.setAttribute('width', img.w);
    this.setAttribute('height', img.h);
  },
  BoxFit: function (bound, align) {
    if (this.localName !== 'text' || !bound) return;
    var b = pe_bbox(this);
    align = align || new Alignment(0, 0);
    var dx = Math.round((bound.w - b.width) * align.h / 2 + bound.x - b.x);
    var dy = Math.round((bound.h - b.height) * align.v / 2 + bound.y - b.y);
    this.setAttribute('transform', translate(dx, dy));
  },
  TextContent: function (text) {
    var v = text && text.value !== undefined ? text.value : String(text === null || text === undefined ? '' : text);
    var rich = text && text.constructor === RichText;
    if (this.localName === 'text' && !rich) {
      Dom.empty(this);
      this.appendChild(this.ownerDocument.createTextNode(v));
      return;
    }
    PE.layoutInto(this, rich ? PE.htmlToBlocks(v) : PE.plainToBlocks(v), new Bound(0, 0, 100000, 0), new Alignment(0, 0), true);
  },
  PlainTextContent: function (text, bound, align) {
    var v = text && text.value !== undefined ? text.value : String(text === null || text === undefined ? '' : text);
    PE.layoutInto(this, PE.plainToBlocks(v), bound || new Bound(0, 0, 100000, 0), align || new Alignment(0, 0), false);
  },
  RichTextContent: function (text, bound, align) {
    var v = text && text.value !== undefined ? text.value : String(text === null || text === undefined ? '' : text);
    PE.layoutInto(this, PE.htmlToBlocks(v), bound || new Bound(0, 0, 100000, 0), align || new Alignment(0, 0), false);
  },
  NPatchDomContentFromImage: function (img, dim) {
    Dom.empty(this);
    if (!img || !(img.w > 0) || !dim) return;
    var url = PE.resolveUrl(img.data);
    if (!url) return;
    var doc = this.ownerDocument;
    var xs = PE.patchSegments(img.xCells, img.w, dim.w);
    var ys = PE.patchSegments(img.yCells, img.h, dim.h);
    var self = this;
    ys.forEach(function (sy) {
      xs.forEach(function (sx) {
        if (sx.size <= 0 || sy.size <= 0 || sx.scaled <= 0 || sy.scaled <= 0) return;
        var view = doc.createElementNS(PE_NS.svg, 'svg');
        view.setAttribute('x', pe_round(sx.pos));
        view.setAttribute('y', pe_round(sy.pos));
        view.setAttribute('width', pe_round(sx.scaled));
        view.setAttribute('height', pe_round(sy.scaled));
        view.setAttribute('viewBox', [sx.start, sy.start, sx.size, sy.size].join(' '));
        view.setAttribute('preserveAspectRatio', 'none');
        view.setAttribute('overflow', 'hidden');
        var im = doc.createElementNS(PE_NS.svg, 'image');
        im.setAttribute('width', img.w);
        im.setAttribute('height', img.h);
        im.setAttribute('href', url);
        im.setAttributeNS(PE_NS.xlink, 'xlink:href', url);
        im.setAttribute('preserveAspectRatio', 'none');
        view.appendChild(im);
        self.appendChild(view);
      });
    });
  }
};
PE_BEHAVIORS.NPatchDomContent = PE_BEHAVIORS.NPatchDomContentFromImage;
Pencil.behaviors = PE_BEHAVIORS;

// ===== the engine =============================================================

var PE = {
  packages: {},
  current: null,      // package being worked on
  root: null,         // root <g> of the shape being rendered
  host: null,
  measureCtx: null,

  // ----- files and resources -----------------------------------------------------

  file: function (pkg, path) {
    if (!pkg) return null;
    path = String(path || '').replace(/^\.?\//, '').split('?')[0];
    return Object.prototype.hasOwnProperty.call(pkg.files, path) ? pkg.files[path] : null;
  },

  /// The package and path an image reference points to.
  locate: function (ref) {
    ref = String(ref || '');
    var m = /^collection:\/\/@([^\/]+)\/(.*)$/.exec(ref) || /^@([^\/]+)\/(.*)$/.exec(ref);
    if (m) {
      for (var k in PE.packages) if (PE.packages[k].collection.id === m[1]) return { pkg: PE.packages[k], path: m[2] };
      return null;
    }
    m = /^collection:\/\/(.*)$/.exec(ref);
    if (m) return { pkg: PE.current, path: m[1] };
    return null;
  },

  isEmptyImage: function (data) {
    data = String(data || '');
    return !data || data === '-' || data === 'data:' || data === 'data://' || /^data:image\/svg\+xml[^,]*,(%3Csvg%3E%3C\/svg%3E|<svg><\/svg>)$/i.test(data);
  },

  /// A URL usable in <image href> (data URLs only).
  resolveUrl: function (data) {
    data = String(data || '');
    if (PE.isEmptyImage(data)) return '';
    if (/^data:[^,]+,/.test(data)) {
      if (/^data:image\/svg\+xml;utf8,/i.test(data)) return 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(data.replace(/^data:image\/svg\+xml;utf8,/i, ''));
      return data;
    }
    var at = PE.locate(data);
    if (!at) return '';
    var f = PE.file(at.pkg, at.path);
    if (f === null) return '';
    if (/^data:/.test(f)) {
      // Some packages keep SVG text in .png files.
      var text = PE.dataUrlText(f);
      if (text !== null && /^\s*</.test(text) && /<svg/i.test(text)) return 'data:image/svg+xml;base64,' + PE.b64(text);
      return f;
    }
    return 'data:image/svg+xml;base64,' + PE.b64(f);
  },

  /// The text of an SVG reference (null for bitmaps).
  resolveText: function (data) {
    data = String(data || '');
    if (PE.isEmptyImage(data)) return null;
    var m = /^data:image\/svg\+xml(;[^,]*)?,(.*)$/i.exec(data);
    if (m) {
      var meta = m[1] || '';
      try { return /base64/i.test(meta) ? PE.unb64(m[2]) : decodeURIComponent(m[2]); } catch (e) { return m[2]; }
    }
    if (/^json:/.test(data)) return null;
    var at = PE.locate(data);
    if (!at) return null;
    var f = PE.file(at.pkg, at.path);
    if (f === null) return null;
    if (/^data:/.test(f)) {
      var t = PE.dataUrlText(f);
      return t !== null && /<svg/i.test(t) ? t : null;
    }
    return /<svg/i.test(f) ? f : null;
  },

  dataUrlText: function (url) {
    var m = /^data:[^,]*?(;base64)?,(.*)$/.exec(url);
    if (!m) return null;
    try { return m[1] ? PE.unb64(m[2]) : decodeURIComponent(m[2]); } catch (e) { return null; }
  },
  b64: function (text) { return btoa(unescape(encodeURIComponent(text))); },
  unb64: function (b) { return decodeURIComponent(escape(atob(b))); },

  // ----- parsing ----------------------------------------------------------------

  /// `$name` → `properties.name`, `$$name` → collection property value.
  rewrite: function (code) {
    return String(code)
      .replace(/\$\$([a-z][a-z0-9]*)/gi, function (m0, n) { return '(collection.properties.' + n + ' ? collection.properties.' + n + '.value : null)'; })
      .replace(/\$([a-z][a-z0-9]*)/gi, 'properties.$1');
  },

  children: function (el, name) {
    var out = [];
    for (var n = el && el.firstChild; n; n = n.nextSibling) if (n.nodeType === 1 && (!name || n.localName === name)) out.push(n);
    return out;
  },
  child: function (el, name) { return PE.children(el, name)[0] || null; },
  text: function (el) { return el ? el.textContent : ''; },

  /// A default: literal text, an `<E>` expression or `<Null/>`.
  initial: function (el) {
    var e = PE.child(el, 'E');
    if (e) return { kind: 'expr', code: PE.rewrite(PE.text(e)) };
    if (PE.child(el, 'Null')) return { kind: 'null' };
    return { kind: 'literal', text: PE.text(el) };
  },

  parseDefinition: function (xml) {
    var doc = new DOMParser().parseFromString(xml, 'text/xml');
    var root = doc.documentElement;
    if (!root || root.localName !== 'Shapes') throw new Error('Not a Pencil collection');
    var col = {
      id: root.getAttribute('id') || '',
      displayName: root.getAttribute('displayName') || '',
      description: root.getAttribute('description') || '',
      author: root.getAttribute('author') || '',
      url: 'collection://',
      installDirPath: '',
      properties: {},
      propertyGroups: [],
      fonts: []
    };
    var def = { collection: col, shapes: {}, shortcuts: {}, scripts: [], fonts: [] };
    PE.children(root).forEach(function (el) {
      var n = el.localName;
      if (n === 'Script') def.scripts.push(PE.text(el));
      else if (n === 'Properties') {
        PE.children(el, 'PropertyGroup').forEach(function (g) {
          var group = { name: g.getAttribute('name') || '', properties: [] };
          PE.children(g, 'Property').forEach(function (p) {
            var type = p.getAttribute('type');
            var T = PE_TYPES[type];
            var value = T ? T.fromString(PE.text(p)) : PE.text(p);
            var prop = { name: p.getAttribute('name'), displayName: p.getAttribute('displayName') || p.getAttribute('name'), type: type, value: value };
            col.properties[prop.name] = prop;
            group.properties.push(prop);
          });
          col.propertyGroups.push(group);
        });
      } else if (n === 'Fonts') {
        PE.children(el, 'Font').forEach(function (f) {
          var attrs = {};
          for (var i = 0; i < f.attributes.length; i++) attrs[f.attributes[i].name] = f.attributes[i].value;
          def.fonts.push(attrs);
        });
      } else if (n === 'Shape') {
        var sh = PE.parseShape(el);
        def.shapes[sh.id] = sh;
      } else if (n === 'Shortcut') {
        var sc = {
          id: el.getAttribute('id') || ('sc:' + (el.getAttribute('displayName') || '')),
          displayName: el.getAttribute('displayName') || '',
          to: (el.getAttribute('to') || '').replace(/^[^:]*:/, function (m0) { return m0.indexOf(col.id + ':') === 0 ? '' : m0; }),
          values: {}
        };
        PE.children(el, 'PropertyValue').forEach(function (pv) { sc.values[pv.getAttribute('name')] = PE.initial(pv); });
        def.shortcuts[sc.id] = sc;
      }
    });
    return def;
  },

  parseShape: function (el) {
    var sh = { id: el.getAttribute('id'), displayName: el.getAttribute('displayName') || '', props: [], propMap: {}, behaviors: [], actions: [], content: null };
    PE.children(el).forEach(function (part) {
      var n = part.localName;
      if (n === 'Properties') {
        PE.children(part, 'PropertyGroup').forEach(function (g) {
          PE.children(g, 'Property').forEach(function (p) {
            var meta = {};
            for (var i = 0; i < p.attributes.length; i++) {
              var at = p.attributes[i];
              if (at.namespaceURI === PE_NS.p || at.name.indexOf('p:') === 0) meta[at.localName] = PE.rewrite(at.value);
            }
            var prop = {
              name: p.getAttribute('name'), displayName: p.getAttribute('displayName') || p.getAttribute('name'),
              type: p.getAttribute('type'), group: g.getAttribute('name') || '', initial: PE.initial(p), meta: meta
            };
            sh.props.push(prop);
            sh.propMap[prop.name] = prop;
          });
        });
      } else if (n === 'Behaviors') {
        PE.children(part, 'For').forEach(function (f) {
          var items = PE.children(f).map(function (b) {
            var args = PE.children(b, 'Arg');
            var list = args.length ? args.map(function (a2) {
              return a2.hasAttribute('literal') ? { literal: a2.getAttribute('literal') } : { code: PE.rewrite(PE.text(a2)) };
            }) : [{ code: PE.rewrite(PE.text(b)) }];
            return { name: b.localName, args: list };
          });
          sh.behaviors.push({ refs: (f.getAttribute('ref') || '').split(',').map(function (x) { return x.trim(); }).filter(Boolean), items: items });
        });
      } else if (n === 'Actions') {
        PE.children(part, 'Action').forEach(function (a2) {
          var impl = PE.child(a2, 'Impl');
          sh.actions.push({
            id: a2.getAttribute('id'), label: a2.getAttribute('displayName') || a2.getAttribute('id'),
            disabled: PE.rewrite(a2.getAttributeNS(PE_NS.p, 'disabled') || a2.getAttribute('p:disabled') || ''),
            content: !!(a2.getAttributeNS(PE_NS.p, 'content-action') || a2.getAttribute('p:content-action')),
            code: impl ? PE.text(impl) : ''
          });
        });
      } else if (n === 'Content') {
        sh.content = part;
      }
    });
    return sh;
  },

  // ----- fonts --------------------------------------------------------------------

  FONT_VARIANTS: { thin: 100, ulight: 200, light: 300, regular: 400, medium: 500, sbold: 600, bold: 700, xbold: 800, black: 900 },

  registerFonts: function (pkg) {
    var css = [], loads = [];
    pkg.def.fonts.forEach(function (f) {
      var family = f.name;
      if (!family) return;
      Object.keys(f).forEach(function (key) {
        var italic = /Italic$/.test(key);
        var base = key.replace(/Italic$/, '');
        var weight = PE.FONT_VARIANTS[base];
        if (!weight) return;
        var url = PE.file(pkg, f[key]);
        if (!url) return;
        if (!/^data:/.test(url)) url = 'data:font/ttf;base64,' + PE.b64(url);
        var style = italic ? 'italic' : 'normal';
        css.push("@font-face { font-family: '" + family.replace(/'/g, '') + "'; src: url(" + url + '); font-weight: ' + weight + '; font-style: ' + style + '; }');
        if (typeof FontFace !== 'undefined' && document.fonts) {
          try {
            var face = new FontFace(family, 'url(' + url + ')', { weight: String(weight), style: style });
            document.fonts.add(face);
            loads.push(face.load().catch(function () { return null; }));
          } catch (e) { /* ignored */ }
        }
      });
    });
    pkg.fontCss = css.join('\n');
    return Promise.all(loads);
  },

  // ----- values ---------------------------------------------------------------------

  /// Brings a value into the type of a property.
  coerce: function (type, v) {
    var T = PE_TYPES[type];
    if (v === null || v === undefined) return null;
    if (!T) return v;
    if (v instanceof T) return v;
    if (type === 'Bool' && typeof v === 'boolean') return new Bool(v);
    if (type === 'Num' && typeof v === 'number') return new Num(v);
    if (type === 'Enum' && typeof v === 'object' && 'value' in v) return new Enum(v.value);
    if ((type === 'PlainText' || type === 'RichText') && typeof v === 'object' && 'value' in v) return new T(v.value);
    if (typeof v === 'object' && v.constructor && v.constructor !== Object && typeof v.toString === 'function') {
      try { return T.fromString(v.toString()); } catch (e) { return null; }
    }
    if (typeof v === 'object' && type === 'Handle' && 'x' in v) return new Handle(v.x, v.y);
    if (typeof v === 'object' && type === 'Dimension' && 'w' in v) return new Dimension(v.w, v.h);
    if (typeof v === 'object' && type === 'Bound' && 'w' in v) return new Bound(v.x, v.y, v.w, v.h);
    return T.fromString(String(v));
  },

  evaluate: function (code, scope) {
    try { return __peEval(scope, code); } catch (e) { PE.fail(e, code); return null; }
  },

  /// Counts an error of a stencil (the last ones are kept for diagnosis).
  fail: function (e, code) {
    PE.errors++;
    PE.errorLog.push(String(e && e.message || e) + (code ? ' in: ' + String(code).trim().slice(0, 120) : ''));
    if (PE.errorLog.length > 40) PE.errorLog.shift();
  },
  errorLog: [],

  scopeOf: function (inst) {
    return { properties: inst.props, collection: inst.pkg.collection, functions: F };
  },

  /// Shape (and shortcut) of a palette entry.
  resolve: function (pkg, s) {
    var sc = pkg.def.shortcuts[s];
    if (sc) {
      var shape = pkg.def.shapes[sc.to];
      if (!shape) {
        var key = sc.to.indexOf(':') >= 0 ? sc.to.split(':').pop() : sc.to;
        shape = pkg.def.shapes[key];
      }
      return shape ? { shape: shape, shortcut: sc } : null;
    }
    var sh = pkg.def.shapes[s];
    return sh ? { shape: sh, shortcut: null } : null;
  },

  /// A shape instance: its typed property values.
  instance: function (pkg, entry, req) {
    var inst = { pkg: pkg, shape: entry.shape, shortcut: entry.shortcut, props: {} };
    var given = req.props && typeof req.props === 'object' && Object.keys(req.props).length ? req.props : null;
    var scope = PE.scopeOf(inst);
    entry.shape.props.forEach(function (p) {
      var v = null;
      if (given && p.name !== 'box' && Object.prototype.hasOwnProperty.call(given, p.name)) {
        v = PE.coerce(p.type, PE_TYPES[p.type] ? PE_TYPES[p.type].fromString(String(given[p.name])) : given[p.name]);
      } else {
        var init = entry.shortcut && entry.shortcut.values[p.name] ? entry.shortcut.values[p.name] : p.initial;
        if (init.kind === 'literal') v = PE_TYPES[p.type] ? PE_TYPES[p.type].fromString(init.text) : init.text;
        else if (init.kind === 'expr') v = PE.coerce(p.type, PE.evaluate(init.code, scope));
        if ((v === null || v === undefined) && init !== p.initial) {
          // A shortcut value that cannot be computed: the shape's own default.
          if (p.initial.kind === 'literal') v = PE_TYPES[p.type] ? PE_TYPES[p.type].fromString(p.initial.text) : p.initial.text;
          else if (p.initial.kind === 'expr') v = PE.coerce(p.type, PE.evaluate(p.initial.code, scope));
        }
      }
      if ((v === null || v === undefined) && PE_TYPES[p.type]) v = PE_TYPES[p.type].fromString('');
      inst.props[p.name] = v;
    });
    if (!inst.props.box) inst.props.box = new Dimension(100, 100);
    var boxMeta = (entry.shape.propMap.box || { meta: {} }).meta;
    var def = inst.props.box;
    var w = req.w > 0 ? +req.w : def.w, h = req.h > 0 ? +req.h : def.h;
    var oldW = req.oldW > 0 ? +req.oldW : null, oldH = req.oldH > 0 ? +req.oldH : null;
    var flag = function (k) { return boxMeta[k] ? !!PE.truthy(PE.evaluate(boxMeta[k], scope)) : false; };
    inst.boxInfo = { resizable: !flag('disabled'), lockW: flag('lockWidth'), lockH: flag('lockHeight'), lockRatio: flag('lockRatio') };
    var refW = oldW || def.w, refH = oldH || def.h;
    if (inst.boxInfo.lockW) w = refW;
    if (inst.boxInfo.lockH) h = refH;
    if (inst.boxInfo.lockRatio && refW > 0 && refH > 0) {
      if (Math.abs(w - refW) / refW >= Math.abs(h - refH) / refH) h = w * refH / refW;
      else w = h * refW / refH;
    }
    w = Math.max(2, w); h = Math.max(2, h);
    // Handles move with the box (unless they keep their distance).
    if (oldW && oldH && (oldW !== w || oldH !== h)) {
      entry.shape.props.forEach(function (p) {
        if (p.type !== 'Handle' || !inst.props[p.name]) return;
        if (p.meta.noScale && PE.truthy(PE.evaluate(p.meta.noScale, scope))) return;
        var hd = inst.props[p.name];
        inst.props[p.name] = new Handle(Math.round(hd.x * w / oldW), Math.round(hd.y * h / oldH));
      });
    }
    inst.props.box = new Dimension(Math.round(w * 100) / 100, Math.round(h * 100) / 100);
    PE.validateHandles(inst);
    return inst;
  },

  truthy: function (v) { return v && v.constructor === Bool ? v.value : !!v; },

  validateHandles: function (inst) {
    var scope = PE.scopeOf(inst);
    inst.shape.props.forEach(function (p) {
      var hd = inst.props[p.name];
      if (p.type !== 'Handle' || !hd) return;
      var num = function (k) {
        if (!p.meta[k]) return null;
        var v = PE.evaluate(p.meta[k], scope);
        v = v && typeof v === 'object' && 'value' in v ? v.value : v;
        return typeof v === 'number' && isFinite(v) ? v : (typeof v === 'string' && isFinite(parseFloat(v)) ? parseFloat(v) : null);
      };
      var h2 = new Handle(hd.x, hd.y);
      var ex = num('expressionX'), ey = num('expressionY');
      if (ex !== null) h2.x = ex;
      if (ey !== null) h2.y = ey;
      h2.applyMinX(num('minX')); h2.applyMaxX(num('maxX'));
      h2.applyMinY(num('minY')); h2.applyMaxY(num('maxY'));
      inst.props[p.name] = h2;
    });
  },

  // ----- drawing ---------------------------------------------------------------------

  ensureHost: function () {
    if (PE.host && PE.host.parentNode) return PE.host;
    var svg = document.createElementNS(PE_NS.svg, 'svg');
    svg.setAttribute('width', '4000');
    svg.setAttribute('height', '4000');
    svg.setAttribute('style', 'position:absolute;left:-20000px;top:0;');
    (document.body || document.documentElement).appendChild(svg);
    PE.host = svg;
    return svg;
  },

  findNamed: function (name) {
    if (!PE.root) return null;
    var list = PE.root.getElementsByTagName('*');
    for (var i = 0; i < list.length; i++) {
      if (list[i].getAttributeNS(PE_NS.p, 'name') === name) return list[i];
    }
    return null;
  },

  /// Copies the shape's SVG template into the measuring host.
  instantiate: function (inst) {
    var host = PE.ensureHost();
    var g = document.createElementNS(PE_NS.svg, 'g');
    var prefix = 'pe' + Math.random().toString(36).slice(2, 8) + '_';
    var names = {};
    if (inst.shape.content) {
      PE.children(inst.shape.content).forEach(function (n) { g.appendChild(document.importNode(n, true)); });
    }
    // Fixed ids per instance; the template's ids are names.
    var all = [g].concat(Array.prototype.slice.call(g.getElementsByTagName('*')));
    all.forEach(function (el) {
      var id = el.getAttribute('id');
      if (id) {
        names[id] = prefix + id;
        el.setAttributeNS(PE_NS.p, 'p:name', id);
        el.setAttribute('id', prefix + id);
      }
    });
    var fix = function (v) {
      return v.replace(/url\(\s*['"]?#([^'")\s]+)['"]?\s*\)/g, function (m0, n) { return names[n] ? 'url(#' + names[n] + ')' : m0; })
        .replace(/^#(.+)$/, function (m0, n) { return names[n] ? '#' + names[n] : m0; });
    };
    all.forEach(function (el) {
      for (var i = 0; i < el.attributes.length; i++) {
        var at = el.attributes[i];
        if (at.value.indexOf('#') >= 0 && at.localName !== 'id') {
          var nv = fix(at.value);
          if (nv !== at.value) at.value = nv;
        }
      }
    });
    // HTML containers become groups (nine-patch images, texts).
    Array.prototype.slice.call(g.getElementsByTagName('*')).forEach(function (el) {
      if (el.localName !== 'foreignObject') return;
      var ng = document.createElementNS(PE_NS.svg, 'g');
      for (var i = 0; i < el.attributes.length; i++) {
        var at = el.attributes[i];
        if (['x', 'y', 'width', 'height'].indexOf(at.localName) >= 0) continue;
        if (at.namespaceURI) ng.setAttributeNS(at.namespaceURI, at.name, at.value);
        else ng.setAttribute(at.name, at.value);
      }
      var x = pe_num(el.getAttribute('x')), y = pe_num(el.getAttribute('y'));
      if ((x || y) && !ng.getAttribute('transform')) ng.setAttribute('transform', translate(x, y));
      el.parentNode.replaceChild(ng, el);
    });
    host.appendChild(g);
    inst.root = g;
    PE.root = g;
    return g;
  },

  applyBehaviors: function (inst) {
    var scope = PE.scopeOf(inst);
    inst.shape.behaviors.forEach(function (fb) {
      fb.refs.forEach(function (ref) {
        var target = PE.findNamed(ref);
        if (!target) return;
        fb.items.forEach(function (item) {
          var fn = PE_BEHAVIORS[item.name];
          if (!fn) return;
          try {
            F._target = target;
            var args = item.args.map(function (a2) { return a2.literal !== undefined ? a2.literal : PE.evaluate(a2.code, scope); });
            F._target = target;
            fn.apply(target, args);
          } catch (e) {
            PE.fail(e, item.name + ' on ' + ref);
          }
        });
      });
    });
    F._target = inst.root;
  },

  /// Draws a shape; the instance keeps its live DOM until `finish`.
  draw: function (inst) {
    PE.current = inst.pkg;
    PE.instantiate(inst);
    F._target = inst.root;
    // Twice: later behaviours measure what earlier ones drew.
    PE.applyBehaviors(inst);
    PE.applyBehaviors(inst);
  },

  finish: function (inst) {
    if (inst.root && inst.root.parentNode) inst.root.parentNode.removeChild(inst.root);
    if (PE.root === inst.root) PE.root = null;
  },

  // ----- output -------------------------------------------------------------------------

  /// Serialises the drawing without Pencil metadata, scripts or external links.
  serialize: function (g) {
    var clone = g.cloneNode(true);
    var drop = [];
    var walk = function (el) {
      if (el.nodeType !== 1) return;
      var ln = el.localName.toLowerCase();
      if (el.namespaceURI !== PE_NS.svg || ln === 'script' || ln === 'foreignobject' || ln === 'iframe') { drop.push(el); return; }
      for (var i = el.attributes.length - 1; i >= 0; i--) {
        var at = el.attributes[i];
        var name = at.name.toLowerCase();
        var bad = at.namespaceURI === PE_NS.p || name.indexOf('p:') === 0 || name.indexOf('on') === 0 ||
          (at.localName === 'href' && !/^(data:|#)/i.test(at.value.trim())) ||
          (name === 'style' && /url\(\s*['"]?(?!#|data:)/i.test(at.value)) || name.indexOf('xmlns:p') === 0;
        if (bad) el.removeAttributeNode(at);
      }
      for (var c2 = el.firstChild; c2; c2 = c2.nextSibling) walk(c2);
    };
    walk(clone);
    drop.forEach(function (n) { if (n.parentNode) n.parentNode.removeChild(n); });
    var out = new XMLSerializer().serializeToString(clone);
    return out.replace(/\sxmlns:p="[^"]*"/g, '');
  },

  schema: function (inst) {
    var scope = PE.scopeOf(inst);
    return inst.shape.props.filter(function (p) { return p.name !== 'box'; }).map(function (p) {
      var v = inst.props[p.name];
      var out = {
        name: p.name, label: p.displayName, group: p.group, type: p.type,
        value: v === null || v === undefined ? '' : String(v),
        disabled: p.meta.disabled ? !!PE.truthy(PE.evaluate(p.meta.disabled, scope)) : false
      };
      if (p.meta.enumValues) {
        var list = PE.evaluate(p.meta.enumValues, scope);
        if (list && list.length) {
          out.enumValues = list.map(function (e) {
            var s2 = String(e), i = s2.indexOf('|');
            return i >= 0 ? [s2.slice(0, i), s2.slice(i + 1)] : [s2, s2];
          });
        }
      }
      return out;
    });
  },

  edits: function (inst) {
    var scope = PE.scopeOf(inst);
    var out = [];
    var styleOf = function (target) {
      if (!target) return { font: null, color: null };
      var cs = null;
      try { cs = window.getComputedStyle ? window.getComputedStyle(target) : null; } catch (e) { cs = null; }
      var pick = function (n) { return (target.style && target.style.getPropertyValue(n)) || (cs ? cs.getPropertyValue(n) : ''); };
      return { color: pick('fill') && pick('fill') !== 'none' ? pick('fill') : pick('color') };
    };
    inst.shape.props.forEach(function (p) {
      if (p.type !== 'PlainText' && p.type !== 'RichText') return;
      var info = null;
      if (p.meta.editInfo) {
        F._target = inst.root;
        info = PE.evaluate(p.meta.editInfo, scope);
      }
      if (!info) {
        // From a text behaviour that shows this property.
        inst.shape.behaviors.forEach(function (fb) {
          fb.items.forEach(function (item) {
            if (info || ['TextContent', 'PlainTextContent', 'RichTextContent'].indexOf(item.name) < 0) return;
            var first = item.args[0] && item.args[0].code ? item.args[0].code.trim() : '';
            if (first !== 'properties.' + p.name) return;
            var font = null;
            fb.items.forEach(function (it2) { if (it2.name === 'Font' && it2.args[0].code) font = PE.evaluate(it2.args[0].code, scope); });
            info = {
              targetName: fb.refs[0],
              bound: item.args[1] && item.args[1].code ? PE.evaluate(item.args[1].code, scope) : null,
              align: item.args[2] && item.args[2].code ? PE.evaluate(item.args[2].code, scope) : null,
              font: font,
              richText: item.name === 'RichTextContent'
            };
          });
        });
      }
      if (!info) return;
      var target = info.targetName ? PE.findNamed(info.targetName) : null;
      var b = info.bound;
      if (!b && target) { var bb = pe_bbox(target); b = new Bound(bb.x, bb.y, bb.width, bb.height); }
      if (!b) b = Bound.fromBox(inst.props.box);
      var f = info.font && info.font.getPixelHeight ? info.font : (target && target.__peFont) || null;
      var al = info.align || new Alignment(0, 1);
      out.push({
        prop: p.name,
        x: pe_round(b.x), y: pe_round(b.y), w: pe_round(Math.min(b.w, inst.props.box.w * 4)), h: pe_round(b.h),
        font: f ? pe_fontCss(f.style, f.weight, f.getPixelHeight(), f.family) : null,
        color: styleOf(target).color || null,
        align: [al.h || 0, al.v === undefined ? 1 : al.v],
        rich: !!(info.richText || p.type === 'RichText'),
        multi: info.multi !== undefined ? !!info.multi : true
      });
    });
    F._target = inst.root;
    return out;
  },

  handles: function (inst) {
    var scope = PE.scopeOf(inst);
    return inst.shape.props.filter(function (p) { return p.type === 'Handle' && inst.props[p.name]; }).filter(function (p) {
      return !(p.meta.disabled && PE.truthy(PE.evaluate(p.meta.disabled, scope)));
    }).map(function (p) {
      var hd = inst.props[p.name];
      var num = function (k) {
        if (!p.meta[k]) return null;
        var v = PE.evaluate(p.meta[k], scope);
        return typeof v === 'number' && isFinite(v) ? v : null;
      };
      var flag = function (k) { return p.meta[k] ? !!PE.truthy(PE.evaluate(p.meta[k], scope)) : false; };
      return {
        prop: p.name, x: hd.x, y: hd.y, lockX: flag('lockX'), lockY: flag('lockY'),
        minX: num('minX'), maxX: num('maxX'), minY: num('minY'), maxY: num('maxY')
      };
    });
  },

  values: function (inst) {
    var out = {};
    inst.shape.props.forEach(function (p) {
      if (p.name === 'box') return;
      var v = inst.props[p.name];
      out[p.name] = v === null || v === undefined ? '' : String(v);
    });
    return out;
  },

  // ----- text ----------------------------------------------------------------------------

  measure: function (text, css) {
    if (PE.measureCtx === null) {
      try {
        var cv = document.createElement('canvas');
        PE.measureCtx = (cv.getContext && cv.getContext('2d')) || false;
      } catch (e) { PE.measureCtx = false; }
    }
    if (PE.measureCtx) {
      PE.measureCtx.font = css;
      return PE.measureCtx.measureText(text).width;
    }
    var m = /(\d+(?:\.\d+)?)px/.exec(css);
    return text.length * (m ? parseFloat(m[1]) : 12) * 0.55;
  },

  metrics: function (css, size) {
    if (PE.measureCtx) {
      PE.measureCtx.font = css;
      var m = PE.measureCtx.measureText('Hg');
      if (m.fontBoundingBoxAscent) return { asc: m.fontBoundingBoxAscent, desc: m.fontBoundingBoxDescent };
    }
    return { asc: size * 0.8, desc: size * 0.2 };
  },

  plainToBlocks: function (text) {
    // A final line break adds no empty line.
    return String(text).replace(/\r?\n$/, '').split(/\r?\n/).map(function (line) {
      return { runs: [{ text: line, st: {} }], indent: 0, prefix: '' };
    });
  },

  /// HTML (rich text) as paragraphs of styled runs.
  htmlToBlocks: function (html) {
    html = String(html || '');
    if (html.indexOf('<') < 0 && html.indexOf('&') < 0) return PE.plainToBlocks(html);
    var doc = new DOMParser().parseFromString('<!DOCTYPE html><body>' + html + '</body>', 'text/html');
    var blocks = [];
    var cur = null;
    // Blocks start when content arrives (as in HTML, empty elements and a
    // trailing <br> add no line); `next` is the indent/bullet of the next one.
    var next = { indent: 0, prefix: '' };
    var start = function () {
      cur = { runs: [], indent: next.indent, prefix: next.prefix };
      next = { indent: next.indent, prefix: '' };
      blocks.push(cur);
    };
    var BLOCK = /^(p|div|h[1-6]|li|ul|ol|blockquote|pre|table|tr|section|article|header|footer)$/;
    var walk = function (node, st, depth, list) {
      for (var n = node.firstChild; n; n = n.nextSibling) {
        if (n.nodeType === 3) {
          var t = st.pre ? n.nodeValue : n.nodeValue.replace(/\s+/g, ' ');
          if (!cur || !cur.runs.length) t = st.pre ? t : t.replace(/^ /, '');
          if (!t) continue;
          if (!cur) start();
          cur.runs.push({ text: t, st: st });
          continue;
        }
        if (n.nodeType !== 1) continue;
        var tag = n.localName.toLowerCase();
        if (tag === 'script' || tag === 'style') continue;
        if (tag === 'br') {
          // Ends the line; on an empty line it is a blank line of its own.
          if (!cur) start();
          cur = null;
          next = { indent: depth * 18, prefix: '' };
          continue;
        }
        var s2 = Object.assign({}, st);
        if (tag === 'b' || tag === 'strong') s2.weight = 'bold';
        if (tag === 'i' || tag === 'em') s2.style = 'italic';
        if (tag === 'u') s2.decor = 'underline';
        if (tag === 's' || tag === 'strike' || tag === 'del') s2.decor = 'line-through';
        if (tag === 'pre' || tag === 'code') { s2.family = 'monospace'; if (tag === 'pre') s2.pre = true; }
        var hm = /^h([1-6])$/.exec(tag);
        if (hm) { s2.weight = 'bold'; s2.scale = [2, 1.5, 1.17, 1, 0.83, 0.67][+hm[1] - 1]; }
        if (tag === 'font' && n.getAttribute('color')) s2.color = n.getAttribute('color');
        var css = n.getAttribute('style');
        if (css) {
          css.split(';').forEach(function (d) {
            var i = d.indexOf(':');
            if (i < 0) return;
            var k = d.slice(0, i).trim().toLowerCase(), v = d.slice(i + 1).trim();
            if (k === 'color') s2.color = v;
            else if (k === 'font-weight') s2.weight = v;
            else if (k === 'font-style') s2.style = v;
            else if (k === 'text-decoration' || k === 'text-decoration-line') s2.decor = v;
            else if (k === 'font-family') s2.family = v;
            else if (k === 'font-size') { var px = parseFloat(v); if (px) s2.size = /em$/.test(v) ? null : px; if (/em$/.test(v)) s2.scale = px; }
          });
        }
        if (BLOCK.test(tag)) {
          cur = null;
          if (tag === 'ul' || tag === 'ol') {
            walk(n, s2, depth + 1, { ordered: tag === 'ol', n: 0 });
          } else {
            var prefix = '';
            if (tag === 'li' && list) { list.n++; prefix = list.ordered ? list.n + '. ' : '• '; }
            next = { indent: depth * 18, prefix: prefix };
            walk(n, s2, depth, list);
          }
          cur = null;
          next = { indent: depth * 18, prefix: '' };
          continue;
        }
        walk(n, s2, depth, list);
      }
    };
    walk(doc.body, {}, 0, null);
    return blocks.length ? blocks : [{ runs: [], indent: 0, prefix: '' }];
  },

  /// Breaks blocks into lines within `width`.
  layoutRuns: function (blocks, base, width) {
    var lines = [];
    var maxW = 0;
    var lhFactor = base.lh > 0 ? (base.lh > 4 ? base.lh / base.size : base.lh) : 1.2;
    blocks.forEach(function (b) {
      var line = { items: [], w: 0, indent: b.indent, size: base.size };
      var push = function () {
        while (line.items.length && /^\s+$/.test(line.items[line.items.length - 1].text)) {
          line.w -= line.items.pop().w;
        }
        lines.push(line);
        maxW = Math.max(maxW, line.w + line.indent);
        line = { items: [], w: 0, indent: b.indent, size: base.size };
      };
      var runs = b.runs.slice();
      if (b.prefix) runs.unshift({ text: b.prefix, st: runs[0] ? runs[0].st : {} });
      if (!runs.length) { push(); return; }
      runs.forEach(function (r) {
        var st = r.st || {};
        var size = st.size || (st.scale ? base.size * st.scale : base.size);
        var css = pe_fontCss(st.style || base.style, st.weight || base.weight, size, st.family || base.family);
        var words = r.text.split(/(\s+)/).filter(function (x) { return x !== ''; });
        words.forEach(function (wd) {
          var ww = PE.measure(wd, css);
          var space = /^\s+$/.test(wd);
          if (!space && line.items.length && line.indent + line.w + ww > width + 0.5) push();
          if (space && !line.items.length) return;
          line.items.push({ text: wd, w: ww, st: st, size: size, css: css });
          line.w += ww;
          line.size = Math.max(line.size, size);
        });
      });
      push();
    });
    var h = 0;
    lines.forEach(function (l) { l.lh = l.size * lhFactor; h += l.lh; });
    return { lines: lines, w: maxW, h: h };
  },

  /// Lays text out into `target` (a <text> or a group that gets one).
  layoutInto: function (target, blocks, bound, align, top) {
    var doc = target.ownerDocument;
    var text;
    if (target.localName === 'text') {
      text = target;
      Dom.empty(text);
      text.removeAttribute('transform');
    } else {
      Dom.empty(target);
      text = doc.createElementNS(PE_NS.svg, 'text');
      target.appendChild(text);
    }
    var cs = null;
    try { cs = window.getComputedStyle ? window.getComputedStyle(target) : null; } catch (e) { cs = null; }
    var pick = function (n, d) {
      var v = (target.style && target.style.getPropertyValue(n)) || (cs && cs.getPropertyValue(n)) || '';
      return v || d;
    };
    var font = target.__peFont;
    var base = {
      family: font ? font.family : pick('font-family', 'sans-serif'),
      size: font ? font.getPixelHeight() : (parseFloat(pick('font-size', '12')) || 12),
      weight: font ? font.weight : pick('font-weight', 'normal'),
      style: font ? font.style : pick('font-style', 'normal'),
      lh: font ? font.lineHeight : 0
    };
    var fill = target.style && (target.style.getPropertyValue('fill') || '');
    var color = target.style && (target.style.getPropertyValue('color') || '');
    if (!fill && color) text.setAttribute('fill', color);
    text.setAttributeNS('http://www.w3.org/XML/1998/namespace', 'xml:space', 'preserve');
    pe_style(text, 'white-space', 'pre');
    var width = bound.w > 0 ? bound.w : 100000;
    var lay = PE.layoutRuns(blocks, base, width);
    var y = bound.y;
    if (!top) {
      if (align.v === 1) y += (bound.h - lay.h) / 2;
      else if (align.v === 2) y += bound.h - lay.h;
    }
    lay.lines.forEach(function (line) {
      var m = PE.metrics(pe_fontCss(base.style, base.weight, line.size, base.family), line.size);
      var baseline = y + (line.lh - (m.asc + m.desc)) / 2 + m.asc;
      var x = bound.x + line.indent;
      var free = width - line.indent - line.w;
      if (bound.w > 0 && bound.w < 100000) {
        if (align.h === 1) x += free / 2;
        else if (align.h === 2) x += free;
      }
      var first = true;
      // Words of the same style go into one tspan.
      var merged = [];
      line.items.forEach(function (it) {
        var last = merged[merged.length - 1];
        if (last && last.st === it.st && last.size === it.size) last.text += it.text;
        else merged.push({ text: it.text, st: it.st, size: it.size });
      });
      merged.forEach(function (it) {
        var ts = doc.createElementNS(PE_NS.svg, 'tspan');
        if (first) { ts.setAttribute('x', pe_round(x)); first = false; }
        ts.setAttribute('y', pe_round(baseline));
        var st = it.st || {};
        if (st.weight) ts.setAttribute('font-weight', st.weight);
        if (st.style) ts.setAttribute('font-style', st.style);
        if (st.decor) ts.setAttribute('text-decoration', st.decor);
        if (st.family) ts.setAttribute('font-family', st.family);
        if (it.size !== base.size) ts.setAttribute('font-size', it.size);
        if (st.color) ts.setAttribute('fill', st.color);
        ts.appendChild(doc.createTextNode(it.text));
        text.appendChild(ts);
      });
      y += line.lh;
    });
  },

  /// Nine-patch segments along one axis.
  patchSegments: function (cells, size, newSize) {
    cells = (cells || []).filter(function (c2) { return c2.to > c2.from; }).sort(function (x, y) { return x.from - y.from; });
    if (!cells.length) return [{ start: 0, size: size, pos: 0, scaled: newSize }];
    var stretch = 0;
    cells.forEach(function (c2) { stretch += c2.to - c2.from; });
    var fixed = size - stretch;
    var r = stretch > 0 ? Math.max(0, newSize - fixed) / stretch : 0;
    var out = [], at = 0, pos = 0;
    // Boundaries on whole pixels: no hairline gaps between the slices.
    var add = function (start, sz, scaled) {
      var p0 = Math.round(pos), p1 = Math.round(pos + scaled);
      out.push({ start: start, size: sz, pos: p0, scaled: p1 - p0 });
      pos += scaled;
    };
    cells.forEach(function (c2) {
      if (c2.from > at) add(at, c2.from - at, c2.from - at);
      add(c2.from, c2.to - c2.from, (c2.to - c2.from) * r);
      at = c2.to;
    });
    if (at < size) add(at, size - at, size - at);
    return out;
  },

  // ----- API ---------------------------------------------------------------------------

  errors: 0,

  loadPackage: async function (json) {
    try {
      var req = JSON.parse(json);
      if (PE.packages[req.pkg]) {
        var p0 = PE.packages[req.pkg];
        return JSON.stringify({ ok: true, id: p0.collection.id, name: p0.collection.displayName, fontCss: p0.fontCss });
      }
      var files = req.files || {};
      var xml = files['Definition.xml'];
      if (typeof xml !== 'string') throw new Error('Definition.xml is missing');
      if (/^data:/.test(xml)) xml = PE.dataUrlText(xml);
      var def = PE.parseDefinition(xml);
      var pkg = { name: req.pkg, files: files, def: def, collection: def.collection, fontCss: '' };
      PE.packages[req.pkg] = pkg;
      PE.current = pkg;
      def.scripts.forEach(function (code) {
        try { (new Function('collection', code))(def.collection); } catch (e) { PE.fail(e, 'collection script'); }
      });
      await PE.registerFonts(pkg);
      return JSON.stringify({ ok: true, id: def.collection.id, name: def.collection.displayName, fontCss: pkg.fontCss });
    } catch (e) {
      return JSON.stringify({ ok: false, error: String(e && e.message || e) });
    }
  },

  prepare: function (req) {
    var pkg = PE.packages[req.pkg];
    if (!pkg) throw new Error('package not loaded');
    PE.current = pkg;
    var entry = PE.resolve(pkg, req.s);
    if (!entry) throw new Error('unknown shape ' + req.s);
    return PE.instance(pkg, entry, req);
  },

  output: function (inst) {
    var box = inst.props.box;
    return {
      ok: true,
      svg: PE.serialize(inst.root),
      w: box.w, h: box.h,
      props: PE.values(inst),
      schema: PE.schema(inst),
      edits: PE.edits(inst),
      handles: PE.handles(inst),
      box: inst.boxInfo,
      actions: inst.shape.actions.filter(function (a2) {
        if (a2.id === 'getSnappingGuide' || !a2.code) return false;
        // Actions that only open Pencil's dialogs or read the clipboard do nothing here.
        if (/browseResource|browseLucideIcon|CollectionResourceBrowserDialog|ImageData\.prompt|clipboard\.|copyClipboardImage|Dialog\(/.test(a2.code)) return false;
        if (a2.disabled && PE.truthy(PE.evaluate(a2.disabled, PE.scopeOf(inst)))) return false;
        return true;
      }).map(function (a2) { return { id: a2.id, label: a2.label }; })
    };
  },

  render: async function (json) {
    var inst = null;
    try {
      var req = JSON.parse(json);
      if (document.fonts && document.fonts.ready) { try { await document.fonts.ready; } catch (e) { /* ignored */ } }
      inst = PE.prepare(req);
      PE.draw(inst);
      return JSON.stringify(PE.output(inst));
    } catch (e) {
      return JSON.stringify({ ok: false, error: String(e && e.message || e) });
    } finally {
      if (inst) PE.finish(inst);
    }
  },

  action: async function (json) {
    var inst = null;
    try {
      var req = JSON.parse(json);
      inst = PE.prepare(req);
      PE.draw(inst);
      var act = null;
      inst.shape.actions.forEach(function (a2) { if (a2.id === req.action) act = a2; });
      if (!act) throw new Error('unknown action ' + req.action);
      var ctrl = PE.controller(inst);
      var fn = __peEval(PE.scopeOf(inst), '(function () {\n' + act.code + '\n})');
      var result = fn.call(ctrl);
      if (result && typeof result.then === 'function') { try { await result; } catch (e) { /* ignored */ } }
      return JSON.stringify({ ok: true, props: PE.values(inst), w: inst.props.box.w, h: inst.props.box.h });
    } catch (e) {
      return JSON.stringify({ ok: false, error: String(e && e.message || e) });
    } finally {
      if (inst) PE.finish(inst);
    }
  },

  /// What actions see as `this`.
  controller: function (inst) {
    var ctrl = {
      id: 'shape',
      def: { id: inst.shape.id, displayName: inst.shape.displayName, collection: inst.pkg.collection, propertyMap: inst.shape.propMap },
      svg: inst.root,
      getProperty: function (name) {
        var v = inst.props[name];
        var p = inst.shape.propMap[name];
        return v && p && PE_TYPES[p.type] ? PE_TYPES[p.type].fromString(String(v)) : v;
      },
      setProperty: function (name, value) {
        var p = inst.shape.propMap[name];
        if (!p) return;
        var v = PE.coerce(p.type, value);
        if (v === null) return;
        inst.props[name] = v;
        PE.validateHandles(inst);
        PE.finish(inst);
        PE.draw(inst);
        ctrl.svg = inst.root;
      },
      getPropertyGroups: function () { return []; },
      getGeometry: function () {
        var b = inst.props.box;
        return { dim: { w: b.w, h: b.h }, loc: { x: 0, y: 0 }, ctm: null };
      },
      setGeometry: function () {},
      getBoundingRect: function () { var b = inst.props.box; return { x: 0, y: 0, width: b.w, height: b.h }; },
      moveBy: function () {},
      scaleTo: function (w, h) { ctrl.setProperty('box', new Dimension(w, h)); },
      invalidateInboundConnections: function () {},
      invalidateOutboundConnections: function () {},
      getName: function () { return inst.shape.displayName; }
    };
    return ctrl;
  }
};
