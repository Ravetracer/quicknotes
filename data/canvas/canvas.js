// QuickNotes canvas: an infinite SVG board with drawings, shapes, text,
// sticky notes, emoji, stickers, connectors and images.
//
// The scene comes from Rust (`QC.update(scene)`); every finished operation
// sends the whole scene back (`{type: 'canvas', scene}`), where it becomes the
// note's content. Undo and redo go through the note's buffer. Images are
// assets of the note, loaded from `qn-canvas://<note id>/<name>`.
//
// UI strings (window.QC_STRINGS; key: English text):
//   title: Untitled canvas
//   hint: Pick a tool on the left – draw, write, add sticky notes or paste images.
//   select: Select
//   hand: Hand
//   pen: Pen
//   highlighter: Highlighter
//   eraser: Eraser
//   shapes: Shapes
//   rect: Rectangle
//   ellipse: Ellipse
//   diamond: Diamond
//   triangle: Triangle
//   line: Line
//   arrow: Arrow
//   connector: Connector
//   text: Text
//   sticky: Sticky note
//   emoji: Emoji and stickers
//   image: Image…
//   emojiTab: Emoji
//   stickers: Stickers
//   search: Search
//   noResults: Nothing found
//   catSmileys: Smileys
//   catPeople: People and gestures
//   catNature: Nature
//   catFood: Food and drink
//   catActivity: Activities and travel
//   catObjects: Objects
//   catSymbols: Symbols
//   color: Colour
//   fill: Fill
//   stickyColor: Note colour
//   auto: Automatic
//   none: None
//   custom: Custom colour
//   thin: Thin
//   medium: Medium
//   thick: Thick
//   solid: Solid
//   dashed: Dashed
//   dotted: Dotted
//   fontSize: Font size
//   bold: Bold
//   alignLeft: Align left
//   alignCenter: Centre
//   alignRight: Align right
//   rounded: Rounded corners
//   start: Start
//   end: End
//   dot: Dot
//   straight: Straight
//   elbow: Elbow
//   curve: Curved
//   opacity: Opacity
//   front: Bring to front
//   back: Send to back
//   group: Group
//   ungroup: Ungroup
//   lock: Lock
//   unlock: Unlock
//   delete: Delete
//   zoomIn: Zoom in
//   zoomOut: Zoom out
//   zoomReset: Zoom to 100 %
//   fit: Fit to content
//   snap: Snap to grid
//   frame: Frame
//   catBasic: Basic shapes
//   catFrames: Frames
//   alignLefts: Align left edges
//   alignCentres: Align centres
//   alignRights: Align right edges
//   alignTops: Align top edges
//   alignMiddles: Align middles
//   alignBottoms: Align bottom edges
//   distH: Distribute horizontally
//   distV: Distribute vertically
//   sameW: Same width
//   sameH: Same height
//   tCentral: Central topic
//   tIdea: Idea
//   tStart: Start
//   tStep: Step
//   tDecision: Decision?
//   tEnd: End
//   tYes: Yes
//   tNo: No
//   tTitle: Title
//   templates: Start with a template
//   align: Align and distribute
//   catFlow: Flowchart
//   catUi: Wireframe
//   catMobile: Mobile
//   catMisc: Shapes and symbols
//   fA4p: A4 portrait
//   fA4l: A4 landscape
//   fSlide: Slide 16:9
//   fDesktop: Desktop
//   fPhone: Phone
//   tplBlank: Blank
//   tplBrainstorm: Brainstorm
//   tplRetro: Retrospective
//   tWell: Went well
//   tImprove: To improve
//   tActions: Actions
//   tplFlow: Flowchart
//   tplWire: Wireframe
//   tplMobile: Mobile screen
//   tplSwot: SWOT analysis
//   tS: Strengths
//   tW: Weaknesses
//   tO: Opportunities
//   tT: Threats
//   tplWeek: Weekly plan
//   mon: Monday
//   tue: Tuesday
//   wed: Wednesday
//   thu: Thursday
//   fri: Friday
//   sat: Saturday
//   sun: Sunday
//   s_process: Process
//   s_terminator: Start / end
//   s_decision: Decision
//   s_data: Data
//   s_document: Document
//   s_multidoc: Documents
//   s_database: Database
//   s_predefined: Predefined process
//   s_manual: Manual input
//   s_preparation: Preparation
//   s_delay: Delay
//   s_offpage: Off-page reference
//   s_onpage: Connector point
//   s_annotation: Annotation
//   s_window: Window
//   s_browser: Browser
//   s_dialog: Dialog
//   s_button: Button
//   s_primary: Primary button
//   s_textfield: Text field
//   s_textarea: Text area
//   s_checkbox: Checkbox
//   s_radio: Radio button
//   s_toggle: Switch
//   s_dropdown: Dropdown
//   s_slider: Slider
//   s_progress: Progress bar
//   s_tabs: Tabs
//   s_list: List
//   s_table: Table
//   s_image: Image
//   s_avatar: Avatar
//   s_icon: Icon
//   s_heading: Heading
//   s_paragraph: Paragraph
//   s_link: Link
//   s_breadcrumb: Breadcrumb
//   s_pagination: Pagination
//   s_search: Search field
//   s_card: Card
//   s_navbar: Navigation bar
//   s_sidebar: Sidebar
//   s_tooltip: Tooltip
//   s_phone: Phone
//   s_tablet: Tablet
//   s_statusbar: Status bar
//   s_appbar: App bar
//   s_bottomnav: Bottom navigation
//   s_fab: Action button
//   s_listitem: List item
//   s_speech: Speech bubble
//   s_thought: Thought bubble
//   s_callout: Callout
//   s_cloud: Cloud
//   s_star: Star
//   s_heart: Heart
//   s_brace: Curly brace
//   s_banner: Banner
//   s_arrowR: Arrow right
//   s_arrowL: Arrow left
//   s_arrowU: Arrow up
//   s_arrowD: Arrow down
//   s_arrowLR: Double arrow
//   st_check: Check
//   st_cross: Cross
//   st_star: Star
//   st_heart: Heart
//   st_up: Thumbs up
//   st_down: Thumbs down
//   st_question: Question
//   st_bang: Exclamation
//   st_idea: Idea
//   st_flag: Flag
//   st_warning: Warning
//   st_smile: Smile
//   st_sad: Sad
//   st_fire: Fire
//   st_rocket: Rocket
//   st_plus1: Plus one
(function () {
  'use strict';

  var STR = window.QC_STRINGS || {};
  function S(k, en) { return STR[k] || en; }
  var PALETTE = window.QC_PALETTE || ['#62a0ea', '#57e389', '#f8e45c', '#ffa348', '#ed333b', '#c061cb', '#b5835a', '#9a9996', '#2ec27e'];
  var COLORS = ['#1e1e1e', '#77767b', '#ffffff'].concat(PALETTE);
  var STICKY = ['#fff3a3', '#ffc8dd', '#c8f7c5', '#c5e3ff', '#ffd8a8', '#e2d1ff'];
  var CID = window.QC_ID || '';
  var NS = 'http://www.w3.org/2000/svg';
  var FONT = 'Cantarell, "Noto Sans", system-ui, sans-serif';
  var EMOJI_FONT = '"Noto Color Emoji", "Apple Color Emoji", "Segoe UI Emoji", sans-serif';
  var STICKY_INK = '#1e1e1e';
  var GRID = 10, MIN = 8, ZMIN = 0.1, ZMAX = 8;
  var CLOSED = { rect: 1, ellipse: 1, diamond: 1, triangle: 1 };
  var TEXTUAL = { rect: 1, ellipse: 1, diamond: 1, triangle: 1, text: 1, sticky: 1, conn: 1 };
  var SHAPES = ['rect', 'ellipse', 'diamond', 'triangle'];
  var REMEMBER = { s: 1, f: 1, sw: 1, dash: 1, fs: 1, b: 1, ends: 1, k: 1, r: 1 };

  /// Items with text that can be edited.
  function hasText(it) {
    return !!TEXTUAL[it.t] || it.t === 'frame' || (it.t === 'stencil' && stDef(it).text !== false);
  }

  // ----- state -------------------------------------------------------------

  var scene = { canvas: 1, items: [] };
  var view = { x: 80, y: 60, z: 1 };
  var byId = {};
  var sel = [];
  var tool = 'select', shapeKind = 'rect';
  var drag = null, editing = null, ta = null, pending = null;
  var lastJSON = '', lastContent = '', loaded = false;
  var snapOn = true, spaceDown = false, lastPointer = null, hoverId = null, live = null;
  var ink = '#1f2328', canvasBg = '#ffffff';
  var cache = {};
  var toolStyle = { pen: {}, hl: { s: '#f8e45c' }, shape: {}, line: {}, arrow: {}, text: {}, sticky: {}, conn: {}, frame: {} };
  var TOOL_CAT = {
    pen: 'pen', hl: 'hl', rect: 'shape', ellipse: 'shape', diamond: 'shape', triangle: 'shape',
    line: 'line', arrow: 'arrow', text: 'text', sticky: 'sticky', conn: 'conn', frame: 'frame'
  };

  function post(msg) {
    if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.qn) {
      window.webkit.messageHandlers.qn.postMessage(JSON.stringify(msg));
    }
  }

  // ----- helpers -------------------------------------------------------------

  function svgEl(tag, attrs) {
    var e = document.createElementNS(NS, tag);
    if (attrs) for (var k in attrs) if (attrs[k] !== null && attrs[k] !== undefined) e.setAttribute(k, attrs[k]);
    return e;
  }

  function h(tag, attrs) {
    var el = document.createElement(tag);
    if (attrs) for (var k in attrs) if (attrs[k] !== null && attrs[k] !== undefined) el.setAttribute(k, attrs[k]);
    for (var i = 2; i < arguments.length; i++) {
      var c = arguments[i];
      if (c === null || c === undefined) continue;
      el.append(c instanceof Node ? c : document.createTextNode(String(c)));
    }
    return el;
  }

  function r1(n) { return Math.round(n * 10) / 10; }
  function clone(o) { return JSON.parse(JSON.stringify(o)); }
  function rid() {
    var id;
    do { id = Math.random().toString(36).slice(2, 8); } while (byId[id]);
    return id;
  }
  function dist(a, b) { return Math.hypot(a.x - b.x, a.y - b.y); }
  function rotP(x, y, cx, cy, deg) {
    if (!deg) return { x: x, y: y };
    var a = deg * Math.PI / 180, c = Math.cos(a), s = Math.sin(a), dx = x - cx, dy = y - cy;
    return { x: cx + dx * c - dy * s, y: cy + dx * s + dy * c };
  }
  function snap(v) { return Math.round(v / GRID) * GRID; }
  function snapP(p, e) { return snapOn && !(e && e.altKey) ? { x: snap(p.x), y: snap(p.y) } : p; }
  function assetUrl(src) { return 'qn-canvas://' + CID + '/' + src; }

  function swOf(it) { return it.sw || (it.t === 'path' ? (it.hl ? 20 : 3) : 2); }
  function fsOf(it) { return it.fs || { text: 20, sticky: 24, conn: 14, frame: 14 }[it.t] || (it.t === 'stencil' && stDef(it).fs) || 16; }
  function endsOf(it) { return it.ends || (it.t === 'line' ? ['none', 'none'] : ['none', 'arrow']); }
  function alOf(it) { return it.al || (it.t === 'text' || it.t === 'frame' ? 'l' : it.t === 'stencil' ? stDef(it).al || 'c' : 'c'); }
  function lineH(fs) { return fs * 1.25; }
  function dashArr(it, sw) {
    if (it.dash === 'dash') return (sw * 4) + ' ' + (sw * 3);
    if (it.dash === 'dot') return '0.01 ' + (sw * 2.5);
    return null;
  }

  var ICONS = {
    select: 'M6 3l12 8-5.5 1.5L10 18z',
    hand: 'M8 13V6a1.5 1.5 0 0 1 3 0v5M11 11V4.5a1.5 1.5 0 0 1 3 0V11M14 11V6a1.5 1.5 0 0 1 3 0v7c0 4-3 7-6.5 7S5 18 4 15l-1-3a1.5 1.5 0 0 1 2.6-1.4L8 13',
    pen: 'M4 20l4-1 11-11-3-3L5 16zM14 7l3 3',
    hl: 'M9 15l-3 3h5l1-1M9 15l8-8 3 3-8 8zM4 21h16',
    eraser: 'M8 20h12M5 15l9-9 5 5-9 9H9zM9 11l5 5',
    rect: 'M4 6h16v12H4z',
    ellipse: 'M12 6c4.4 0 8 2.7 8 6s-3.6 6-8 6-8-2.7-8-6 3.6-6 8-6z',
    diamond: 'M12 3l9 9-9 9-9-9z',
    triangle: 'M12 4l9 16H3z',
    line: 'M5 19L19 5',
    arrow: 'M5 19L19 5M10 5h9v9',
    conn: 'M3 16h5v5H3zM16 3h5v5h-5zM8 18c7 0 2-12 8-12',
    text: 'M5 7V4h14v3M12 4v16M9 20h6',
    sticky: 'M4 4h16v10l-6 6H4zM14 20v-6h6',
    emoji: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM8 14c1 2 2.5 3 4 3s3-1 4-3M9 9.5h.01M15 9.5h.01',
    image: 'M4 5h16v14H4zM4 16l5-5 4 4 2-2 5 5M15.5 9.5h.01',
    front: 'M9 9h11v11H9zM4 15V4h11',
    back: 'M4 4h11v11H4zM20 9v11H9',
    lock: 'M6 11h12v9H6zM8 11V8a4 4 0 0 1 8 0v3',
    unlock: 'M6 11h12v9H6zM8 11V8a4 4 0 0 1 7.5-2',
    trash: 'M5 7h14M10 7V4h4v3M7 7l1 13h8l1-13',
    group: 'M3 7V3h4M17 3h4v4M21 17v4h-4M7 21H3v-4M7 7h6v6H7zM11 11h6v6h-6z',
    ungroup: 'M4 4h7v7H4zM13 13h7v7h-7z',
    grid: 'M6 6h.01M12 6h.01M18 6h.01M6 12h.01M12 12h.01M18 12h.01M6 18h.01M12 18h.01M18 18h.01',
    fit: 'M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5',
    alignL: 'M4 6h16M4 10h10M4 14h16M4 18h10',
    alignC: 'M4 6h16M7 10h10M4 14h16M7 18h10',
    alignR: 'M4 6h16M10 10h10M4 14h16M10 18h10',
    rounded: 'M5 20V11a6 6 0 0 1 6-6h9',
    minus: 'M6 12h12',
    plus: 'M6 12h12M12 6v12',
    solid: 'M3 12h18',
    dashed: 'M3 12h4M10 12h4M17 12h4',
    dotted: 'M4 12h.01M8 12h.01M12 12h.01M16 12h.01M20 12h.01',
    frame: 'M7 3v18M17 3v18M3 7h18M3 17h18',
    align: 'M4 3v18M8 6h12v5H8zM8 14h7v5H8z',
    aL: 'M4 3v18M8 6h12v5H8zM8 14h7v5H8z',
    aC: 'M12 3v18M5 6h14v5H5zM8 14h8v5H8z',
    aR: 'M20 3v18M4 6h12v5H4zM9 14h7v5H9z',
    aT: 'M3 4h18M6 8h5v12H6zM14 8h5v7h-5z',
    aM: 'M3 12h18M6 5h5v14H6zM14 8h5v8h-5z',
    aB: 'M3 20h18M6 4h5v12H6zM14 9h5v7h-5z',
    dH: 'M3 4v16M21 4v16M8 7h3v10H8zM13 7h3v10h-3z',
    dV: 'M4 3h16M4 21h16M7 8h10v3H7zM7 13h10v3H7z',
    sW: 'M4 6h16v4H4zM4 14h16v4H4z',
    sH: 'M6 4h4v16H6zM14 4h4v16h-4z'
  };

  function icon(name, sw) {
    var s = sw || (name === 'grid' || name === 'dotted' ? 3 : 1.8);
    return '<svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="' + s +
      '" stroke-linecap="round" stroke-linejoin="round"><path d="' + ICONS[name] + '"/></svg>';
  }

  function ibtn(iconHtml, title, onClick, cls) {
    var b = h('button', { class: 'ibtn' + (cls ? ' ' + cls : ''), title: title, type: 'button' });
    b.innerHTML = iconHtml;
    b.addEventListener('click', function (e) { e.stopPropagation(); b.blur(); onClick(e, b); });
    b.addEventListener('pointerdown', function (e) { e.stopPropagation(); });
    return b;
  }

  // ----- stickers (original artwork, 100 × 100) -----------------------------

  var THUMB = '<path d="M27 47h11v29H27z" fill="#fff"/><path d="M42 47l10-23c2-5 10-4 9 3l-3 15h14c5 0 8 4 7 9l-4 19c-1 4-4 6-8 6H42z" fill="#fff"/>';
  var STICKERS = {
    check: '<circle cx="50" cy="50" r="46" fill="#2ec27e"/><path d="M28 52l16 16 30-34" fill="none" stroke="#fff" stroke-width="10" stroke-linecap="round" stroke-linejoin="round"/>',
    cross: '<circle cx="50" cy="50" r="46" fill="#e01b24"/><path d="M33 33l34 34M67 33L33 67" stroke="#fff" stroke-width="10" stroke-linecap="round"/>',
    star: '<path d="M50 6l12 31 33 1-26 20 9 33-28-19-28 19 9-33L5 38l33-1z" fill="#f6d32d" stroke="#e5a50a" stroke-width="4" stroke-linejoin="round"/>',
    heart: '<path d="M50 88C20 66 6 50 6 32 6 18 17 8 30 8c9 0 16 5 20 13 4-8 11-13 20-13 13 0 24 10 24 24 0 18-14 34-44 56z" fill="#ed333b"/><path d="M22 26c4-5 10-7 15-5" stroke="#fff" stroke-opacity=".6" stroke-width="5" fill="none" stroke-linecap="round"/>',
    up: '<circle cx="50" cy="50" r="46" fill="#3584e4"/>' + THUMB,
    down: '<circle cx="50" cy="50" r="46" fill="#ff7800"/><g transform="rotate(180 50 50)">' + THUMB + '</g>',
    question: '<circle cx="50" cy="50" r="46" fill="#9141ac"/><path d="M37 38c0-10 7-16 14-16 8 0 14 6 14 13 0 9-11 11-13 20v5" fill="none" stroke="#fff" stroke-width="9" stroke-linecap="round"/><circle cx="51" cy="75" r="6" fill="#fff"/>',
    bang: '<circle cx="50" cy="50" r="46" fill="#ff7800"/><path d="M50 22v36" stroke="#fff" stroke-width="11" stroke-linecap="round"/><circle cx="50" cy="75" r="6.5" fill="#fff"/>',
    idea: '<path d="M50 10c-20 0-32 15-32 32 0 13 8 20 14 27 3 3 4 6 4 10h28c0-4 1-7 4-10 6-7 14-14 14-27 0-17-12-32-32-32z" fill="#f8e45c" stroke="#e5a50a" stroke-width="3"/><rect x="36" y="82" width="28" height="6" rx="2" fill="#9a9996"/><rect x="39" y="90" width="22" height="6" rx="3" fill="#77767b"/><path d="M42 50l8 10 8-10" fill="none" stroke="#e5a50a" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>',
    flag: '<path d="M22 8v86" stroke="#5e5c64" stroke-width="6" stroke-linecap="round"/><path d="M25 12c15-8 30 8 45 0 6-3 12-2 16 0v42c-6-4-12-4-16 0-15 8-30-8-45 0z" fill="#e01b24"/>',
    warning: '<path d="M50 8c3 0 5 2 7 5l37 67c3 6 0 12-7 12H13c-7 0-10-6-7-12l37-67c2-3 4-5 7-5z" fill="#f6d32d" stroke="#1e1e1e" stroke-width="3"/><path d="M50 34v28" stroke="#1e1e1e" stroke-width="9" stroke-linecap="round"/><circle cx="50" cy="77" r="5.5" fill="#1e1e1e"/>',
    smile: '<circle cx="50" cy="50" r="46" fill="#f8e45c" stroke="#e5a50a" stroke-width="3"/><ellipse cx="35" cy="40" rx="5" ry="8" fill="#3d3846"/><ellipse cx="65" cy="40" rx="5" ry="8" fill="#3d3846"/><path d="M28 60c8 16 36 16 44 0" fill="none" stroke="#3d3846" stroke-width="6" stroke-linecap="round"/>',
    sad: '<circle cx="50" cy="50" r="46" fill="#f9f06b" stroke="#e5a50a" stroke-width="3"/><ellipse cx="35" cy="40" rx="5" ry="7" fill="#3d3846"/><ellipse cx="65" cy="40" rx="5" ry="7" fill="#3d3846"/><path d="M30 74c8-14 32-14 40 0" fill="none" stroke="#3d3846" stroke-width="6" stroke-linecap="round"/><path d="M68 50c0 0-7 9-3 13 3 3 8 0 6-6z" fill="#62a0ea"/>',
    fire: '<path d="M50 6c6 18 26 28 28 52 2 20-12 36-28 36S20 80 22 62c2-14 10-20 12-32 6 8 8 14 8 20 6-10 10-26 8-44z" fill="#ff7800"/><path d="M50 50c4 10 14 16 13 28-1 9-7 14-13 14-7 0-13-6-12-15 1-9 8-13 12-27z" fill="#f8e45c"/>',
    rocket: '<path d="M50 6c16 12 22 32 18 60H32C28 38 34 18 50 6z" fill="#deddda" stroke="#77767b" stroke-width="3"/><circle cx="50" cy="38" r="9" fill="#62a0ea" stroke="#77767b" stroke-width="3"/><path d="M32 52L18 70l14-2zM68 52l14 18-14-2z" fill="#e01b24"/><path d="M40 70c0 12 10 24 10 24s10-12 10-24z" fill="#ff7800"/>',
    plus1: '<rect x="6" y="20" width="88" height="60" rx="30" fill="#2ec27e"/><text x="50" y="63" text-anchor="middle" font-size="40" font-weight="700" fill="#fff" font-family="Cantarell, sans-serif">+1</text>'
  };

  var STICKER_NAMES = {
    check: 'Check', cross: 'Cross', star: 'Star', heart: 'Heart', up: 'Thumbs up', down: 'Thumbs down',
    question: 'Question', bang: 'Exclamation', idea: 'Idea', flag: 'Flag', warning: 'Warning', smile: 'Smile',
    sad: 'Sad', fire: 'Fire', rocket: 'Rocket', plus1: 'Plus one'
  };
  function stickerName(k) { return S('st_' + k, STICKER_NAMES[k] || k); }

  // ----- stencils (original artwork) -----------------------------------------
  //
  // A stencil draws itself into its box with a small kit (`k`). `k.body` is
  // the fill: the item's `f` or the stencil's default `bf`. Colours may be
  // 'ink', 'soft', 'faint', 'mid' (the theme's ink at low opacity, so they
  // work in light and dark), 'bg', 'accent' or a colour. Details keep their
  // pixel size; only the body stretches. `tb(w, h)` is the label box;
  // `text: 'own'` draws the text itself, `text: 'lines'` takes one entry
  // per line of `txt`, `text: false` has none. `ph` draws a placeholder bar
  // of that width (share of the box) while there is no text.

  var ACCENT = '#3584e4';
  var D = { main: true };
  var HEART_D = 'M50 90 C20 68 4 52 4 32 C4 17 16 7 30 7 C39 7 46 12 50 20 C54 12 61 7 70 7 C84 7 96 17 96 32 C96 52 80 68 50 90Z';

  function poly(a) {
    var s = '';
    for (var i = 0; i < a.length; i += 2) s += (i ? 'L' : 'M') + r1(a[i]) + ' ' + r1(a[i + 1]);
    return s + 'Z';
  }
  /// A rectangle with rounded top corners only.
  function rrTop(x, y, w, h, r) {
    return 'M' + r1(x) + ' ' + r1(y + h) + 'V' + r1(y + r) + 'a' + r + ' ' + r + ' 0 0 1 ' + r + ' -' + r + 'H' + r1(x + w - r) +
      'a' + r + ' ' + r + ' 0 0 1 ' + r + ' ' + r + 'V' + r1(y + h) + 'Z';
  }
  function docD(x, y, w, h) {
    var a = Math.min(12, h * 0.15);
    return 'M' + r1(x) + ' ' + r1(y) + 'H' + r1(x + w) + 'V' + r1(y + h - a) + 'C' + r1(x + w * 0.75) + ' ' + r1(y + h - a * 2.6) + ' ' +
      r1(x + w * 0.3) + ' ' + r1(y + h + a) + ' ' + r1(x) + ' ' + r1(y + h - a * 0.8) + 'Z';
  }
  function cloudD(x, y, w, h) {
    var cx = x + w / 2, cy = y + h / 2, rx = w / 2 * 0.74, ry = h / 2 * 0.7, n = 9, s = '';
    for (var i = 0; i <= n; i++) {
      var a = i / n * Math.PI * 2 - Math.PI / 2, px = cx + rx * Math.cos(a), py = cy + ry * Math.sin(a);
      if (!i) { s = 'M' + r1(px) + ' ' + r1(py); continue; }
      var m = a - Math.PI / n;
      s += 'Q' + r1(cx + rx * 1.36 * Math.cos(m)) + ' ' + r1(cy + ry * 1.36 * Math.sin(m)) + ' ' + r1(px) + ' ' + r1(py);
    }
    return s + 'Z';
  }
  function starD(x, y, w, h) {
    var pts = [];
    for (var i = 0; i < 10; i++) {
      var a = i / 10 * Math.PI * 2 - Math.PI / 2, f = i % 2 ? 0.42 : 1;
      pts.push(x + w / 2 + w / 2 * f * Math.cos(a), y + h * 0.53 + h * 0.53 * f * Math.sin(a));
    }
    return poly(pts);
  }
  function bubbleD(w, h, r) {
    var bh = h * 0.8;
    r = Math.min(r, bh / 2, w / 2);
    return 'M' + r1(r) + ' 0H' + r1(w - r) + 'A' + r1(r) + ' ' + r1(r) + ' 0 0 1 ' + r1(w) + ' ' + r1(r) + 'V' + r1(bh - r) +
      'A' + r1(r) + ' ' + r1(r) + ' 0 0 1 ' + r1(w - r) + ' ' + r1(bh) + 'H' + r1(w * 0.36) + 'L' + r1(w * 0.14) + ' ' + r1(h) +
      'L' + r1(w * 0.22) + ' ' + r1(bh) + 'H' + r1(r) + 'A' + r1(r) + ' ' + r1(r) + ' 0 0 1 0 ' + r1(bh - r) + 'V' + r1(r) +
      'A' + r1(r) + ' ' + r1(r) + ' 0 0 1 ' + r1(r) + ' 0Z';
  }
  /// Block arrow points: 'r', 'l', 'u', 'd' or 'lr'.
  function arrowPts(w, h, dir) {
    var vert = dir === 'u' || dir === 'd', W = vert ? h : w, H = vert ? w : h, pts, hl;
    if (dir === 'lr') {
      hl = Math.min(W * 0.3, H * 0.9);
      pts = [0, H / 2, hl, 0, hl, H * 0.25, W - hl, H * 0.25, W - hl, 0, W, H / 2, W - hl, H, W - hl, H * 0.75, hl, H * 0.75, hl, H];
    } else {
      hl = Math.min(W * 0.45, H * 0.9);
      pts = [0, H * 0.25, W - hl, H * 0.25, W - hl, 0, W, H / 2, W - hl, H, W - hl, H * 0.75, 0, H * 0.75];
    }
    var out = [];
    for (var i = 0; i < pts.length; i += 2) {
      var x = pts[i], y = pts[i + 1];
      if (dir === 'l') x = W - x;
      if (vert) { var t = x; x = y; y = t; if (dir === 'u') y = h - y; }
      out.push(x, y);
    }
    return out;
  }
  function arrowTB(w, h, dir) {
    if (dir === 'u' || dir === 'd') return { x: w * 0.27, y: h * 0.2, w: w * 0.46, h: h * 0.6 };
    var hl = dir === 'lr' ? Math.min(w * 0.3, h * 0.9) : Math.min(w * 0.45, h * 0.9);
    var x0 = dir === 'r' ? 4 : hl, x1 = dir === 'l' ? w - 4 : w - hl;
    return { x: x0, y: h * 0.25, w: Math.max(10, x1 - x0), h: h * 0.5 };
  }
  function blockArrow(dir, name) {
    return {
      c: 'misc', n: name, w: dir === 'u' || dir === 'd' ? 90 : 170, h: dir === 'u' || dir === 'd' ? 150 : 90,
      d: function (k) { k.p(poly(arrowPts(k.w, k.h, dir)), k.body, 'ink', D); },
      tb: function (w, h) { return arrowTB(w, h, dir); }
    };
  }
  function sk(w) { return Math.min(w * 0.2, 24); }
  function sideLabel(w, h) { return { x: 24, y: 0, w: Math.max(10, w - 24), h: h }; }

  var STENCILS = {
    // Flowchart
    process: { c: 'flow', n: 'Process', w: 160, h: 90, d: function (k) { k.r(0, 0, k.w, k.h, 0, k.body, 'ink', D); } },
    terminator: {
      c: 'flow', n: 'Start / end', w: 160, h: 64,
      d: function (k) { k.r(0, 0, k.w, k.h, k.h / 2, k.body, 'ink', D); },
      tb: function (w, h) { return { x: h / 3, y: 4, w: Math.max(10, w - h / 1.5), h: h - 8 }; }
    },
    decision: {
      c: 'flow', n: 'Decision', w: 170, h: 110,
      d: function (k) { k.p(poly([k.w / 2, 0, k.w, k.h / 2, k.w / 2, k.h, 0, k.h / 2]), k.body, 'ink', D); },
      tb: function (w, h) { return { x: w / 4, y: h / 4, w: w / 2, h: h / 2 }; }
    },
    data: {
      c: 'flow', n: 'Data', w: 170, h: 90,
      d: function (k) { var s = sk(k.w); k.p(poly([s, 0, k.w, 0, k.w - s, k.h, 0, k.h]), k.body, 'ink', D); },
      tb: function (w, h) { var s = sk(w); return { x: s, y: 4, w: Math.max(10, w - 2 * s), h: h - 8 }; }
    },
    document: {
      c: 'flow', n: 'Document', w: 160, h: 100,
      d: function (k) { k.p(docD(0, 0, k.w, k.h), k.body, 'ink', D); },
      tb: function (w, h) { return { x: 8, y: 4, w: w - 16, h: h - 8 - Math.min(12, h * 0.15) }; }
    },
    multidoc: {
      c: 'flow', n: 'Documents', w: 170, h: 110,
      d: function (k) {
        var o = 8, w = k.w - 2 * o, h = k.h - 2 * o, back = k.body === 'none' ? 'bg' : k.body;
        k.p(docD(2 * o, 0, w, h), back, 'ink', D);
        k.p(docD(o, o, w, h), back, 'ink', D);
        k.p(docD(0, 2 * o, w, h), back, 'ink', D);
      },
      tb: function (w, h) { return { x: 8, y: 20, w: w - 32, h: h - 28 - Math.min(12, (h - 16) * 0.15) }; }
    },
    database: {
      c: 'flow', n: 'Database', w: 120, h: 130,
      d: function (k) {
        var w = k.w, h = k.h, ry = Math.min(h * 0.15, 14), rx = r1(w / 2), a = 'A' + rx + ' ' + r1(ry) + ' 0 0 0 ';
        k.p('M0 ' + r1(ry) + 'V' + r1(h - ry) + a + r1(w) + ' ' + r1(h - ry) + 'V' + r1(ry) + a + '0 ' + r1(ry) + 'Z', k.body, 'ink', D);
        k.p('M0 ' + r1(ry) + a + r1(w) + ' ' + r1(ry), 'none', 'ink', D);
      },
      tb: function (w, h) { var ry = Math.min(h * 0.15, 14); return { x: 8, y: 2 * ry + 2, w: w - 16, h: Math.max(10, h - 3 * ry - 4) }; }
    },
    predefined: {
      c: 'flow', n: 'Predefined process', w: 170, h: 90,
      d: function (k) {
        k.r(0, 0, k.w, k.h, 0, k.body, 'ink', D);
        k.p('M10 0V' + r1(k.h) + 'M' + r1(k.w - 10) + ' 0V' + r1(k.h), 'none', 'ink');
      },
      tb: function (w, h) { return { x: 14, y: 4, w: Math.max(10, w - 28), h: h - 8 }; }
    },
    manual: {
      c: 'flow', n: 'Manual input', w: 160, h: 90,
      d: function (k) { k.p(poly([0, k.h * 0.28, k.w, 0, k.w, k.h, 0, k.h]), k.body, 'ink', D); },
      tb: function (w, h) { return { x: 8, y: h * 0.28, w: w - 16, h: h * 0.72 - 4 }; }
    },
    preparation: {
      c: 'flow', n: 'Preparation', w: 170, h: 80,
      d: function (k) {
        var s = Math.min(k.w * 0.2, k.h / 2, 30);
        k.p(poly([s, 0, k.w - s, 0, k.w, k.h / 2, k.w - s, k.h, s, k.h, 0, k.h / 2]), k.body, 'ink', D);
      },
      tb: function (w, h) { var s = Math.min(w * 0.2, h / 2, 30); return { x: s, y: 4, w: Math.max(10, w - 2 * s), h: h - 8 }; }
    },
    delay: {
      c: 'flow', n: 'Delay', w: 150, h: 80,
      d: function (k) {
        var r = Math.min(k.h / 2, k.w / 2);
        k.p('M0 0H' + r1(k.w - r) + 'A' + r1(r) + ' ' + r1(k.h / 2) + ' 0 0 1 ' + r1(k.w - r) + ' ' + r1(k.h) + 'H0Z', k.body, 'ink', D);
      },
      tb: function (w, h) { return { x: 8, y: 4, w: Math.max(10, w - Math.min(h / 2, w / 2) - 8), h: h - 8 }; }
    },
    offpage: {
      c: 'flow', n: 'Off-page reference', w: 100, h: 100,
      d: function (k) { k.p(poly([0, 0, k.w, 0, k.w, k.h * 0.68, k.w / 2, k.h, 0, k.h * 0.68]), k.body, 'ink', D); },
      tb: function (w, h) { return { x: 6, y: 4, w: w - 12, h: h * 0.68 - 4 }; }
    },
    onpage: {
      c: 'flow', n: 'Connector point', w: 60, h: 60,
      d: function (k) { k.e(k.w / 2, k.h / 2, k.w / 2, k.h / 2, k.body, 'ink', D); },
      tb: function (w, h) { return { x: w * 0.15, y: h * 0.15, w: w * 0.7, h: h * 0.7 }; }
    },
    annotation: {
      c: 'flow', n: 'Annotation', w: 160, h: 80, al: 'l',
      d: function (k) { var b = r1(Math.min(16, k.w * 0.3)); k.p('M' + b + ' 0H0V' + r1(k.h) + 'H' + b, 'none', 'ink', D); },
      tb: function (w, h) { return { x: 10, y: 4, w: Math.max(10, w - 14), h: h - 8 }; }
    },

    // Wireframes
    window: {
      c: 'ui', n: 'Window', w: 480, h: 320, fs: 13,
      d: function (k) {
        k.r(0, 0, k.w, k.h, 6, k.body, 'ink', D);
        k.p(rrTop(0, 0, k.w, 28, 6), 'soft', 'none');
        k.p('M0 28H' + r1(k.w), 'none', 'ink', { sw: 1 });
        [14, 28, 42].forEach(function (x) { k.e(x, 14, 4.5, 4.5, 'mid', 'none'); });
      },
      tb: function (w) { return { x: 54, y: 0, w: Math.max(10, w - 108), h: 28 }; }
    },
    browser: {
      c: 'ui', n: 'Browser', w: 640, h: 420, fs: 12, al: 'l', mut: true,
      d: function (k) {
        k.r(0, 0, k.w, k.h, 6, k.body, 'ink', D);
        k.p(rrTop(0, 0, k.w, 40, 6), 'soft', 'none');
        k.p('M0 40H' + r1(k.w), 'none', 'ink', { sw: 1 });
        [14, 28, 42].forEach(function (x) { k.e(x, 20, 4.5, 4.5, 'mid', 'none'); });
        k.r(60, 9, Math.max(20, k.w - 72), 22, 11, 'bg', 'mid', { sw: 1 });
      },
      tb: function (w) { return { x: 72, y: 9, w: Math.max(10, w - 96), h: 22 }; }
    },
    dialog: {
      c: 'ui', n: 'Dialog', w: 360, h: 200, al: 'l', b: true,
      d: function (k) {
        var w = k.w, h = k.h;
        k.r(0, 0, w, h, 8, k.body, 'ink', D);
        k.p('M' + r1(w - 26) + ' 14l10 10m0-10l-10 10', 'none', 'mid', { sw: 1.6 });
        for (var i = 0; i < 3 && 52 + i * 18 < h - 56; i++) k.bar(16, 52 + i * 18, (w - 32) * (i === 2 ? 0.6 : 1), 8);
        k.r(w - 180, h - 46, 76, 30, 6, 'soft', 'mid', { sw: 1 });
        k.r(w - 94, h - 46, 78, 30, 6, 'accent', 'none');
      },
      tb: function (w) { return { x: 16, y: 10, w: Math.max(10, w - 56), h: 28 }; }
    },
    button: { c: 'ui', n: 'Button', w: 120, h: 40, bf: 'soft', ph: 0.5, d: function (k) { k.r(0, 0, k.w, k.h, 6, k.body, 'ink', D); } },
    primary: {
      c: 'ui', n: 'Primary button', w: 120, h: 40, bf: ACCENT, ph: 0.5,
      d: function (k) { k.r(0, 0, k.w, k.h, 6, k.body, k.it.s ? 'ink' : 'none', D); }
    },
    textfield: {
      c: 'ui', n: 'Text field', w: 220, h: 36, al: 'l', mut: true, ph: 0.45,
      d: function (k) { k.r(0, 0, k.w, k.h, 4, k.body, 'ink', D); },
      tb: function (w, h) { return { x: 10, y: 2, w: Math.max(10, w - 20), h: h - 4 }; }
    },
    textarea: {
      c: 'ui', n: 'Text area', w: 260, h: 120, al: 'l', va: 't', mut: true,
      d: function (k) {
        var w = k.w, h = k.h;
        k.r(0, 0, w, h, 4, k.body, 'ink', D);
        k.p('M' + r1(w - 4) + ' ' + r1(h - 12) + 'L' + r1(w - 12) + ' ' + r1(h - 4) + 'M' + r1(w - 4) + ' ' + r1(h - 7) + 'L' + r1(w - 7) + ' ' + r1(h - 4), 'none', 'mid', { sw: 1.2 });
      },
      tb: function (w, h) { return { x: 10, y: 8, w: Math.max(10, w - 20), h: Math.max(10, h - 16) }; }
    },
    checkbox: {
      c: 'ui', n: 'Checkbox', w: 160, h: 24, al: 'l', ph: 0.6,
      d: function (k) {
        var y = (k.h - 16) / 2;
        k.r(0, y, 16, 16, 3, 'accent', 'none');
        k.p('M4 ' + r1(y + 8.5) + 'l3 3 5-6', 'none', '#ffffff', { sw: 2 });
      },
      tb: sideLabel
    },
    radio: {
      c: 'ui', n: 'Radio button', w: 160, h: 24, al: 'l', ph: 0.6,
      d: function (k) { k.e(8, k.h / 2, 7.5, 7.5, 'none', 'ink', { sw: 1.5 }); k.e(8, k.h / 2, 4, 4, 'accent', 'none'); },
      tb: sideLabel
    },
    toggle: {
      c: 'ui', n: 'Switch', w: 160, h: 24, al: 'l', ph: 0.6,
      d: function (k) { k.r(0, (k.h - 20) / 2, 38, 20, 10, 'accent', 'none'); k.e(28, k.h / 2, 7, 7, '#ffffff', 'none'); },
      tb: function (w, h) { return { x: 46, y: 0, w: Math.max(10, w - 46), h: h }; }
    },
    dropdown: {
      c: 'ui', n: 'Dropdown', w: 200, h: 36, al: 'l', ph: 0.5,
      d: function (k) {
        k.r(0, 0, k.w, k.h, 4, k.body, 'ink', D);
        k.p('M' + r1(k.w - 24) + ' ' + r1(k.h / 2 - 3) + 'l5 5 5-5', 'none', 'ink', { sw: 1.6 });
      },
      tb: function (w, h) { return { x: 10, y: 2, w: Math.max(10, w - 44), h: h - 4 }; }
    },
    slider: {
      c: 'ui', n: 'Slider', w: 200, h: 24, text: false,
      d: function (k) {
        var y = r1(k.h / 2), x = Math.max(8, k.w * 0.6);
        k.p('M8 ' + y + 'H' + r1(k.w - 8), 'none', 'faint', { sw: 4 });
        k.p('M8 ' + y + 'H' + r1(x), 'none', 'accent', { sw: 4 });
        k.e(x, k.h / 2, 8, 8, 'bg', 'ink', { sw: 1.5 });
      }
    },
    progress: {
      c: 'ui', n: 'Progress bar', w: 200, h: 14, text: false,
      d: function (k) { k.r(0, 0, k.w, k.h, k.h / 2, 'faint', 'none'); k.r(0, 0, k.w * 0.6, k.h, k.h / 2, 'accent', 'none'); }
    },
    tabs: {
      c: 'ui', n: 'Tabs', w: 360, h: 40, text: 'lines',
      d: function (k) {
        var names = k.lines(3), cw = k.w / names.length;
        k.p('M0 ' + r1(k.h - 0.5) + 'H' + r1(k.w), 'none', 'mid', { sw: 1 });
        names.forEach(function (s, i) {
          k.line(s, { x: i * cw + 6, y: 0, w: cw - 12, h: k.h - 3 }, { b: !i, al: 'c', op: i ? 0.75 : null, ph: 0.6 });
          if (!i) k.r(8, k.h - 3, cw - 16, 3, 1.5, 'accent', 'none');
        });
      }
    },
    list: {
      c: 'ui', n: 'List', w: 220, h: 160, text: 'lines',
      d: function (k) {
        var rows = k.lines(4), rh = k.h / rows.length;
        k.r(0, 0, k.w, k.h, 6, k.body, 'ink', D);
        rows.forEach(function (s, i) {
          if (i) k.p('M0 ' + r1(i * rh) + 'H' + r1(k.w), 'none', 'faint', { sw: 1 });
          k.line(s, { x: 12, y: i * rh, w: k.w - 24, h: rh }, { al: 'l', ph: 0.6 });
        });
      }
    },
    table: {
      c: 'ui', n: 'Table', w: 360, h: 160, text: 'lines',
      d: function (k) {
        var rows = k.lines(4).map(function (r) {
          r = r.replace(/^\s*\|/, '').replace(/\|\s*$/, '');
          return r ? r.split(/\s*\|\s*|\t/) : null;
        });
        var cols = Math.max(1, rows.reduce(function (m, r) { return Math.max(m, r ? r.length : 3); }, 0));
        var rh = k.h / rows.length, cw = k.w / cols, i;
        k.r(0, 0, k.w, k.h, 2, k.body, 'none');
        k.r(0, 0, k.w, rh, 0, 'soft', 'none');
        for (i = 1; i < rows.length; i++) k.p('M0 ' + r1(i * rh) + 'H' + r1(k.w), 'none', 'faint', { sw: 1 });
        for (i = 1; i < cols; i++) k.p('M' + r1(i * cw) + ' 0V' + r1(k.h), 'none', 'faint', { sw: 1 });
        rows.forEach(function (r, ri) {
          for (var c = 0; c < cols; c++) {
            k.line(r ? (r[c] || '').trim() : '', { x: c * cw + 8, y: ri * rh, w: cw - 16, h: rh }, { al: 'l', b: !ri, ph: r ? 0 : 0.7 });
          }
        });
        k.r(0, 0, k.w, k.h, 2, 'none', 'ink', D);
      }
    },
    image: {
      c: 'ui', n: 'Image', w: 200, h: 140, text: false,
      d: function (k) {
        var w = k.w, h = k.h, m = Math.min(w, h);
        k.r(0, 0, w, h, 4, k.it.f || 'soft', 'mid', { sw: 1, main: true });
        k.p(poly([w * 0.12, h * 0.82, w * 0.38, h * 0.42, w * 0.55, h * 0.64, w * 0.68, h * 0.5, w * 0.88, h * 0.82]), 'faint', 'none');
        k.e(w * 0.72, h * 0.27, m * 0.08, m * 0.08, 'faint', 'none');
      }
    },
    avatar: {
      c: 'ui', n: 'Avatar', w: 64, h: 64, text: false,
      d: function (k) {
        var w = k.w, h = k.h;
        k.e(w / 2, h / 2, w / 2, h / 2, k.it.f || 'soft', 'mid', { sw: 1, main: true });
        k.e(w / 2, h * 0.4, w * 0.17, h * 0.17, 'mid', 'none');
        k.p('M' + r1(w * 0.2) + ' ' + r1(h * 0.84) + 'C' + r1(w * 0.26) + ' ' + r1(h * 0.6) + ' ' + r1(w * 0.74) + ' ' + r1(h * 0.6) + ' ' +
          r1(w * 0.8) + ' ' + r1(h * 0.84) + 'Z', 'mid', 'none');
      }
    },
    icon: {
      c: 'ui', n: 'Icon', w: 40, h: 40, text: false,
      d: function (k) {
        k.r(0, 0, k.w, k.h, 8, k.it.f || 'soft', 'mid', { sw: 1, main: true });
        k.p(starD(k.w * 0.27, k.h * 0.25, k.w * 0.46, k.h * 0.46), 'mid', 'none');
      }
    },
    heading: {
      c: 'ui', n: 'Heading', w: 300, h: 44, fs: 28, b: true, al: 'l', ph: 0.7, fill: false,
      d: function () {}, tb: function (w, h) { return { x: 0, y: 0, w: w, h: h }; }
    },
    paragraph: {
      c: 'ui', n: 'Paragraph', w: 320, h: 100, fs: 14, al: 'l', va: 't', text: 'own', fill: false,
      d: function (k) {
        var fs = fsOf(k.it);
        if (k.it.txt) { k.label({ x: 0, y: 0, w: k.w, h: k.h }); return; }
        var lh = fs * 1.6, n = Math.max(1, Math.floor((k.h + fs * 0.6) / lh));
        for (var i = 0; i < n; i++) k.bar(0, i * lh + fs * 0.3, k.w * (i === n - 1 && n > 1 ? 0.6 : 1), fs * 0.55);
      },
      tb: function (w, h) { return { x: 0, y: 0, w: w, h: h }; }
    },
    link: {
      c: 'ui', n: 'Link', w: 120, h: 24, al: 'l', ul: true, col: ACCENT, ph: 0.8, fill: false,
      d: function () {}, tb: function (w, h) { return { x: 0, y: 0, w: w, h: h }; }
    },
    breadcrumb: {
      c: 'ui', n: 'Breadcrumb', w: 300, h: 24, fs: 14, text: 'lines', fill: false,
      d: function (k) {
        var parts = k.lines(3), x = 0, fs = fsOf(k.it);
        parts.forEach(function (s, i) {
          var last = i === parts.length - 1;
          var wd = s ? Math.min(textWidth([s], fs, last), Math.max(20, k.w - x)) : 50;
          k.line(s, { x: x, y: 0, w: wd, h: k.h }, { al: 'l', b: last, ph: 1, col: last ? null : ACCENT });
          x += wd + 6;
          if (!last) { k.line('›', { x: x, y: 0, w: 10, h: k.h }, { al: 'l', op: 0.6 }); x += 16; }
        });
      }
    },
    pagination: {
      c: 'ui', n: 'Pagination', w: 280, h: 32, text: false, fill: false,
      d: function (k) {
        var labels = ['‹', '1', '2', '3', '…', '9', '›'], s = Math.min(k.h, 40), gap = 6;
        var x = (k.w - (labels.length * s + (labels.length - 1) * gap)) / 2, y = (k.h - s) / 2;
        labels.forEach(function (l, i) {
          var on = i === 1;
          k.r(x, y, s, s, 6, on ? 'accent' : 'none', on || l === '…' ? 'none' : 'faint', { sw: 1 });
          k.line(l, { x: x, y: y, w: s, h: s }, { al: 'c', col: on ? '#ffffff' : null, fs: Math.max(8, Math.min(14, s * 0.45)) });
          x += s + gap;
        });
      }
    },
    search: {
      c: 'ui', n: 'Search field', w: 240, h: 36, al: 'l', mut: true, ph: 0.4,
      d: function (k) {
        var c = k.h / 2;
        k.r(0, 0, k.w, k.h, k.h / 2, k.body, 'ink', D);
        k.e(c, c - 1, 5.5, 5.5, 'none', 'mid', { sw: 1.6 });
        k.p('M' + r1(c + 4) + ' ' + r1(c + 3) + 'l4 4', 'none', 'mid', { sw: 1.6 });
      },
      tb: function (w, h) { return { x: h, y: 2, w: Math.max(10, w - h - 14), h: h - 4 }; }
    },
    card: {
      c: 'ui', n: 'Card', w: 220, h: 260, al: 'l', b: true, ph: 0.6,
      d: function (k) {
        var w = k.w, h = k.h, ih = h * 0.45;
        k.r(0, 0, w, h, 8, k.body, 'none');
        k.p(rrTop(0, 0, w, ih, 8), 'soft', 'none');
        k.p(poly([w * 0.2, ih * 0.82, w * 0.42, ih * 0.45, w * 0.56, ih * 0.65, w * 0.66, ih * 0.52, w * 0.82, ih * 0.82]), 'faint', 'none');
        for (var i = 0; i < 3 && ih + 46 + i * 16 + 7 < h - 6; i++) k.bar(12, ih + 46 + i * 16, (w - 24) * (i === 2 ? 0.55 : 1), 7);
        k.r(0, 0, w, h, 8, 'none', 'ink', D);
      },
      tb: function (w, h) { return { x: 12, y: h * 0.45 + 8, w: Math.max(10, w - 24), h: 26 }; }
    },
    navbar: {
      c: 'ui', n: 'Navigation bar', w: 640, h: 56, bf: 'soft', fs: 14, text: 'lines',
      d: function (k) {
        var w = k.w, h = k.h, fs = fsOf(k.it), items = k.lines(4), x = w - 16;
        k.r(0, 0, w, h, 0, k.body, 'none');
        k.p('M0 ' + r1(h - 0.5) + 'H' + r1(w), 'none', 'mid', { sw: 1 });
        k.e(28, h / 2, 12, 12, 'mid', 'none');
        for (var i = items.length - 1; i >= 0; i--) {
          var s = items[i], iw = s ? textWidth([s], fs, false) : 56;
          x -= iw;
          if (x < 52) break;
          k.line(s, { x: x, y: 0, w: iw + 1, h: h }, { al: 'l', ph: 1 });
          x -= 22;
        }
      }
    },
    sidebar: {
      c: 'ui', n: 'Sidebar', w: 200, h: 400, bf: 'soft', fs: 14, text: 'lines',
      d: function (k) {
        k.r(0, 0, k.w, k.h, 0, k.body, 'none');
        k.p('M' + r1(k.w - 0.5) + ' 0V' + r1(k.h), 'none', 'mid', { sw: 1 });
        k.lines(5).forEach(function (s, i) {
          var y = 16 + i * 40;
          if (y + 32 > k.h) return;
          if (!i) k.r(8, y, k.w - 16, 32, 6, 'faint', 'none');
          k.e(26, y + 16, 7, 7, i ? 'faint' : 'mid', 'none');
          k.line(s, { x: 42, y: y, w: k.w - 54, h: 32 }, { al: 'l', b: !i, ph: 0.7 });
        });
      }
    },
    tooltip: {
      c: 'ui', n: 'Tooltip', w: 160, h: 48, bf: 'soft', fs: 13,
      d: function (k) {
        var w = k.w, bh = k.h - 8, cx = w / 2;
        k.p('M6 0H' + r1(w - 6) + 'A6 6 0 0 1 ' + r1(w) + ' 6V' + r1(bh - 6) + 'A6 6 0 0 1 ' + r1(w - 6) + ' ' + r1(bh) + 'H' + r1(cx + 8) +
          'L' + r1(cx) + ' ' + r1(k.h) + 'L' + r1(cx - 8) + ' ' + r1(bh) + 'H6A6 6 0 0 1 0 ' + r1(bh - 6) + 'V6A6 6 0 0 1 6 0Z', k.body, 'ink', D);
      },
      tb: function (w, h) { return { x: 8, y: 2, w: Math.max(10, w - 16), h: Math.max(10, h - 12) }; }
    },

    // Mobile
    phone: {
      c: 'mobile', n: 'Phone', w: 300, h: 620, text: false,
      d: function (k) {
        var w = k.w, h = k.h;
        k.r(0, 0, w, h, 40, k.body, 'ink', { main: true, sw: k.sw + 1 });
        k.r(10, 10, w - 20, h - 20, 30, 'none', 'faint', { sw: 1 });
        k.r(w / 2 - Math.min(60, w * 0.18), 18, Math.min(120, w * 0.36), 24, 12, 'ink', 'none');
        k.r(w / 2 - Math.min(60, w * 0.2), h - 22, Math.min(120, w * 0.4), 5, 2.5, 'mid', 'none');
      }
    },
    tablet: {
      c: 'mobile', n: 'Tablet', w: 520, h: 700, text: false,
      d: function (k) {
        k.r(0, 0, k.w, k.h, 26, k.body, 'ink', { main: true, sw: k.sw + 1 });
        k.r(18, 18, k.w - 36, k.h - 36, 8, 'none', 'faint', { sw: 1 });
        k.e(k.w / 2, 9, 3, 3, 'mid', 'none');
      }
    },
    statusbar: {
      c: 'mobile', n: 'Status bar', w: 280, h: 24, text: false, fill: false,
      d: function (k) {
        var w = k.w, y = k.h / 2;
        k.line('9:41', { x: 8, y: 0, w: 60, h: k.h }, { al: 'l', b: true, fs: 13 });
        for (var i = 0; i < 4; i++) k.r(w - 70 + i * 6, y + 5 - (4 + i * 2), 4, 4 + i * 2, 1, i < 3 ? 'ink' : 'mid', 'none');
        k.r(w - 34, y - 5.5, 24, 11, 3, 'none', 'ink', { sw: 1.2 });
        k.r(w - 32, y - 3.5, 15, 7, 1.5, 'ink', 'none');
        k.r(w - 9, y - 2, 2, 4, 1, 'ink', 'none');
      }
    },
    appbar: {
      c: 'mobile', n: 'App bar', w: 280, h: 56, bf: 'soft', fs: 18, b: true, al: 'l', ph: 0.5,
      d: function (k) {
        var y = k.h / 2;
        k.r(0, 0, k.w, k.h, 0, k.body, 'none');
        k.p('M24 ' + r1(y - 7) + 'l-7 7 7 7M17 ' + r1(y) + 'h14', 'none', 'ink', { sw: 2 });
        [-6, 0, 6].forEach(function (d) { k.e(k.w - 18, y + d, 2, 2, 'ink', 'none'); });
      },
      tb: function (w, h) { return { x: 48, y: 0, w: Math.max(10, w - 84), h: h }; }
    },
    bottomnav: {
      c: 'mobile', n: 'Bottom navigation', w: 280, h: 64, bf: 'soft', fs: 11, text: 'lines',
      d: function (k) {
        var items = k.lines(4), cw = k.w / items.length;
        k.r(0, 0, k.w, k.h, 0, k.body, 'none');
        k.p('M0 0.5H' + r1(k.w), 'none', 'mid', { sw: 1 });
        items.forEach(function (s, i) {
          k.e(cw * i + cw / 2, k.h * 0.38, 9, 9, i ? 'faint' : 'accent', 'none');
          k.line(s, { x: cw * i + 4, y: k.h * 0.62, w: cw - 8, h: k.h * 0.3 }, { al: 'c', b: !i, ph: 0.5 });
        });
      }
    },
    fab: {
      c: 'mobile', n: 'Action button', w: 56, h: 56, bf: ACCENT, text: false,
      d: function (k) {
        k.e(k.w / 2, k.h / 2, k.w / 2, k.h / 2, k.body, k.it.s ? 'ink' : 'none', D);
        var m = Math.min(k.w, k.h) * 0.2, cx = r1(k.w / 2), cy = r1(k.h / 2);
        k.p('M' + cx + ' ' + r1(k.h / 2 - m) + 'v' + r1(2 * m) + 'M' + r1(k.w / 2 - m) + ' ' + cy + 'h' + r1(2 * m), 'none', textOn(k.body), { sw: 2.5 });
      }
    },
    listitem: {
      c: 'mobile', n: 'List item', w: 280, h: 64, fs: 15, text: 'lines',
      d: function (k) {
        var y = k.h / 2, l = k.lines(2), fs = fsOf(k.it);
        k.e(32, y, 18, 18, 'soft', 'mid', { sw: 1 });
        k.line(l[0] || '', { x: 62, y: y - fs * 1.25, w: k.w - 96, h: fs * 1.2 }, { al: 'l', b: true, ph: 0.6 });
        k.line(l[1] || '', { x: 62, y: y + 2, w: k.w - 96, h: fs * 1.1 }, { al: 'l', fs: fs * 0.85, op: 0.7, ph: 0.8 });
        k.p('M' + r1(k.w - 22) + ' ' + r1(y - 5) + 'l5 5-5 5', 'none', 'mid', { sw: 1.6 });
        k.p('M62 ' + r1(k.h - 0.5) + 'H' + r1(k.w), 'none', 'faint', { sw: 1 });
      }
    },

    // Shapes and symbols
    speech: {
      c: 'misc', n: 'Speech bubble', w: 200, h: 120,
      d: function (k) { k.p(bubbleD(k.w, k.h, 16), k.body, 'ink', D); },
      tb: function (w, h) { return { x: 10, y: 6, w: w - 20, h: Math.max(10, h * 0.8 - 12) }; }
    },
    thought: {
      c: 'misc', n: 'Thought bubble', w: 200, h: 140,
      d: function (k) {
        k.p(cloudD(0, 0, k.w, k.h * 0.8), k.body, 'ink', D);
        k.e(k.w * 0.22, k.h * 0.87, 7, 6, k.body, 'ink', D);
        k.e(k.w * 0.13, k.h * 0.96, 4, 3.5, k.body, 'ink', D);
      },
      tb: function (w, h) { return { x: w * 0.2, y: h * 0.16, w: w * 0.6, h: h * 0.48 }; }
    },
    callout: {
      c: 'misc', n: 'Callout', w: 200, h: 110,
      d: function (k) { k.p(bubbleD(k.w, k.h, 2), k.body, 'ink', D); },
      tb: function (w, h) { return { x: 10, y: 6, w: w - 20, h: Math.max(10, h * 0.8 - 12) }; }
    },
    cloud: {
      c: 'misc', n: 'Cloud', w: 200, h: 130,
      d: function (k) { k.p(cloudD(0, 0, k.w, k.h), k.body, 'ink', D); },
      tb: function (w, h) { return { x: w * 0.2, y: h * 0.2, w: w * 0.6, h: h * 0.6 }; }
    },
    star: {
      c: 'misc', n: 'Star', w: 120, h: 115,
      d: function (k) { k.p(starD(0, 0, k.w, k.h), k.body, 'ink', D); },
      tb: function (w, h) { return { x: w * 0.3, y: h * 0.4, w: w * 0.4, h: h * 0.32 }; }
    },
    heart: {
      c: 'misc', n: 'Heart', w: 120, h: 110,
      d: function (k) { k.p(scaleD(HEART_D, k.w / 100, k.h / 100), k.body, 'ink', D); },
      tb: function (w, h) { return { x: w * 0.22, y: h * 0.2, w: w * 0.56, h: h * 0.42 }; }
    },
    arrowR: blockArrow('r', 'Arrow right'),
    arrowL: blockArrow('l', 'Arrow left'),
    arrowU: blockArrow('u', 'Arrow up'),
    arrowD: blockArrow('d', 'Arrow down'),
    arrowLR: blockArrow('lr', 'Double arrow'),
    brace: {
      c: 'misc', n: 'Curly brace', w: 40, h: 160, text: false, fill: false,
      d: function (k) {
        var w = k.w, h = k.h, m = r1(w * 0.5), c = r1(w * 0.3), q = function (f) { return r1(h * f); };
        k.p('M' + r1(w) + ' 0C' + m + ' 0 ' + m + ' ' + q(0.04) + ' ' + m + ' ' + q(0.12) + 'V' + q(0.4) + 'C' + m + ' ' + q(0.47) + ' ' + c + ' ' + q(0.5) +
          ' 0 ' + q(0.5) + 'C' + c + ' ' + q(0.5) + ' ' + m + ' ' + q(0.53) + ' ' + m + ' ' + q(0.6) + 'V' + q(0.88) + 'C' + m + ' ' + q(0.96) + ' ' +
          m + ' ' + r1(h) + ' ' + r1(w) + ' ' + r1(h), 'none', 'ink', D);
      }
    },
    banner: {
      c: 'misc', n: 'Banner', w: 240, h: 56, bf: 'soft',
      d: function (k) {
        var n = Math.min(16, k.w * 0.15);
        k.p(poly([0, 0, k.w, 0, k.w - n, k.h / 2, k.w, k.h, 0, k.h, n, k.h / 2]), k.body, 'ink', D);
      },
      tb: function (w, h) { var n = Math.min(16, w * 0.15); return { x: n + 4, y: 4, w: Math.max(10, w - 2 * n - 8), h: h - 8 }; }
    }
  };

  var STENCIL_CATS = [['flow', 'catFlow', 'Flowchart'], ['ui', 'catUi', 'Wireframe'], ['mobile', 'catMobile', 'Mobile'], ['misc', 'catMisc', 'Shapes and symbols']];

  function stDef(it) { return STENCILS[it.k] || STENCILS.process; }
  function stName(k) { return S('s_' + k, STENCILS[k].n); }
  function stencilTB(it) {
    var def = stDef(it);
    return def.tb ? def.tb(it.w, it.h) : { x: 8, y: 4, w: Math.max(10, it.w - 16), h: Math.max(10, it.h - 8) };
  }

  /// Text colour on a fill: dark or light on a colour, else the ink.
  function textOn(f) {
    if (f === 'accent') return '#ffffff';
    if (f && f.charAt(0) === '#') return contrastInk(f);
    return ink;
  }

  function stencilLabelCol(it) {
    var def = stDef(it);
    return it.s || def.col || textOn(it.f || def.bf || 'none');
  }

  function trimTo(s, maxW, fs, bold) {
    mctx.font = (bold ? 'bold ' : '') + fs + 'px ' + FONT;
    if (mctx.measureText(s).width <= maxW) return s;
    while (s.length > 1 && mctx.measureText(s + '…').width > maxW) s = s.slice(0, -1);
    return s + '…';
  }

  function stencilKit(g, it, hide) {
    var def = stDef(it), col = it.s || ink, sw = swOf(it);
    var labelCol = stencilLabelCol(it);
    function paint(a, f, s, o) {
      o = o || {};
      [['fill', f], ['stroke', s]].forEach(function (pair) {
        var key = pair[0], v = pair[1];
        if (!v || v === 'none') { a[key] = 'none'; return; }
        if (v === 'soft' || v === 'faint' || v === 'mid') {
          a[key] = col;
          a[key + '-opacity'] = v === 'soft' ? 0.08 : v === 'faint' ? 0.16 : 0.32;
          return;
        }
        a[key] = v === 'ink' ? col : v === 'bg' ? canvasBg : v === 'accent' ? ACCENT : v;
      });
      if (a.stroke !== 'none') {
        var w = o.sw || sw;
        a['stroke-width'] = r1(w);
        a['stroke-linejoin'] = 'round';
        a['stroke-linecap'] = 'round';
        if (o.main) a['stroke-dasharray'] = dashArr(it, w);
      }
      return a;
    }
    var k = {
      w: it.w, h: it.h, sw: sw, it: it, hide: hide,
      body: it.f || def.bf || 'none',
      p: function (d, f, s, o) { g.appendChild(svgEl('path', paint({ d: d }, f, s, o))); },
      r: function (x, y, w, h, rx, f, s, o) {
        w = Math.max(0, w); h = Math.max(0, h);
        g.appendChild(svgEl('rect', paint({ x: r1(x), y: r1(y), width: r1(w), height: r1(h), rx: rx ? r1(Math.min(rx, w / 2, h / 2)) : null }, f, s, o)));
      },
      e: function (cx, cy, rx, ry, f, s, o) {
        g.appendChild(svgEl('ellipse', paint({ cx: r1(cx), cy: r1(cy), rx: r1(Math.max(0, rx)), ry: r1(Math.max(0, ry)) }, f, s, o)));
      },
      bar: function (x, y, w, h) { if (w > 0 && h > 0) k.r(x, y, w, h, h / 2, 'faint', 'none'); },
      /// The lines of `txt`, or `n` empty ones (placeholders).
      lines: function (n) {
        var l = String(it.txt || '').split('\n');
        while (l.length && !l[l.length - 1].trim()) l.pop();
        if (!l.length) for (var i = 0; i < n; i++) l.push('');
        return l;
      },
      /// One line of text in box `b` (cut to fit), or a placeholder bar.
      line: function (s, b, o) {
        o = o || {};
        if (hide && def.text === 'lines') return;
        var fs = o.fs || fsOf(it), bold = o.b !== undefined ? !!o.b : !!it.b, al = o.al || 'l';
        if (!s) {
          var ph = o.ph || 0;
          if (ph > 0 && b.w > 0) {
            var bw = b.w * ph, bh = Math.max(4, Math.min(fs * 0.55, b.h * 0.6));
            k.bar(al === 'l' ? b.x : al === 'r' ? b.x + b.w - bw : b.x + (b.w - bw) / 2, b.y + (b.h - bh) / 2, bw, bh);
          }
          return;
        }
        var t = textBlock(g, [trimTo(s, Math.max(b.w, fs), fs, bold)], b.x, b.y + (b.h - lineH(fs)) / 2, b.w, fs, o.col || labelCol, al, bold);
        if (o.op) t.setAttribute('opacity', o.op);
      },
      /// The item's text, wrapped into box `b`.
      label: function (b) {
        if (hide) return;
        var fs = fsOf(it), bold = it.b !== undefined ? !!it.b : !!def.b, al = alOf(it);
        if (!it.txt) {
          if (def.ph) k.line('', b, { ph: def.ph, al: al, fs: fs });
          return;
        }
        var lines = wrap(it.txt, b.w, fs, bold);
        var top = def.va === 't' ? b.y : b.y + (b.h - lines.length * lineH(fs)) / 2;
        var t = textBlock(g, lines, b.x, top, b.w, fs, labelCol, al, bold);
        if (def.mut && !it.s) t.setAttribute('opacity', 0.65);
        if (def.ul) t.setAttribute('text-decoration', 'underline');
      }
    };
    return k;
  }

  function drawStencil(g, it, ex, hide) {
    var def = stDef(it);
    if (!ex) hitRect(g, it.w, it.h);
    var k = stencilKit(g, it, hide);
    def.d(k);
    if (def.text === undefined || def.text === true) k.label(stencilTB(it));
  }

  // ----- emoji ---------------------------------------------------------------

  var EMOJI = [
    ['catSmileys', 'Smileys', '😀 grin smile|😃 smile happy|😄 laugh|😁 beam grin|😆 laughing|😅 sweat relief|🤣 rofl laugh|😂 joy tears laugh|🙂 slight smile|🙃 upside down|😉 wink|😊 blush happy|😇 angel innocent|🥰 love hearts|😍 heart eyes love|🤩 star struck wow|😘 kiss|😋 yum tasty|😛 tongue|😜 crazy wink|🤪 zany crazy|🤑 money rich|🤗 hug|🤭 oops giggle|🤫 shush quiet secret|🤔 think hmm|🤐 zipper secret|🤨 raised eyebrow doubt|😐 neutral|😑 expressionless|😶 speechless|😏 smirk|😒 unamused|🙄 eye roll|😬 grimace awkward|😌 relieved calm|😔 pensive|😪 sleepy|😴 sleep tired|😷 mask sick|🤒 fever ill|🤕 hurt injured|🤢 nausea sick|🤮 vomit|🥵 hot sweat|🥶 cold freezing|🥴 woozy|😵 dizzy|🤯 mind blown exploding|🤠 cowboy|🥳 party celebrate|😎 cool sunglasses|🤓 nerd geek|🧐 monocle inspect|😕 confused|😟 worried|🙁 frown sad|😮 surprised wow|😲 astonished shock|😳 flushed embarrassed|🥺 pleading please|😨 fearful scared|😰 anxious|😢 cry sad tear|😭 sob cry|😱 scream fear|😖 confounded|😞 disappointed|😓 downcast sweat|😩 weary|😫 tired exhausted|🥱 yawn bored|😤 huff triumph|😡 angry rage|😠 angry mad|🤬 swear cursing|😈 devil evil|💀 skull dead|💩 poo|🤡 clown|👻 ghost|👽 alien|🤖 robot bot|😺 cat smile'],
    ['catPeople', 'People and gestures', '👍 thumbs up yes like|👎 thumbs down no dislike|👌 ok perfect|✌️ victory peace|🤞 fingers crossed luck|🤟 love you|🤘 rock|👈 left point|👉 right point|👆 up point|👇 down point|☝️ index attention|✋ hand stop|👋 wave hello bye|🤙 call me|💪 strong muscle|👏 clap applause|🙌 raise hands celebrate|🙏 pray thanks please|🤝 handshake deal agreement|✍️ write|👀 eyes look see|🧠 brain smart idea|👤 person user|👥 people users team|🧑‍💻 developer coder programmer|🧑‍🏫 teacher|🧑‍🔬 scientist research|🧑‍🎨 artist design|🕵️ detective investigate|🦸 hero super|🧙 wizard magic|🏃 run fast|🚶 walk|💃 dance|👶 baby|🧑 person adult|👨 man|👩 woman|👴 old man|👵 old woman'],
    ['catNature', 'Nature', '🐶 dog|🐱 cat|🐭 mouse|🐰 rabbit bunny|🦊 fox|🐻 bear|🐼 panda|🐨 koala|🐯 tiger|🦁 lion|🐮 cow|🐷 pig|🐸 frog|🐵 monkey|🐔 chicken|🐧 penguin linux|🐦 bird|🦉 owl wise|🦄 unicorn|🐝 bee busy|🐛 bug|🦋 butterfly|🐢 turtle slow|🐍 snake python|🐙 octopus|🐟 fish|🐳 whale docker|🦈 shark|🌵 cactus|🌲 tree evergreen|🌳 tree|🌴 palm|🌱 seedling grow|🍀 clover luck|🍁 leaf autumn|🌸 blossom spring|🌹 rose|🌻 sunflower|🌞 sun|🌙 moon night|⭐ star|🌟 glowing star|⚡ lightning fast power|🔥 fire hot|🌈 rainbow|☁️ cloud|⛅ partly sunny|🌧️ rain|❄️ snow cold|🌊 wave water ocean|🌍 earth world globe'],
    ['catFood', 'Food and drink', '🍎 apple|🍌 banana|🍇 grapes|🍓 strawberry|🍒 cherry|🍑 peach|🍍 pineapple|🥑 avocado|🥕 carrot|🌽 corn|🌶️ chili pepper hot|🍄 mushroom|🍞 bread|🧀 cheese|🍕 pizza|🍔 burger|🍟 fries|🌭 hot dog|🌮 taco|🍣 sushi|🍜 noodles ramen|🍝 pasta spaghetti|🍦 ice cream|🍩 donut|🍪 cookie|🎂 cake birthday|🍫 chocolate|🍬 candy|☕ coffee|🍵 tea|🍺 beer|🍷 wine|🥂 cheers toast|🍾 champagne'],
    ['catActivity', 'Activities and travel', '⚽ soccer football|🏀 basketball|🎾 tennis|🎯 target goal bullseye|🎲 dice game|🎮 game controller|🧩 puzzle piece|♟️ chess strategy|🎨 art palette|🎬 film movie|🎤 microphone sing|🎧 headphones music|🎵 music note|🎸 guitar|🏆 trophy winner|🥇 gold medal first|🎉 party tada celebrate|🎁 gift present|🎈 balloon|🚀 rocket launch|✈️ plane travel flight|🚗 car|🚲 bike bicycle|🚂 train|🚢 ship|🏠 house home|🏢 office building|🏖️ beach vacation|⛰️ mountain|🗺️ map|🧭 compass direction|🏁 finish flag'],
    ['catObjects', 'Objects', '💡 idea light bulb|📌 pin pushpin|📍 location pin|📎 paperclip attachment|✏️ pencil|🖊️ pen|📝 memo note|📋 clipboard|📅 calendar date|⏰ alarm clock|⏳ hourglass wait|⌛ time|📊 bar chart|📈 chart up growth|📉 chart down|📁 folder|📂 open folder|📄 page document|📚 books|📖 book read|🔖 bookmark|📦 package box|✉️ envelope mail|📧 email|📞 phone call|📱 mobile phone|💻 laptop computer|🖥️ desktop computer|⌨️ keyboard|🖱️ computer mouse|🖨️ printer|💾 save disk|🔍 search magnifier|🔑 key|🔒 locked lock|🔓 unlocked|🔔 bell notification|📣 megaphone announce|💬 speech chat|💭 thought|🛠️ tools|🔧 wrench fix|🔨 hammer|⚙️ gear settings|🧲 magnet|🧪 test tube experiment|🔬 microscope|🔭 telescope|💰 money bag|💳 credit card|💎 gem diamond|🏷️ label tag|🗑️ trash bin|🧹 broom clean|🛒 shopping cart|🎓 graduation|🧾 receipt|📷 camera photo|🔋 battery|🔌 plug'],
    ['catSymbols', 'Symbols', '❤️ red heart love|🧡 orange heart|💛 yellow heart|💚 green heart|💙 blue heart|💜 purple heart|🖤 black heart|💔 broken heart|✅ check done yes|☑️ checkbox|✔️ check mark|❌ cross no wrong|❎ cross box|➕ plus add|➖ minus|❓ question|❗ exclamation important|‼️ double exclamation|⚠️ warning caution|🚫 forbidden prohibited|⛔ no entry|🛑 stop sign|🔴 red circle|🟠 orange circle|🟡 yellow circle|🟢 green circle|🔵 blue circle|🟣 purple circle|⚫ black circle|⚪ white circle|🟥 red square|🟩 green square|🟦 blue square|⬆️ up arrow|⬇️ down arrow|⬅️ left arrow|➡️ right arrow|↩️ back return|🔄 refresh cycle|🔁 repeat loop|♻️ recycle|💯 hundred perfect|🆕 new|🆗 ok|🆒 cool|🔝 top|✨ sparkles magic|💥 boom collision|💤 sleep zzz|♾️ infinity|ℹ️ information'],
  ].map(function (c) {
    return {
      key: c[0], name: c[1],
      list: c[2].split('|').map(function (e) {
        var i = e.indexOf(' ');
        return { ch: e.slice(0, i), words: e.slice(i + 1) };
      })
    };
  });

  // ----- DOM -----------------------------------------------------------------

  var app = document.getElementById('app');
  app.textContent = '';
  var svg = svgEl('svg', { id: 'board' });
  var defs = svgEl('defs');
  defs.innerHTML =
    '<pattern id="qc-grid" patternUnits="userSpaceOnUse" width="20" height="20"><circle class="dot" cx="1" cy="1" r="1"/></pattern>' +
    '<filter id="qc-shadow" x="-10%" y="-10%" width="125%" height="135%"><feDropShadow dx="0" dy="2" stdDeviation="3" flood-color="#000" flood-opacity=".18"/></filter>';
  svg.appendChild(defs);
  var gridRect = svgEl('rect', { x: 0, y: 0, width: '100%', height: '100%', fill: 'url(#qc-grid)' });
  svg.appendChild(gridRect);
  var world = svgEl('g');
  svg.appendChild(world);
  var overlay = svgEl('g');
  svg.appendChild(overlay);
  app.appendChild(svg);
  var pattern = defs.querySelector('#qc-grid');
  var patternDot = pattern.querySelector('circle');

  var hint = h('div', { class: 'hint' }, h('div', { class: 'hint-text' }, S('hint', 'Pick a tool on the left – draw, write, add sticky notes or paste images.')));
  var tplBox = h('div', { class: 'templates' });
  hint.appendChild(tplBox);
  app.appendChild(hint);

  var titleInput = h('input', { type: 'text', placeholder: S('title', 'Untitled canvas'), spellcheck: 'false' });
  var titleBox = h('div', { class: 'titlebox panel' }, titleInput);
  app.appendChild(titleBox);
  titleInput.addEventListener('keydown', function (e) {
    e.stopPropagation();
    if (e.key === 'Enter' || e.key === 'Escape') titleInput.blur();
  });
  titleInput.addEventListener('change', function () {
    var v = titleInput.value.trim();
    if (v) scene.title = v; else delete scene.title;
    commit();
  });

  var toolsBox = h('div', { class: 'tools panel' });
  app.appendChild(toolsBox);
  var styleBar = h('div', { class: 'stylebar panel' });
  app.appendChild(styleBar);
  var zoomBox = h('div', { class: 'zoom panel' });
  app.appendChild(zoomBox);

  // ----- toolbar ---------------------------------------------------------------

  var toolButtons = {};
  var shapeBtn = null;
  var SHAPE_NAMES = { rect: S('rect', 'Rectangle'), ellipse: S('ellipse', 'Ellipse'), diamond: S('diamond', 'Diamond'), triangle: S('triangle', 'Triangle') };
  var SHAPE_KEYS = { rect: 'R', ellipse: 'O', diamond: 'D', triangle: '' };
  var TOOL_KEYS = { v: 'select', h: 'hand', p: 'pen', m: 'hl', e: 'eraser', r: 'rect', o: 'ellipse', d: 'diamond', l: 'line', a: 'arrow', c: 'conn', t: 'text', n: 'sticky', f: 'frame' };

  function tip(label, key) { return key ? label + ' (' + key + ')' : label; }

  function buildTools() {
    function add(name, label, key) {
      var b = ibtn(icon(name), tip(label, key), function () { setTool(name); });
      toolButtons[name] = b;
      toolsBox.appendChild(b);
    }
    function sep() { toolsBox.appendChild(h('div', { class: 'sep' })); }
    add('select', S('select', 'Select'), 'V');
    add('hand', S('hand', 'Hand'), 'H');
    sep();
    add('pen', S('pen', 'Pen'), 'P');
    add('hl', S('highlighter', 'Highlighter'), 'M');
    add('eraser', S('eraser', 'Eraser'), 'E');
    sep();
    shapeBtn = ibtn(icon(shapeKind), tip(S('shapes', 'Shapes'), 'S'), function (e, b) {
      setTool(shapeKind);
      libraryPop(b);
    }, 'more');
    toolsBox.appendChild(shapeBtn);
    add('frame', S('frame', 'Frame'), 'F');
    add('line', S('line', 'Line'), 'L');
    add('arrow', S('arrow', 'Arrow'), 'A');
    add('conn', S('connector', 'Connector'), 'C');
    sep();
    add('text', S('text', 'Text'), 'T');
    add('sticky', S('sticky', 'Sticky note'), 'N');
    toolsBox.appendChild(ibtn(icon('emoji'), S('emoji', 'Emoji and stickers'), function (e, b) { emojiPop(b); }));
    toolsBox.appendChild(ibtn(icon('image'), S('image', 'Image…'), function () { post({ type: 'canvas-image' }); }));
  }

  function setTool(t) {
    if (editing) finishEdit();
    tool = t;
    if (SHAPES.indexOf(t) >= 0) {
      shapeKind = t;
      shapeBtn.innerHTML = icon(t);
      shapeBtn.title = tip(S('shapes', 'Shapes'), 'S');
    }
    Object.keys(toolButtons).forEach(function (k) { toolButtons[k].classList.toggle('on', k === t); });
    shapeBtn.classList.toggle('on', SHAPES.indexOf(t) >= 0);
    svg.setAttribute('class', 't-' + t);
    hoverId = null;
    if (t !== 'select' && sel.length) { sel = []; renderOverlay(); }
    refreshStyleBar();
    renderOverlay();
  }

  // ----- zoom widget -------------------------------------------------------------

  var zoomLabel, snapBtn;
  function buildZoom() {
    zoomBox.appendChild(ibtn(icon('minus'), S('zoomOut', 'Zoom out'), function () { zoomCentered(view.z / 1.25); }));
    zoomLabel = h('button', { class: 'ibtn txt pct', type: 'button', title: S('zoomReset', 'Zoom to 100 %') }, '100%');
    zoomLabel.addEventListener('click', function () { zoomCentered(1); });
    zoomLabel.addEventListener('pointerdown', function (e) { e.stopPropagation(); });
    zoomBox.appendChild(zoomLabel);
    zoomBox.appendChild(ibtn(icon('plus'), S('zoomIn', 'Zoom in'), function () { zoomCentered(view.z * 1.25); }));
    zoomBox.appendChild(ibtn(icon('fit'), S('fit', 'Fit to content'), function () { fit(2); applyView(); }));
    zoomBox.appendChild(h('div', { class: 'sep' }));
    snapBtn = ibtn(icon('grid'), S('snap', 'Snap to grid'), function () {
      snapOn = !snapOn;
      snapBtn.classList.toggle('on', snapOn);
    }, 'on');
    zoomBox.appendChild(snapBtn);
  }

  // ----- geometry ----------------------------------------------------------------

  function index() {
    byId = {};
    scene.items.forEach(function (it) { byId[it.id] = it; });
  }

  function center(it) { return { x: it.x + it.w / 2, y: it.y + it.h / 2 }; }

  function corners(it) {
    var c = center(it), r = it.rot || 0;
    return [[it.x, it.y], [it.x + it.w, it.y], [it.x + it.w, it.y + it.h], [it.x, it.y + it.h]].map(function (p) {
      return rotP(p[0], p[1], c.x, c.y, r);
    });
  }

  function boxOf(pts) {
    var x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
    pts.forEach(function (p) {
      x0 = Math.min(x0, p.x); y0 = Math.min(y0, p.y); x1 = Math.max(x1, p.x); y1 = Math.max(y1, p.y);
    });
    if (x0 === Infinity) return { x: 0, y: 0, w: 0, h: 0 };
    return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
  }

  function aabb(it) {
    if (it.t === 'line' || it.t === 'arrow') {
      return boxOf([{ x: it.pts[0], y: it.pts[1] }, { x: it.pts[2], y: it.pts[3] }]);
    }
    if (it.t === 'conn') return boxOf(route(it).pts);
    if (it.t === 'frame') {
      var t = fsOf(it) * 1.8;
      return { x: it.x, y: it.y - t, w: it.w, h: it.h + t };
    }
    return boxOf(corners(it));
  }

  function union(items) {
    var pts = [];
    items.forEach(function (it) {
      var b = aabb(it);
      pts.push({ x: b.x, y: b.y }, { x: b.x + b.w, y: b.y + b.h });
    });
    return boxOf(pts);
  }

  function contentBox() {
    if (!scene.items.length) return { x: 0, y: 0, w: 400, h: 300 };
    return union(scene.items);
  }

  // ----- frames ------------------------------------------------------------------
  //
  // Frames are titled containers. They always lie below everything else;
  // moving one moves what lies inside it (by centre).

  function zOrder(items) {
    var fr = [], rest = [];
    items.forEach(function (it) { (it.t === 'frame' ? fr : rest).push(it); });
    return fr.length ? fr.concat(rest) : items;
  }

  /// Items whose centre lies in one of `frames` (not in `skip`, which is
  /// extended with what is found).
  function framedItems(frames, skip) {
    var out = [];
    frames.forEach(function (f) {
      scene.items.forEach(function (it) {
        if (it === f || skip[it.id] || it.t === 'conn') return;
        var b = it.t === 'frame' ? { x: it.x, y: it.y, w: it.w, h: it.h } : aabb(it), cx = b.x + b.w / 2, cy = b.y + b.h / 2;
        if (cx >= f.x && cx <= f.x + f.w && cy >= f.y && cy <= f.y + f.h) { skip[it.id] = 1; out.push(it); }
      });
    });
    return out;
  }

  function idSet(items) { var s = {}; items.forEach(function (it) { s[it.id] = 1; }); return s; }

  /// The selection plus the contents of selected frames.
  function selWithContents() {
    var items = selItems(), frames = items.filter(function (it) { return it.t === 'frame'; });
    return frames.length ? items.concat(framedItems(frames, idSet(items))) : items;
  }

  /// The box items are aligned by (frames without their title).
  function bodyBox(it) { return it.t === 'frame' ? { x: it.x, y: it.y, w: it.w, h: it.h } : aabb(it); }

  /// Moves items ([item, dx, dy]); frames take their contents along.
  function shiftAll(moves) {
    var moved = idSet(moves.map(function (m) { return m[0]; }));
    moves.forEach(function (m) {
      if (!m[1] && !m[2]) return;
      var list = [m[0]];
      if (m[0].t === 'frame') framedItems([m[0]], moved).forEach(function (x) { if (!x.lock) list.push(x); });
      list.forEach(function (x) { moveItem(x, clone(x), m[1], m[2]); });
    });
  }

  function drawFrame(g, it, ex, hide) {
    var w = it.w, hh = it.h, col = it.s || ink, fs = fsOf(it), title = it.txt || '';
    g.appendChild(svgEl('rect', {
      width: r1(w), height: r1(hh), rx: 3, fill: it.f || col, 'fill-opacity': it.f ? null : 0.035,
      stroke: col, 'stroke-opacity': it.s ? null : 0.3, 'stroke-width': 1.5, 'stroke-dasharray': dashArr(it, 1.5),
      'pointer-events': 'none'
    }));
    if (title && !hide) {
      var tt = svgEl('text', {
        x: 0, y: r1(-fs * 0.55), 'font-size': fs, 'font-family': FONT, 'font-weight': 600, fill: col,
        'fill-opacity': it.s ? null : 0.75, style: 'white-space:pre'
      });
      tt.textContent = trimTo(title, Math.max(w, 40), fs, true);
      g.appendChild(tt);
    }
    if (!ex) {
      hitPath(g, 'M0 0H' + r1(w) + 'V' + r1(hh) + 'H0Z', 2);
      var tw = title ? Math.min(w, textWidth([title], fs, true)) : 0;
      g.appendChild(svgEl('rect', { class: 'hit', y: r1(-fs * 1.7), width: r1(Math.max(tw, 60)), height: r1(fs * 1.6), fill: 'none', 'pointer-events': 'all' }));
    }
  }

  var FRAME_PRESETS = [
    ['fA4p', 'A4 portrait', 595, 842], ['fA4l', 'A4 landscape', 842, 595], ['fSlide', 'Slide 16:9', 1280, 720],
    ['fDesktop', 'Desktop', 1440, 900], ['fPhone', 'Phone', 390, 844]
  ];

  /// Zooms out (if needed) so that box `bb` fits in the view, centred.
  function showBox(bb) {
    var r = svg.getBoundingClientRect(), W = r.width || 800, H = r.height || 600;
    var z = Math.min(view.z, (W - 160) / Math.max(bb.w, 1), (H - 140) / Math.max(bb.h, 1));
    z = Math.max(ZMIN, Math.min(ZMAX, z));
    view = { x: W / 2 - (bb.x + bb.w / 2) * z, y: H / 2 - (bb.y + bb.h / 2) * z, z: z };
    applyView();
  }

  function ensureVisible(it) {
    var r = svg.getBoundingClientRect(), b = aabb(it);
    if (b.w * view.z > (r.width || 800) - 160 || b.h * view.z > (r.height || 600) - 140) showBox(b);
  }

  // ----- connectors ----------------------------------------------------------------

  var SIDES = { n: [0.5, 0, 0, -1], e: [1, 0.5, 1, 0], s: [0.5, 1, 0, 1], w: [0, 0.5, -1, 0] };

  function anchor(it, side) {
    var d = SIDES[side], c = center(it), r = it.rot || 0;
    var p = rotP(it.x + it.w * d[0], it.y + it.h * d[1], c.x, c.y, r);
    var v = rotP(d[2], d[3], 0, 0, r);
    return { x: p.x, y: p.y, dx: v.x, dy: v.y };
  }

  function nearestSide(it, q) {
    var best = 'e', bd = Infinity;
    Object.keys(SIDES).forEach(function (s) {
      var a = anchor(it, s), d = dist(a, q);
      if (d < bd) { bd = d; best = s; }
    });
    return best;
  }

  /// The anchor near `p` (within a few screen pixels), else 'c' (automatic).
  function anchorNear(it, p) {
    var best = 'c', bd = 14 / view.z;
    Object.keys(SIDES).forEach(function (s) {
      var d = dist(anchor(it, s), p);
      if (d < bd) { bd = d; best = s; }
    });
    return best;
  }

  function endItem(end) { return end && end.id ? byId[end.id] : null; }
  function refPoint(end) {
    var it = endItem(end);
    return it ? center(it) : { x: end ? end.x || 0 : 0, y: end ? end.y || 0 : 0 };
  }

  function endPoint(end, toward) {
    var it = endItem(end);
    if (!it) return { x: end ? end.x || 0 : 0, y: end ? end.y || 0 : 0, dx: 0, dy: 0, free: true };
    var side = end.p && SIDES[end.p] ? end.p : nearestSide(it, toward);
    return anchor(it, side);
  }

  function freeDir(P, Q, axis) {
    var dx = Q.x - P.x, dy = Q.y - P.y;
    if (axis) {
      if (Math.abs(dx) >= Math.abs(dy)) { P.dx = dx >= 0 ? 1 : -1; P.dy = 0; } else { P.dx = 0; P.dy = dy >= 0 ? 1 : -1; }
    } else {
      var l = Math.hypot(dx, dy) || 1;
      P.dx = dx / l; P.dy = dy / l;
    }
  }

  function route(c) {
    var ra = refPoint(c.a), rb = refPoint(c.b);
    var A = endPoint(c.a, rb), B = endPoint(c.b, ra);
    var kind = c.k || 'straight';
    if (A.free) freeDir(A, B, kind === 'elbow');
    if (B.free) freeDir(B, A, kind === 'elbow');
    if (kind === 'curve') {
      var d = Math.max(30, dist(A, B) / 2.5);
      var c1 = { x: A.x + A.dx * d, y: A.y + A.dy * d }, c2 = { x: B.x + B.dx * d, y: B.y + B.dy * d };
      var pts = [];
      for (var i = 0; i <= 12; i++) pts.push(bez(A, c1, c2, B, i / 12));
      return { kind: kind, A: A, B: B, c1: c1, c2: c2, pts: pts };
    }
    if (kind === 'elbow') {
      // Stubs out of the anchors; shorter when the ends face each other closely.
      var stub = function (P, Q) {
        var front = (Q.x - P.x) * P.dx + (Q.y - P.y) * P.dy;
        return front > 0 ? Math.min(24, front / 2) : 24;
      };
      var ga = stub(A, B), gb = stub(B, A);
      var A1 = { x: A.x + A.dx * ga, y: A.y + A.dy * ga }, B1 = { x: B.x + B.dx * gb, y: B.y + B.dy * gb };
      var list = [A, A1];
      if (Math.abs(A.dx) >= Math.abs(A.dy)) {
        var mx = (A1.x + B1.x) / 2;
        if (Math.abs(B.dx) >= Math.abs(B.dy)) list.push({ x: mx, y: A1.y }, { x: mx, y: B1.y });
        else list.push({ x: B1.x, y: A1.y });
      } else {
        var my = (A1.y + B1.y) / 2;
        if (Math.abs(B.dy) >= Math.abs(B.dx)) list.push({ x: A1.x, y: my }, { x: B1.x, y: my });
        else list.push({ x: A1.x, y: B1.y });
      }
      list.push(B1, B);
      var out = [];
      list.forEach(function (p) {
        var last = out[out.length - 1];
        if (!last || Math.abs(last.x - p.x) > 0.01 || Math.abs(last.y - p.y) > 0.01) out.push({ x: p.x, y: p.y });
      });
      return { kind: kind, A: A, B: B, pts: out };
    }
    return { kind: 'straight', A: A, B: B, pts: [{ x: A.x, y: A.y }, { x: B.x, y: B.y }] };
  }

  function bez(a, b, c, d, t) {
    var u = 1 - t;
    return {
      x: u * u * u * a.x + 3 * u * u * t * b.x + 3 * u * t * t * c.x + t * t * t * d.x,
      y: u * u * u * a.y + 3 * u * u * t * b.y + 3 * u * t * t * c.y + t * t * t * d.y
    };
  }

  function midOf(r) {
    if (r.kind === 'curve') return bez(r.A, r.c1, r.c2, r.B, 0.5);
    var pts = r.pts, total = 0, i;
    for (i = 1; i < pts.length; i++) total += dist(pts[i - 1], pts[i]);
    var half = total / 2;
    for (i = 1; i < pts.length; i++) {
      var l = dist(pts[i - 1], pts[i]);
      if (half <= l && l > 0) {
        var t = half / l;
        return { x: pts[i - 1].x + (pts[i].x - pts[i - 1].x) * t, y: pts[i - 1].y + (pts[i].y - pts[i - 1].y) * t };
      }
      half -= l;
    }
    return pts[0];
  }

  function arrowLen(sw) { return 8 + sw * 2.5; }

  function shorten(p, from, kind, sw) {
    if (kind !== 'arrow') return p;
    var d = dist(p, from);
    if (!d) return p;
    var k = Math.min(arrowLen(sw) * 0.7, d * 0.5) / d;
    return { x: p.x + (from.x - p.x) * k, y: p.y + (from.y - p.y) * k };
  }

  function endMark(g, kind, tipP, from, sw, col) {
    if (!kind || kind === 'none') return;
    var dx = tipP.x - from.x, dy = tipP.y - from.y, l = Math.hypot(dx, dy) || 1;
    dx /= l; dy /= l;
    if (kind === 'dot') {
      g.appendChild(svgEl('circle', { cx: r1(tipP.x), cy: r1(tipP.y), r: r1(2.5 + sw * 1.2), fill: col }));
      return;
    }
    var len = arrowLen(sw), wd = len * 0.5;
    var bx = tipP.x - dx * len, by = tipP.y - dy * len;
    var pts = [tipP.x, tipP.y, bx - dy * wd, by + dx * wd, bx + dy * wd, by - dx * wd].map(r1).join(' ');
    g.appendChild(svgEl('polygon', { points: pts, fill: col, stroke: col, 'stroke-width': 1, 'stroke-linejoin': 'round' }));
  }

  function polyD(pts) {
    return pts.map(function (p, i) { return (i ? 'L' : 'M') + r1(p.x) + ' ' + r1(p.y); }).join('');
  }

  // ----- text layout -----------------------------------------------------------------

  var mctx = document.createElement('canvas').getContext('2d');

  function wrap(text, maxW, fs, bold) {
    mctx.font = (bold ? 'bold ' : '') + fs + 'px ' + FONT;
    maxW = Math.max(maxW, fs);
    var out = [];
    String(text || '').split('\n').forEach(function (par) {
      var line = '';
      par.split(' ').forEach(function (w) {
        var test = line ? line + ' ' + w : w;
        if (mctx.measureText(test).width <= maxW) { line = test; return; }
        if (line) { out.push(line); line = ''; }
        while (w.length > 1 && mctx.measureText(w).width > maxW) {
          var k = w.length - 1;
          while (k > 1 && mctx.measureText(w.slice(0, k)).width > maxW) k--;
          out.push(w.slice(0, k));
          w = w.slice(k);
        }
        line = w;
      });
      out.push(line);
    });
    return out;
  }

  /// Dark or white text for a fill colour (#rgb or #rrggbb).
  function contrastInk(fill) {
    var m = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(fill || '');
    if (!m) return ink;
    var hx = m[1].length === 3 ? m[1].replace(/./g, '$&$&') : m[1];
    var c = [0, 2, 4].map(function (i) {
      var v = parseInt(hx.substr(i, 2), 16) / 255;
      return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
    });
    var lum = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    return lum > 0.18 ? '#1e1e1e' : '#ffffff';
  }

  function textWidth(lines, fs, bold) {
    mctx.font = (bold ? 'bold ' : '') + fs + 'px ' + FONT;
    return lines.reduce(function (m, l) { return Math.max(m, mctx.measureText(l).width); }, 0);
  }

  function textBlock(g, lines, x, top, w, fs, col, al, bold) {
    var t = svgEl('text', {
      'font-size': fs, 'font-family': FONT, fill: col, 'font-weight': bold ? 700 : null,
      'text-anchor': al === 'l' ? 'start' : al === 'r' ? 'end' : 'middle', style: 'white-space:pre'
    });
    var tx = al === 'l' ? x : al === 'r' ? x + w : x + w / 2;
    lines.forEach(function (line, i) {
      var ts = svgEl('tspan', { x: r1(tx), y: r1(top + i * lineH(fs) + fs * 0.93) });
      ts.textContent = line;
      t.appendChild(ts);
    });
    g.appendChild(t);
    return t;
  }

  function textLayout(it) {
    var fs = fsOf(it), lines = wrap(it.txt || '', it.w - 8, fs, it.b);
    return { fs: fs, lines: lines, h: Math.max(1, lines.length) * lineH(fs) + 8 };
  }

  function stickyFit(it) {
    var pad = 12, maxW = it.w - 2 * pad, maxH = it.h - 2 * pad, fs = fsOf(it), lines;
    for (;;) {
      lines = wrap(it.txt || '', maxW, fs, it.b);
      if (lines.length * lineH(fs) <= maxH || fs <= 8) break;
      fs = Math.max(8, fs - 2);
    }
    return { fs: fs, lines: lines };
  }

  // ----- drawing items ---------------------------------------------------------------

  function hitRect(g, w, h) {
    g.appendChild(svgEl('rect', { class: 'hit', width: r1(w), height: r1(h), fill: 'none', 'pointer-events': 'all' }));
  }

  function hitPath(g, d, sw) {
    g.appendChild(svgEl('path', {
      class: 'hit', d: d, fill: 'none', stroke: 'transparent', 'stroke-width': Math.max(sw + 8, 14),
      'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'pointer-events': 'stroke'
    }));
  }

  function draw(it, ex) {
    var g = svgEl('g');
    if (!ex) g.setAttribute('data-id', it.id);
    if (it.op !== undefined && it.op < 1) g.setAttribute('opacity', it.op);
    var t = it.t, col = it.s || ink, sw = swOf(it);
    var hideText = !ex && editing === it.id;

    if (t === 'line' || t === 'arrow') {
      var P = it.pts, A = { x: P[0], y: P[1] }, B = { x: P[2], y: P[3] }, ends = endsOf(it);
      var d = polyD([shorten(A, B, ends[0], sw), shorten(B, A, ends[1], sw)]);
      g.appendChild(svgEl('path', {
        d: d, fill: 'none', stroke: col, 'stroke-width': sw, 'stroke-linecap': 'round',
        'stroke-dasharray': dashArr(it, sw)
      }));
      endMark(g, ends[0], A, B, sw, col);
      endMark(g, ends[1], B, A, sw, col);
      if (!ex) hitPath(g, polyD([A, B]), sw);
      return g;
    }

    if (t === 'conn') {
      var r = route(it), e2 = endsOf(it), pd, hd;
      if (r.kind === 'curve') {
        var a2 = shorten(r.A, r.c1, e2[0], sw), b2 = shorten(r.B, r.c2, e2[1], sw);
        pd = 'M' + r1(a2.x) + ' ' + r1(a2.y) + 'C' + [r.c1.x, r.c1.y, r.c2.x, r.c2.y, b2.x, b2.y].map(r1).join(' ');
        hd = 'M' + r1(r.A.x) + ' ' + r1(r.A.y) + 'C' + [r.c1.x, r.c1.y, r.c2.x, r.c2.y, r.B.x, r.B.y].map(r1).join(' ');
        endMark(g, e2[0], r.A, r.c1, sw, col);
        endMark(g, e2[1], r.B, r.c2, sw, col);
      } else {
        var pts = r.pts.slice(), n = pts.length;
        var s0 = pts[0], s1 = pts[1], t0 = pts[n - 1], t1 = pts[n - 2];
        pts[0] = shorten(s0, s1, e2[0], sw);
        pts[n - 1] = shorten(t0, t1, e2[1], sw);
        pd = polyD(pts);
        hd = polyD(r.pts);
        endMark(g, e2[0], s0, s1, sw, col);
        endMark(g, e2[1], t0, t1, sw, col);
      }
      g.insertBefore(svgEl('path', {
        d: pd, fill: 'none', stroke: col, 'stroke-width': sw, 'stroke-linecap': 'round', 'stroke-linejoin': 'round',
        'stroke-dasharray': dashArr(it, sw)
      }), g.firstChild);
      if (!ex) hitPath(g, hd, sw);
      if (it.txt && !hideText) {
        var fs = fsOf(it), lines = wrap(it.txt, 200, fs, it.b), m = midOf(r);
        var tw = textWidth(lines, fs, it.b) + 8, th = lines.length * lineH(fs) + 4;
        g.appendChild(svgEl('rect', { x: r1(m.x - tw / 2), y: r1(m.y - th / 2), width: r1(tw), height: r1(th), rx: 3, fill: canvasBg }));
        textBlock(g, lines, m.x - tw / 2, m.y - th / 2 + 2, tw, fs, col, 'c', it.b);
      }
      return g;
    }

    var tf = 'translate(' + r1(it.x) + ' ' + r1(it.y) + ')';
    if (it.rot) tf += ' rotate(' + it.rot + ' ' + r1(it.w / 2) + ' ' + r1(it.h / 2) + ')';
    g.setAttribute('transform', tf);
    var w = it.w, hh = it.h, el;

    if (CLOSED[t]) {
      var common = {
        fill: it.f || 'none', stroke: col, 'stroke-width': sw, 'stroke-dasharray': dashArr(it, sw),
        'stroke-linejoin': 'round', 'stroke-linecap': it.dash === 'dot' ? 'round' : null,
        'pointer-events': ex ? null : 'all'
      };
      if (t === 'rect') {
        el = svgEl('rect', { width: r1(w), height: r1(hh), rx: it.r ? Math.min(it.r, w / 2, hh / 2) : null });
      } else if (t === 'ellipse') {
        el = svgEl('ellipse', { cx: r1(w / 2), cy: r1(hh / 2), rx: r1(w / 2), ry: r1(hh / 2) });
      } else if (t === 'diamond') {
        el = svgEl('polygon', { points: [w / 2, 0, w, hh / 2, w / 2, hh, 0, hh / 2].map(r1).join(' ') });
      } else {
        el = svgEl('polygon', { points: [w / 2, 0, w, hh, 0, hh].map(r1).join(' ') });
      }
      for (var k in common) if (common[k] !== null && common[k] !== undefined) el.setAttribute(k, common[k]);
      g.appendChild(el);
      if (it.txt && !hideText) {
        var sfs = fsOf(it), sl = wrap(it.txt, w - 16, sfs, it.b);
        // Default ink on a filled shape: dark or light, whichever reads on the fill.
        var tcol = it.s || (it.f && it.f !== 'none' ? contrastInk(it.f) : ink);
        textBlock(g, sl, 8, (hh - sl.length * lineH(sfs)) / 2, w - 16, sfs, tcol, alOf(it), it.b);
      }
    } else if (t === 'path') {
      g.appendChild(svgEl('path', {
        d: it.d, fill: 'none', stroke: col, 'stroke-width': sw, 'stroke-linecap': 'round', 'stroke-linejoin': 'round',
        'stroke-opacity': it.hl ? 0.45 : null, 'stroke-dasharray': dashArr(it, sw)
      }));
      if (!ex) hitPath(g, it.d, sw);
    } else if (t === 'text') {
      var lay = textLayout(it);
      if (!ex) hitRect(g, w, lay.h);
      if (!hideText) textBlock(g, lay.lines, 4, 4, w - 8, lay.fs, col, alOf(it), it.b);
    } else if (t === 'sticky') {
      g.appendChild(svgEl('rect', { width: r1(w), height: r1(hh), rx: 4, fill: it.f || STICKY[0], filter: 'url(#qc-shadow)' }));
      if (it.txt && !hideText) {
        var fit = stickyFit(it);
        textBlock(g, fit.lines, 12, (hh - fit.lines.length * lineH(fit.fs)) / 2, w - 24, fit.fs, STICKY_INK, alOf(it), it.b);
      }
    } else if (t === 'emoji') {
      if (!ex) hitRect(g, w, hh);
      var et = svgEl('text', {
        x: r1(w / 2), y: r1(hh / 2), 'font-size': r1(Math.min(w, hh) * 0.82), 'font-family': EMOJI_FONT,
        'text-anchor': 'middle', 'dominant-baseline': 'central'
      });
      et.textContent = it.ch || '?';
      g.appendChild(et);
    } else if (t === 'sticker') {
      if (!ex) hitRect(g, w, hh);
      var sg = svgEl('g', { transform: 'scale(' + r1(w) / 100 + ' ' + r1(hh) / 100 + ')' });
      sg.innerHTML = STICKERS[it.k] || STICKERS.star;
      g.appendChild(sg);
    } else if (t === 'image') {
      g.appendChild(svgEl('image', { href: assetUrl(it.src), width: r1(w), height: r1(hh), preserveAspectRatio: 'none' }));
      if (!ex) hitRect(g, w, hh);
    } else if (t === 'stencil') {
      drawStencil(g, it, ex, hideText);
    } else if (t === 'frame') {
      drawFrame(g, it, ex, hideText);
    }
    return g;
  }

  // ----- rendering --------------------------------------------------------------------

  function itemKey(it) {
    var k = JSON.stringify(it);
    if (it.t === 'conn') k += JSON.stringify(endItem(it.a) || 0) + JSON.stringify(endItem(it.b) || 0);
    if (editing === it.id) k += '#edit';
    return k;
  }

  function render() {
    index();
    var keep = {}, i = 0;
    zOrder(scene.items).forEach(function (it) {
      var key = itemKey(it), c = cache[it.id];
      if (!c || c.key !== key) {
        if (c) c.el.remove();
        c = cache[it.id] = { key: key, el: draw(it, false) };
      }
      keep[it.id] = 1;
      if (world.childNodes[i] !== c.el) world.insertBefore(c.el, world.childNodes[i] || null);
      i++;
    });
    Object.keys(cache).forEach(function (id) {
      if (!keep[id]) { cache[id].el.remove(); delete cache[id]; }
    });
    if (live) world.appendChild(live);
    var empty = !scene.items.length && !drag;
    hint.style.display = empty ? '' : 'none';
    if (empty && !tplHidden && !tplBox.childNodes.length) buildTemplates();
    tplBox.style.display = tplHidden ? 'none' : '';
    renderOverlay();
  }

  function clearCache() {
    Object.keys(cache).forEach(function (id) { cache[id].el.remove(); });
    cache = {};
  }

  function sp(p) { return { x: p.x * view.z + view.x, y: p.y * view.z + view.y }; }

  function toWorld(cx, cy) {
    var r = svg.getBoundingClientRect();
    return { x: (cx - r.left - view.x) / view.z, y: (cy - r.top - view.y) / view.z };
  }

  function viewCenter() {
    var r = svg.getBoundingClientRect();
    return toWorld(r.left + r.width / 2, r.top + r.height / 2);
  }

  var HF = { nw: [0, 0], n: [0.5, 0], ne: [1, 0], e: [1, 0.5], se: [1, 1], s: [0.5, 1], sw: [0, 1], w: [0, 0.5] };
  var HCURSOR = { nw: 'nwse-resize', se: 'nwse-resize', ne: 'nesw-resize', sw: 'nesw-resize', n: 'ns-resize', s: 'ns-resize', e: 'ew-resize', w: 'ew-resize' };

  function polygonOf(pts, cls) {
    return svgEl('polygon', { class: cls, points: pts.map(function (p) { return r1(p.x) + ',' + r1(p.y); }).join(' ') });
  }

  function handle(p, name, round) {
    var el = round
      ? svgEl('circle', { class: 'ov-handle', cx: r1(p.x), cy: r1(p.y), r: 5, 'data-h': name })
      : svgEl('rect', { class: 'ov-handle', x: r1(p.x - 4), y: r1(p.y - 4), width: 8, height: 8, rx: 1.5, 'data-h': name });
    el.style.cursor = HCURSOR[name] || (name === 'rot' ? 'grab' : 'move');
    overlay.appendChild(el);
  }

  function selItems() {
    return sel.map(function (id) { return byId[id]; }).filter(Boolean);
  }

  function renderOverlay() {
    overlay.textContent = '';
    if (drag && drag.kind === 'band') {
      var a = sp(drag.p0), b = sp(drag.p1);
      overlay.appendChild(svgEl('rect', {
        class: 'ov-band', x: Math.min(a.x, b.x), y: Math.min(a.y, b.y), width: Math.abs(a.x - b.x), height: Math.abs(a.y - b.y)
      }));
    }
    var hov = hoverId && byId[hoverId];
    if (hov) {
      overlay.appendChild(polygonOf(corners(hov).map(sp), 'ov-hover'));
      Object.keys(SIDES).forEach(function (s) {
        var q = sp(anchor(hov, s));
        overlay.appendChild(svgEl('circle', { class: 'ov-anchor', cx: r1(q.x), cy: r1(q.y), r: 4 }));
      });
    }
    var items = selItems();
    if (!items.length) return;
    var locked = items.some(function (it) { return it.lock; });
    var moving = drag && drag.kind === 'move' && drag.moved;

    if (items.length === 1 && (items[0].t === 'line' || items[0].t === 'arrow' || items[0].t === 'conn')) {
      var it = items[0], A, B, names;
      if (it.t === 'conn') {
        var r = route(it);
        A = r.A; B = r.B; names = ['a', 'b'];
        var path = svgEl('path', { class: 'ov-item', d: polyD(r.pts.map(sp)) });
        overlay.appendChild(path);
      } else {
        A = { x: it.pts[0], y: it.pts[1] }; B = { x: it.pts[2], y: it.pts[3] }; names = ['p0', 'p1'];
        overlay.appendChild(svgEl('path', { class: 'ov-item', d: polyD([sp(A), sp(B)]) }));
      }
      if (!locked && !moving) {
        handle(sp(A), names[0], true);
        handle(sp(B), names[1], true);
      }
      return;
    }

    if (items.length > 1) {
      items.forEach(function (x) {
        if (x.t === 'conn') overlay.appendChild(svgEl('path', { class: 'ov-item', d: polyD(route(x).pts.map(sp)) }));
        else if (x.t === 'line' || x.t === 'arrow') overlay.appendChild(svgEl('path', { class: 'ov-item', d: polyD([sp({ x: x.pts[0], y: x.pts[1] }), sp({ x: x.pts[2], y: x.pts[3] })]) }));
        else overlay.appendChild(polygonOf(corners(x).map(sp), 'ov-item'));
      });
    }
    var box, rot = 0;
    if (items.length === 1) {
      box = { x: items[0].x, y: items[0].y, w: items[0].w, h: items[0].h };
      if (items[0].t === 'text') box.h = textLayout(items[0]).h;
      rot = items[0].rot || 0;
    } else {
      box = union(items);
    }
    var c = { x: box.x + box.w / 2, y: box.y + box.h / 2 };
    var P = function (fx, fy) { return sp(rotP(box.x + box.w * fx, box.y + box.h * fy, c.x, c.y, rot)); };
    overlay.appendChild(polygonOf([P(0, 0), P(1, 0), P(1, 1), P(0, 1)], 'ov-sel' + (locked ? ' locked' : '')));
    if (locked || moving || editing) return;
    var only = items.length === 1 ? items[0] : null;
    var names2 = only && only.t === 'text' ? ['e', 'w'] : Object.keys(HF);
    names2.forEach(function (n) { handle(P(HF[n][0], HF[n][1]), n); });
    if (items.some(function (x) { return x.t === 'frame'; })) return;
    var top = P(0.5, 0), dir = rotP(0, -1, 0, 0, rot);
    var q = { x: top.x + dir.x * 24, y: top.y + dir.y * 24 };
    overlay.appendChild(svgEl('line', { class: 'ov-sel', x1: r1(top.x), y1: r1(top.y), x2: r1(q.x), y2: r1(q.y) }));
    handle(q, 'rot', true);
  }

  function applyView() {
    world.setAttribute('transform', 'translate(' + r1(view.x) + ' ' + r1(view.y) + ') scale(' + view.z + ')');
    var step = 20 * view.z;
    while (step < 12) step *= 5;
    pattern.setAttribute('width', step);
    pattern.setAttribute('height', step);
    pattern.setAttribute('x', r1(view.x));
    pattern.setAttribute('y', r1(view.y));
    patternDot.setAttribute('cx', 0);
    patternDot.setAttribute('cy', 0);
    patternDot.setAttribute('r', view.z < 0.5 ? 0.8 : 1);
    gridRect.style.opacity = view.z < 0.25 ? 0.5 : 1;
    if (zoomLabel) zoomLabel.textContent = Math.round(view.z * 100) + '%';
    renderOverlay();
    placeEditor();
  }

  function zoomAt(z, sx, sy) {
    z = Math.max(ZMIN, Math.min(ZMAX, z));
    var wx = (sx - view.x) / view.z, wy = (sy - view.y) / view.z;
    view.z = z;
    view.x = sx - wx * z;
    view.y = sy - wy * z;
    applyView();
  }

  function zoomCentered(z) {
    var r = svg.getBoundingClientRect();
    zoomAt(z, r.width / 2, r.height / 2);
  }

  function fit(maxZ) {
    var r = svg.getBoundingClientRect(), W = r.width || 800, H = r.height || 600;
    if (!scene.items.length) { view = { x: Math.round(W / 2), y: Math.round(H / 2), z: 1 }; return; }
    var bb = contentBox();
    var z = Math.min((W - 140) / Math.max(bb.w, 1), (H - 120) / Math.max(bb.h, 1), maxZ || 1);
    z = Math.max(ZMIN, Math.min(ZMAX, z));
    view = { x: W / 2 - (bb.x + bb.w / 2) * z, y: H / 2 - (bb.y + bb.h / 2) * z, z: z };
  }

  // ----- model changes ----------------------------------------------------------------

  function contentKey() { return JSON.stringify([scene.title || '', scene.items]); }

  function normalize(it) {
    if (it.t === 'line' || it.t === 'arrow') {
      it.pts = it.pts.map(r1);
      var b = aabb(it);
      it.x = r1(b.x); it.y = r1(b.y); it.w = r1(b.w); it.h = r1(b.h);
      return;
    }
    if (it.t === 'conn') {
      delete it.x; delete it.y; delete it.w; delete it.h;
      ['a', 'b'].forEach(function (k) {
        var e = it[k];
        if (e && !e.id) { e.x = r1(e.x || 0); e.y = r1(e.y || 0); }
      });
      return;
    }
    if (it.t === 'text') it.h = textLayout(it).h;
    if (it.t === 'frame') delete it.rot;
    it.x = r1(it.x); it.y = r1(it.y); it.w = r1(Math.max(1, it.w)); it.h = r1(Math.max(1, it.h));
    if (it.rot) {
      var r = r1(((it.rot % 360) + 540) % 360 - 180);
      if (Math.abs(r) < 0.5) delete it.rot; else it.rot = r;
    } else delete it.rot;
  }

  function commit() {
    scene.items = zOrder(scene.items);
    index();
    scene.items.forEach(normalize);
    scene.canvas = 1;
    scene.view = { x: r1(view.x), y: r1(view.y), z: Math.round(view.z * 1000) / 1000 };
    var ck = contentKey();
    if (ck !== lastContent) {
      lastContent = ck;
      lastJSON = JSON.stringify(scene);
      post({ type: 'canvas', scene: scene });
    }
    render();
    refreshStyleBar();
  }

  function flush() {
    if (!drag && !editing && pending) {
      var p = pending;
      pending = null;
      window.QC.update(p);
    }
  }

  function groupOf(id) {
    var it = byId[id];
    if (!it || !it.g) return [id];
    return scene.items.filter(function (x) { return x.g === it.g; }).map(function (x) { return x.id; });
  }

  function setSel(ids) {
    var seen = {};
    sel = ids.filter(function (id) { if (seen[id] || !byId[id]) return false; seen[id] = 1; return true; });
    renderOverlay();
    refreshStyleBar();
  }

  function deleteIds(ids, force) {
    var gone = {};
    ids.forEach(function (id) { var it = byId[id]; if (it && (force || !it.lock)) gone[id] = 1; });
    scene.items.forEach(function (it) {
      if (it.t === 'conn' && ((it.a && gone[it.a.id]) || (it.b && gone[it.b.id]))) gone[it.id] = 1;
    });
    scene.items = scene.items.filter(function (it) { return !gone[it.id]; });
    sel = sel.filter(function (id) { return !gone[id]; });
    index();
    return Object.keys(gone).length;
  }

  function moveItem(it, o, dx, dy) {
    if (it.t === 'line' || it.t === 'arrow') {
      it.pts = [o.pts[0] + dx, o.pts[1] + dy, o.pts[2] + dx, o.pts[3] + dy];
      return;
    }
    if (it.t === 'conn') {
      ['a', 'b'].forEach(function (k) {
        if (o[k] && !o[k].id) it[k] = { x: o[k].x + dx, y: o[k].y + dy };
      });
      return;
    }
    it.x = o.x + dx;
    it.y = o.y + dy;
  }

  function scaleD(d, sx, sy) {
    return d.replace(/(-?\d*\.?\d+)\s+(-?\d*\.?\d+)/g, function (m, a, b) {
      return r1(parseFloat(a) * sx) + ' ' + r1(parseFloat(b) * sy);
    });
  }

  function copyPayload() {
    var items = selWithContents();
    var ids = {};
    items.forEach(function (it) { ids[it.id] = 1; });
    scene.items.forEach(function (it) {
      if (it.t === 'conn' && !ids[it.id] && it.a && it.b && ids[it.a.id] && ids[it.b.id]) items.push(it);
    });
    return { from: CID, items: clone(items) };
  }

  /// Adds copies of `items` with new ids, centred at `at` (or moved by `offset`).
  function insertClones(items, at, offset) {
    if (!items || !items.length) return;
    var map = {}, gmap = {};
    var out = clone(items);
    out.forEach(function (it) { map[it.id] = rid(); byId[map[it.id]] = 1; });
    out.forEach(function (it) {
      it.id = map[it.id];
      if (it.g) { gmap[it.g] = gmap[it.g] || rid(); it.g = gmap[it.g]; }
      if (it.t === 'conn') {
        ['a', 'b'].forEach(function (k) {
          var e = it[k];
          if (e && e.id) {
            if (map[e.id]) e.id = map[e.id];
            else it[k] = { x: 0, y: 0 };
          }
        });
      }
    });
    out.forEach(function (it) { scene.items.push(it); });
    index();
    var dx, dy;
    if (offset) { dx = dy = offset; } else {
      var bb = union(out);
      dx = at.x - (bb.x + bb.w / 2);
      dy = at.y - (bb.y + bb.h / 2);
      if (snapOn) { dx = snap(bb.x + dx) - bb.x; dy = snap(bb.y + dy) - bb.y; }
    }
    out.forEach(function (it) { moveItem(it, clone(it), dx, dy); });
    setSel(out.map(function (it) { return it.id; }));
    commit();
  }

  function arrange(mode) {
    var s = {};
    sel.forEach(function (id) { s[id] = 1; });
    var items = scene.items.slice(), i, tmp;
    if (mode === 'front') items = items.filter(function (it) { return !s[it.id]; }).concat(items.filter(function (it) { return s[it.id]; }));
    else if (mode === 'back') items = items.filter(function (it) { return s[it.id]; }).concat(items.filter(function (it) { return !s[it.id]; }));
    else if (mode === 'fwd') {
      for (i = items.length - 2; i >= 0; i--) if (s[items[i].id] && !s[items[i + 1].id]) { tmp = items[i]; items[i] = items[i + 1]; items[i + 1] = tmp; }
    } else {
      for (i = 1; i < items.length; i++) if (s[items[i].id] && !s[items[i - 1].id]) { tmp = items[i]; items[i] = items[i - 1]; items[i - 1] = tmp; }
    }
    scene.items = items;
    commit();
  }

  function group(on) {
    var items = selItems();
    if (on) {
      if (items.length < 2) return;
      var g = rid();
      items.forEach(function (it) { it.g = g; });
    } else {
      items.forEach(function (it) { delete it.g; });
    }
    commit();
  }

  function nudge(dx, dy) {
    var moves = selItems().filter(function (it) { return !it.lock; }).map(function (it) { return [it, dx, dy]; });
    if (!moves.length) return;
    shiftAll(moves);
    commit();
  }

  function addItem(it) {
    scene.items.push(it);
    index();
    return it;
  }

  function newItem(t, cat, extra) {
    var it = { id: rid(), t: t };
    var st = toolStyle[cat] || {};
    for (var k in st) it[k] = clone(st[k]);
    for (var j in extra) it[j] = extra[j];
    return it;
  }

  function insertAtCenter(it) {
    var c = viewCenter();
    it.x = c.x - it.w / 2;
    it.y = c.y - it.h / 2;
    if (snapOn) { it.x = snap(it.x); it.y = snap(it.y); }
    addItem(it);
    setTool('select');
    setSel([it.id]);
    commit();
  }

  // ----- text editing --------------------------------------------------------------------

  function startEdit(id) {
    var it = byId[id];
    if (!it || it.lock || !hasText(it)) return;
    if (editing) finishEdit();
    editing = id;
    if (sel.length !== 1 || sel[0] !== id) sel = [id];
    render();
    ta = h('textarea', { class: 'editor' + (it.t === 'conn' ? ' label' : ''), spellcheck: 'true' });
    ta.value = it.txt || '';
    app.appendChild(ta);
    ta.addEventListener('keydown', function (e) {
      e.stopPropagation();
      if (e.key === 'Escape' || (e.key === 'Enter' && (e.ctrlKey || e.metaKey))) { e.preventDefault(); finishEdit(); }
    });
    ta.addEventListener('input', placeEditor);
    ta.addEventListener('blur', finishEdit);
    ta.addEventListener('pointerdown', function (e) { e.stopPropagation(); });
    placeEditor();
    ta.focus();
    ta.setSelectionRange(ta.value.length, ta.value.length);
    refreshStyleBar();
  }

  /// Puts the text editor over the item's text, laid out like the drawn text.
  function placeEditor() {
    if (!editing || !ta) return;
    var it = byId[editing];
    if (!it) return;
    var z = view.z, st = ta.style, fs = fsOf(it), bold = !!it.b, al = alOf(it), va = 'c';
    var col = it.s || ink, box, lines = 1, raw = false;
    if (it.t === 'conn') {
      var m = midOf(route(it));
      box = { x: m.x - 100, y: m.y - lineH(fs) / 2 - 4 / z, w: 200, h: null };
      al = 'c';
    } else if (it.t === 'text') {
      box = { x: it.x + 4, y: it.y + 4, w: it.w - 8, h: null };
    } else if (it.t === 'frame') {
      box = { x: it.x, y: it.y - fs * 1.7, w: it.w, h: null };
      bold = true;
    } else if (it.t === 'sticky') {
      var fit = stickyFit({ w: it.w, h: it.h, txt: ta.value, fs: it.fs, b: it.b, t: 'sticky' });
      fs = fit.fs;
      lines = fit.lines.length;
      box = { x: it.x + 12, y: it.y + 12, w: it.w - 24, h: it.h - 24 };
      col = STICKY_INK;
    } else if (it.t === 'stencil') {
      var def = stDef(it);
      if (def.text === 'lines') {
        raw = true;
        fs = Math.max(12, Math.min(fs, 16));
        box = { x: it.x, y: it.y, w: Math.max(it.w, 180 / z), h: null };
        al = 'l';
      } else {
        var tb = stencilTB(it);
        bold = it.b !== undefined ? !!it.b : !!def.b;
        va = def.va || 'c';
        lines = wrap(ta.value, tb.w, fs, bold).length;
        box = { x: it.x + tb.x, y: it.y + tb.y, w: tb.w, h: tb.h };
        col = stencilLabelCol(it);
      }
    } else {
      lines = wrap(ta.value, it.w - 16, fs, bold).length;
      box = { x: it.x + 8, y: it.y, w: it.w - 16, h: it.h };
      col = it.s || (it.f && it.f !== 'none' ? contrastInk(it.f) : ink);
    }
    var p = sp({ x: box.x, y: box.y });
    ta.classList.toggle('raw', raw);
    st.left = p.x + 'px';
    st.top = p.y + 'px';
    st.width = Math.max(box.w * z, 40) + 'px';
    st.fontSize = (fs * z) + 'px';
    st.fontWeight = bold ? '700' : '400';
    st.color = raw ? '' : col;
    st.textAlign = { l: 'left', c: 'center', r: 'right' }[al];
    st.paddingLeft = st.paddingRight = raw ? '6px' : '0px';
    st.paddingBottom = '0px';
    if (box.h === null) {
      st.paddingTop = raw ? '4px' : '0px';
      st.height = 'auto';
      st.height = ta.scrollHeight + 'px';
    } else {
      var hh = box.h * z, content = lines * lineH(fs) * z;
      st.height = Math.max(hh, content) + 'px';
      st.paddingTop = (va === 't' ? 0 : Math.max(0, (hh - content) / 2)) + 'px';
    }
    if (it.rot && it.t !== 'conn' && !raw) {
      var c = sp(center(it));
      st.transformOrigin = (c.x - p.x) + 'px ' + (c.y - p.y) + 'px';
      st.transform = 'rotate(' + it.rot + 'deg)';
    } else {
      st.transform = '';
    }
  }

  function finishEdit() {
    if (!editing) return;
    var id = editing, val = ta ? ta.value : '';
    editing = null;
    if (ta) { var t = ta; ta = null; t.remove(); }
    var it = byId[id];
    if (it) {
      if (val.replace(/\s+$/, '')) it.txt = val.replace(/\s+$/, ''); else delete it.txt;
      if (it.t === 'text' && !it.txt) deleteIds([id], true);
    }
    commit();
    flush();
  }

  // ----- popovers ----------------------------------------------------------------------

  var pop = null, popAnchor = null;

  function closePop() {
    if (pop) { pop.remove(); pop = null; popAnchor = null; }
  }

  function openPop(anchorEl, build, cls) {
    closePop();
    pop = h('div', { class: 'pop panel' + (cls ? ' ' + cls : '') });
    popAnchor = anchorEl;
    pop.addEventListener('pointerdown', function (e) { e.stopPropagation(); });
    build(pop);
    app.appendChild(pop);
    var r = anchorEl.getBoundingClientRect(), pw = pop.offsetWidth, ph = pop.offsetHeight;
    var left, top;
    if (toolsBox.contains(anchorEl)) { left = r.right + 8; top = r.top; } else { left = r.left; top = r.bottom + 6; }
    left = Math.max(6, Math.min(left, window.innerWidth - pw - 6));
    top = Math.max(6, Math.min(top, window.innerHeight - ph - 6));
    pop.style.left = left + 'px';
    pop.style.top = top + 'px';
  }

  document.addEventListener('pointerdown', function (e) {
    if (pop && !pop.contains(e.target) && !(popAnchor && popAnchor.contains(e.target))) closePop();
  }, true);

  function colorPop(anchorEl, key, current, colors, extra) {
    openPop(anchorEl, function (p) {
      var grid = h('div', { class: 'swatches' });
      function sw(val, cls, title, bg) {
        var b = h('button', { class: 'sw' + (cls ? ' ' + cls : '') + (val === current ? ' cur' : ''), type: 'button', title: title || val });
        if (bg) b.style.background = bg;
        b.addEventListener('click', function () { setStyle(key, val); closePop(); });
        grid.appendChild(b);
      }
      if (extra === 'auto') sw(null, 'auto', S('auto', 'Automatic'));
      if (extra === 'none') sw(null, 'none', S('none', 'None'));
      colors.forEach(function (c) { sw(c, '', c, c); });
      p.appendChild(grid);
      var input = h('input', { type: 'color', value: /^#[0-9a-f]{6}$/i.test(current || '') ? current : '#3584e4' });
      input.addEventListener('change', function () { setStyle(key, input.value); closePop(); });
      p.appendChild(h('label', { class: 'custom' }, input, S('custom', 'Custom colour')));
    });
  }

  function emojiPop(anchorEl) {
    var tab = 'emoji';
    openPop(anchorEl, function (p) {
      var tabs = h('div', { class: 'tabs' });
      var bE = h('button', { type: 'button', class: 'on' }, S('emojiTab', 'Emoji'));
      var bS = h('button', { type: 'button' }, S('stickers', 'Stickers'));
      tabs.append(bE, bS);
      var search = h('input', { class: 'search', type: 'search', placeholder: S('search', 'Search') });
      var list = h('div', { class: 'list' });
      p.append(tabs, search, list);
      search.addEventListener('keydown', function (e) { e.stopPropagation(); if (e.key === 'Escape') closePop(); });
      function fill() {
        list.textContent = '';
        var q = search.value.trim().toLowerCase();
        search.style.display = tab === 'emoji' ? '' : 'none';
        if (tab === 'stickers') {
          var g = h('div', { class: 'grid stickers' });
          Object.keys(STICKERS).forEach(function (k) {
            var b = h('button', { class: 'e', type: 'button', title: stickerName(k) });
            b.innerHTML = '<svg viewBox="0 0 100 100" width="48" height="48">' + STICKERS[k] + '</svg>';
            b.addEventListener('click', function () {
              closePop();
              insertAtCenter(newItem('sticker', null, { k: k, w: 96, h: 96 }));
            });
            g.appendChild(b);
          });
          list.appendChild(g);
          return;
        }
        var any = false;
        EMOJI.forEach(function (cat) {
          var items = cat.list.filter(function (e) { return !q || e.words.indexOf(q) >= 0 || e.ch === q; });
          if (!items.length) return;
          any = true;
          list.appendChild(h('h4', null, S(cat.key, cat.name)));
          var g = h('div', { class: 'grid' });
          items.forEach(function (e) {
            var b = h('button', { class: 'e', type: 'button', title: e.words }, e.ch);
            b.addEventListener('click', function () {
              closePop();
              insertAtCenter(newItem('emoji', null, { ch: e.ch, w: 64, h: 64 }));
            });
            g.appendChild(b);
          });
          list.appendChild(g);
        });
        if (!any) list.appendChild(h('div', { class: 'empty' }, S('noResults', 'Nothing found')));
      }
      bE.addEventListener('click', function () { tab = 'emoji'; bE.classList.add('on'); bS.classList.remove('on'); fill(); search.focus(); });
      bS.addEventListener('click', function () { tab = 'stickers'; bS.classList.add('on'); bE.classList.remove('on'); fill(); });
      search.addEventListener('input', fill);
      fill();
      setTimeout(function () { search.focus(); }, 0);
    }, 'emojipop');
  }

  // ----- shape library -----------------------------------------------------------------

  function iconSpan(name) {
    var s = h('span', { class: 'ico' });
    s.innerHTML = icon(name);
    return s;
  }

  function stencilPreview(k) {
    var def = STENCILS[k], pad = Math.max(def.w, def.h) * 0.06 + 2;
    var s = svgEl('svg', { viewBox: [-pad, -pad, def.w + 2 * pad, def.h + 2 * pad].map(r1).join(' '), width: 44, height: 36 });
    s.appendChild(draw({ id: 'pv', t: 'stencil', k: k, x: 0, y: 0, w: def.w, h: def.h }, true));
    return s;
  }

  function framePreview(w, hh) {
    var s = svgEl('svg', { viewBox: [-8, -8, w + 16, hh + 16].join(' '), width: 44, height: 36 });
    s.appendChild(svgEl('rect', { width: w, height: hh, rx: 3, fill: 'currentColor', 'fill-opacity': 0.06, stroke: 'currentColor', 'stroke-opacity': 0.6 }));
    return s;
  }

  function addStencil(k) {
    var def = STENCILS[k];
    var it = newItem('stencil', null, { k: k, w: def.w, h: def.h });
    insertAtCenter(it);
    ensureVisible(it);
  }

  function addFrame(f) {
    var it = newItem('frame', 'frame', { w: f[2], h: f[3], txt: S(f[0], f[1]) });
    insertAtCenter(it);
    ensureVisible(it);
  }

  function libraryPop(anchorEl) {
    openPop(anchorEl, function (p) {
      var search = h('input', { class: 'search', type: 'search', placeholder: S('search', 'Search') });
      var list = h('div', { class: 'list' });
      p.append(search, list);
      search.addEventListener('keydown', function (e) { e.stopPropagation(); if (e.key === 'Escape') closePop(); });
      function tile(name, art, onClick, on) {
        var b = h('button', { class: 'tile' + (on ? ' on' : ''), type: 'button', title: name });
        b.appendChild(art);
        b.addEventListener('click', onClick);
        return b;
      }
      function section(title, tiles) {
        if (!tiles.length) return;
        list.appendChild(h('h4', null, title));
        var g = h('div', { class: 'grid tiles' });
        tiles.forEach(function (t) { g.appendChild(t); });
        list.appendChild(g);
      }
      function fill() {
        list.textContent = '';
        var q = search.value.trim().toLowerCase();
        // Translated and English names both match.
        var match = function () {
          if (!q) return true;
          for (var i = 0; i < arguments.length; i++) if (arguments[i] && String(arguments[i]).toLowerCase().indexOf(q) >= 0) return true;
          return false;
        };
        var basic = SHAPES.filter(function (s) { return match(SHAPE_NAMES[s], s); }).map(function (s) {
          return tile(tip(SHAPE_NAMES[s], SHAPE_KEYS[s]), iconSpan(s), function () { setTool(s); closePop(); }, s === tool);
        });
        if (match(S('frame', 'Frame'), 'Frame')) {
          basic.push(tile(tip(S('frame', 'Frame'), 'F'), iconSpan('frame'), function () { setTool('frame'); closePop(); }, tool === 'frame'));
        }
        section(S('catBasic', 'Basic shapes'), basic);
        section(S('catFrames', 'Frames'), FRAME_PRESETS.filter(function (f) { return match(S(f[0], f[1]), f[1]); }).map(function (f) {
          return tile(S(f[0], f[1]) + ' · ' + f[2] + ' × ' + f[3], framePreview(f[2], f[3]), function () { closePop(); addFrame(f); });
        }));
        STENCIL_CATS.forEach(function (c) {
          section(S(c[1], c[2]), Object.keys(STENCILS).filter(function (k) { return STENCILS[k].c === c[0] && match(stName(k), STENCILS[k].n, k); }).map(function (k) {
            return tile(stName(k), stencilPreview(k), function () { closePop(); addStencil(k); });
          }));
        });
        if (!list.childNodes.length) list.appendChild(h('div', { class: 'empty' }, S('noResults', 'Nothing found')));
      }
      search.addEventListener('input', fill);
      fill();
      setTimeout(function () { search.focus(); }, 0);
    }, 'libpop');
  }

  // ----- align and distribute ----------------------------------------------------------

  function alignable() { return selItems().filter(function (it) { return it.t !== 'conn'; }); }

  function alignSel(mode) {
    var items = alignable();
    if (items.length < 2) return;
    var bx = items.map(bodyBox), moves = [];
    var x0 = Math.min.apply(null, bx.map(function (b) { return b.x; })), x1 = Math.max.apply(null, bx.map(function (b) { return b.x + b.w; }));
    var y0 = Math.min.apply(null, bx.map(function (b) { return b.y; })), y1 = Math.max.apply(null, bx.map(function (b) { return b.y + b.h; }));
    if (mode === 'dh' || mode === 'dv') {
      if (items.length < 3) return;
      var hz = mode === 'dh';
      var idx = items.map(function (it, i) { return i; }).sort(function (a, b) {
        return hz ? bx[a].x + bx[a].w / 2 - bx[b].x - bx[b].w / 2 : bx[a].y + bx[a].h / 2 - bx[b].y - bx[b].h / 2;
      });
      var first = bx[idx[0]], last = bx[idx[idx.length - 1]];
      var span = hz ? last.x + last.w - first.x : last.y + last.h - first.y;
      var sum = idx.reduce(function (s, i) { return s + (hz ? bx[i].w : bx[i].h); }, 0);
      var gap = (span - sum) / (idx.length - 1), pos = hz ? first.x : first.y;
      idx.forEach(function (i) {
        var d = pos - (hz ? bx[i].x : bx[i].y);
        if (!items[i].lock) moves.push([items[i], hz ? d : 0, hz ? 0 : d]);
        pos += (hz ? bx[i].w : bx[i].h) + gap;
      });
    } else {
      items.forEach(function (it, i) {
        var b = bx[i], dx = 0, dy = 0;
        if (mode === 'l') dx = x0 - b.x;
        else if (mode === 'c') dx = (x0 + x1) / 2 - (b.x + b.w / 2);
        else if (mode === 'r') dx = x1 - (b.x + b.w);
        else if (mode === 't') dy = y0 - b.y;
        else if (mode === 'm') dy = (y0 + y1) / 2 - (b.y + b.h / 2);
        else if (mode === 'b') dy = y1 - (b.y + b.h);
        if (!it.lock) moves.push([it, dx, dy]);
      });
    }
    shiftAll(moves);
    commit();
  }

  function sameSize(key) {
    var items = alignable().filter(function (it) {
      return !it.lock && it.t !== 'line' && it.t !== 'arrow' && !(it.t === 'text' && key === 'h');
    });
    if (items.length < 2) return;
    var target = Math.max.apply(null, items.map(function (it) { return it[key]; }));
    items.forEach(function (it) {
      var o = clone(it), f = target / (o[key] || 1), other = key === 'w' ? 'h' : 'w';
      it[key] = target;
      if (it.t === 'image' || it.t === 'emoji' || it.t === 'sticker') it[other] = o[other] * f;
      if (it.t === 'path') it.d = scaleD(o.d, it.w / o.w, it.h / o.h);
    });
    commit();
  }

  function alignPop(anchorEl) {
    var n = alignable().length;
    openPop(anchorEl, function (p) {
      [
        [['l', 'aL', S('alignLefts', 'Align left edges')], ['c', 'aC', S('alignCentres', 'Align centres')], ['r', 'aR', S('alignRights', 'Align right edges')]],
        [['t', 'aT', S('alignTops', 'Align top edges')], ['m', 'aM', S('alignMiddles', 'Align middles')], ['b', 'aB', S('alignBottoms', 'Align bottom edges')]],
        [['dh', 'dH', S('distH', 'Distribute horizontally')], ['dv', 'dV', S('distV', 'Distribute vertically')]],
        [['sw', 'sW', S('sameW', 'Same width')], ['sh', 'sH', S('sameH', 'Same height')]]
      ].forEach(function (row) {
        var r = h('div', { class: 'row' });
        row.forEach(function (o) {
          var b = ibtn(icon(o[1]), o[2], function () {
            if (o[0] === 'sw') sameSize('w'); else if (o[0] === 'sh') sameSize('h'); else alignSel(o[0]);
          });
          if ((o[0] === 'dh' || o[0] === 'dv') && n < 3) b.disabled = true;
          r.appendChild(b);
        });
        p.appendChild(r);
      });
    }, 'alignpop');
  }

  // ----- templates -------------------------------------------------------------------

  var tplHidden = false;

  function tItem(t, x, y, w, hh, ex) {
    var it = { id: rid(), t: t, x: x, y: y, w: w, h: hh };
    for (var k in ex) it[k] = ex[k];
    return it;
  }
  function tSt(k, x, y, w, hh, txt) { return tItem('stencil', x, y, w, hh, txt ? { k: k, txt: txt } : { k: k }); }
  function tConn(a, b, ex) {
    var c = { id: rid(), t: 'conn', a: { id: a.id, p: 'c' }, b: { id: b.id, p: 'c' } };
    for (var k in ex) c[k] = ex[k];
    return c;
  }

  var TEMPLATES = [
    ['tplBlank', 'Blank', null],
    ['tplBrainstorm', 'Brainstorm', function () {
      var c = tItem('ellipse', -130, -70, 260, 140, { txt: S('tCentral', 'Central topic'), fs: 24, f: '#c5e3ff' }), out = [c];
      for (var i = 0; i < 6; i++) {
        var a = i / 6 * Math.PI * 2 - Math.PI / 2;
        var s = tItem('sticky', Math.round(Math.cos(a) * 380) - 80, Math.round(Math.sin(a) * 270) - 80, 160, 160, { f: STICKY[i % STICKY.length], txt: S('tIdea', 'Idea') });
        out.push(s, tConn(c, s, { k: 'curve', ends: ['none', 'none'] }));
      }
      return out;
    }],
    ['tplRetro', 'Retrospective', function () {
      var out = [];
      [['tWell', 'Went well', '#c8f7c5'], ['tImprove', 'To improve', '#ffc8dd'], ['tActions', 'Actions', '#c5e3ff']].forEach(function (n, i) {
        out.push(tItem('frame', i * 400, 0, 360, 480, { txt: S(n[0], n[1]) }), tItem('sticky', i * 400 + 24, 24, 150, 150, { f: n[2] }));
      });
      return out;
    }],
    ['tplFlow', 'Flowchart', function () {
      var a = tSt('terminator', 0, 0, 160, 60, S('tStart', 'Start'));
      var b = tSt('process', 0, 120, 160, 80, S('tStep', 'Step'));
      var c = tSt('decision', -30, 255, 220, 120, S('tDecision', 'Decision?'));
      var d = tSt('process', 260, 275, 160, 80, S('tStep', 'Step'));
      var e = tSt('terminator', 0, 450, 160, 60, S('tEnd', 'End'));
      return [a, b, c, d, e, tConn(a, b), tConn(b, c), tConn(c, e, { txt: S('tYes', 'Yes') }),
        tConn(c, d, { txt: S('tNo', 'No') }), tConn(d, e, { k: 'elbow' })];
    }],
    ['tplWire', 'Wireframe', function () {
      return [
        tSt('browser', 0, 0, 960, 620, 'https://example.com'), tSt('navbar', 0, 40, 960, 56),
        tSt('heading', 48, 140, 480, 48), tSt('paragraph', 48, 210, 500, 110),
        tSt('primary', 48, 350, 150, 44), tSt('button', 214, 350, 130, 44), tSt('image', 600, 140, 312, 220),
        tSt('card', 48, 420, 260, 170), tSt('card', 350, 420, 260, 170), tSt('card', 652, 420, 260, 170)
      ];
    }],
    ['tplMobile', 'Mobile screen', function () {
      var out = [tSt('phone', 0, 0, 300, 620), tSt('statusbar', 16, 18, 268, 24), tSt('appbar', 10, 50, 280, 56, S('tTitle', 'Title'))];
      for (var i = 0; i < 4; i++) out.push(tSt('listitem', 10, 106 + i * 64, 280, 64));
      out.push(tSt('fab', 226, 466, 52, 52), tSt('bottomnav', 10, 540, 280, 64));
      return out;
    }],
    ['tplSwot', 'SWOT analysis', function () {
      var out = [];
      [['tS', 'Strengths', '#c8f7c5'], ['tW', 'Weaknesses', '#ffc8dd'], ['tO', 'Opportunities', '#c5e3ff'], ['tT', 'Threats', '#ffd8a8']].forEach(function (n, i) {
        var x = (i % 2) * 440, y = Math.floor(i / 2) * 360;
        out.push(tItem('frame', x, y, 400, 310, { txt: S(n[0], n[1]) }), tItem('sticky', x + 24, y + 24, 150, 150, { f: n[2] }));
      });
      return out;
    }],
    ['tplWeek', 'Weekly plan', function () {
      return [['mon', 'Monday'], ['tue', 'Tuesday'], ['wed', 'Wednesday'], ['thu', 'Thursday'], ['fri', 'Friday'], ['sat', 'Saturday'], ['sun', 'Sunday']]
        .map(function (d, i) { return tItem('frame', i * 240, 0, 220, 420, { txt: S(d[0], d[1]) }); });
    }]
  ];

  /// Runs `fn` with `byId` holding `items` (for connectors of items that are
  /// not part of the scene).
  function withIndex(items, fn) {
    var saved = byId;
    byId = idSet([]);
    items.forEach(function (it) { byId[it.id] = it; });
    try { return fn(); } finally { byId = saved; }
  }

  function templatePreview(items) {
    return withIndex(items, function () {
      var bb = union(items), pad = Math.max(bb.w, bb.h) * 0.04;
      var s = svgEl('svg', { viewBox: [bb.x - pad, bb.y - pad, bb.w + 2 * pad, bb.h + 2 * pad].map(r1).join(' '), width: 112, height: 72 });
      zOrder(items).forEach(function (it) { s.appendChild(draw(it, true)); });
      return s;
    });
  }

  function applyTemplate(build) {
    var items = build();
    var bb = withIndex(items, function () { return union(items); });
    var c = viewCenter(), dx = snap(c.x - (bb.x + bb.w / 2)), dy = snap(c.y - (bb.y + bb.h / 2));
    items.forEach(function (it) { moveItem(it, clone(it), dx, dy); scene.items.push(it); });
    index();
    setTool('select');
    setSel([]);
    fit(1);
    applyView();
    commit();
  }

  function buildTemplates() {
    tplBox.textContent = '';
    tplBox.appendChild(h('div', { class: 'tpl-title' }, S('templates', 'Start with a template')));
    var grid = h('div', { class: 'tpl-grid' });
    TEMPLATES.forEach(function (t) {
      var b = h('button', { class: 'tpl', type: 'button' });
      if (t[2]) b.appendChild(templatePreview(t[2]()));
      else b.appendChild(h('span', { class: 'tpl-blank' }));
      b.appendChild(h('span', { class: 'tpl-name' }, S(t[0], t[1])));
      b.addEventListener('click', function () {
        if (t[2]) applyTemplate(t[2]);
        else { tplHidden = true; render(); }
      });
      grid.appendChild(b);
    });
    tplBox.appendChild(grid);
  }

  // ----- style bar ----------------------------------------------------------------------

  function catOfItem(it) {
    if (it.t === 'path') return it.hl ? 'hl' : 'pen';
    if (CLOSED[it.t]) return 'shape';
    return { line: 'line', arrow: 'arrow', text: 'text', sticky: 'sticky', conn: 'conn' }[it.t] || null;
  }

  function styleTargets() {
    if (sel.length) return { items: selItems(), cat: null };
    var cat = TOOL_CAT[tool];
    if (!cat) return null;
    var proto = { pen: { t: 'path' }, hl: { t: 'path', hl: true }, shape: { t: shapeKind }, line: { t: 'line' }, arrow: { t: 'arrow' }, text: { t: 'text' }, sticky: { t: 'sticky' }, conn: { t: 'conn' }, frame: { t: 'frame' } }[cat];
    var st = toolStyle[cat];
    for (var k in st) proto[k] = st[k];
    return { items: [proto], cat: cat };
  }

  function caps(it) {
    var t = it.t, lineish = t === 'line' || t === 'arrow' || t === 'conn';
    if (t === 'frame') return { stroke: true, fill: true, dash: true, font: true };
    if (t === 'stencil') {
      var def = stDef(it);
      return {
        stroke: true, fill: def.fill !== false, width: def.fill !== false, dash: def.fill !== false,
        font: def.text !== false, align: def.text === undefined || def.text === true || def.text === 'own'
      };
    }
    return {
      stroke: t !== 'sticky' && t !== 'emoji' && t !== 'sticker' && t !== 'image',
      fill: !!CLOSED[t],
      sticky: t === 'sticky',
      width: !!CLOSED[t] || lineish || t === 'path',
      dash: !!CLOSED[t] || lineish,
      font: !!TEXTUAL[t],
      align: !!CLOSED[t] || t === 'text' || t === 'sticky',
      ends: lineish,
      kind: t === 'conn',
      radius: t === 'rect'
    };
  }

  function setStyle(key, val) {
    var ae = document.activeElement;
    if (ae && (ae.tagName === 'SELECT' || (ae.tagName === 'INPUT' && ae.type === 'range'))) ae.blur();
    var tg = styleTargets();
    if (!tg) return;
    if (tg.cat) {
      if (val === null || val === undefined) delete toolStyle[tg.cat][key]; else toolStyle[tg.cat][key] = val;
      refreshStyleBar();
      return;
    }
    tg.items.forEach(function (it) {
      if (it.lock && key !== 'lock') return;
      if (val === null || val === undefined) delete it[key]; else it[key] = clone(val);
      var cat = catOfItem(it);
      if (cat && REMEMBER[key]) {
        if (val === null || val === undefined) delete toolStyle[cat][key]; else toolStyle[cat][key] = clone(val);
      }
    });
    commit();
    if (editing) placeEditor();
  }

  function refreshStyleBar() {
    styleBar.textContent = '';
    var tg = styleTargets();
    if (!tg || !tg.items.length) { styleBar.style.display = 'none'; return; }
    var items = tg.items, first = items[0];
    var c = {};
    items.forEach(function (it) {
      var k = caps(it);
      for (var n in k) c[n] = c[n] || k[n];
    });
    var locked = !tg.cat && items.some(function (it) { return it.lock; });
    var groups = [];
    function grp() { var g = []; groups.push(g); return g; }
    function val(key) {
      for (var i = 0; i < items.length; i++) if (items[i][key] !== undefined) return items[i][key];
      return undefined;
    }
    function find(pred) { for (var i = 0; i < items.length; i++) if (pred(items[i])) return items[i]; return first; }

    if (!locked) {
      var g1 = grp();
      if (c.stroke) {
        var cur = val('s');
        var b = ibtn('<span class="swatch-btn" style="background:' + (cur || ink) + '"></span>', S('color', 'Colour'), function (e, el) {
          colorPop(el, 's', cur, COLORS, 'auto');
        });
        g1.push(b);
      }
      if (c.fill) {
        var cf = val('f');
        g1.push(ibtn('<span class="swatch-btn fill" style="background:' + (cf || 'transparent') + '"></span>', S('fill', 'Fill'), function (e, el) {
          colorPop(el, 'f', cf, STICKY.concat(COLORS), 'none');
        }));
      }
      if (c.sticky) {
        var cs = find(function (it) { return it.t === 'sticky'; }).f || STICKY[0];
        g1.push(ibtn('<span class="swatch-btn fill" style="background:' + cs + '"></span>', S('stickyColor', 'Note colour'), function (e, el) {
          colorPop(el, 'f', cs, STICKY.concat(PALETTE), null);
        }));
      }
      if (c.width) {
        var wi = find(function (it) { return caps(it).width; });
        var opts = wi.t === 'path' && wi.hl ? [12, 20, 32] : [2, 4, 8];
        var curW = swOf(wi);
        var g2 = grp();
        [[S('thin', 'Thin'), 1.2], [S('medium', 'Medium'), 2.4], [S('thick', 'Thick'), 4]].forEach(function (o, i) {
          g2.push(ibtn(icon('solid', o[1]), o[0], function () { setStyle('sw', opts[i]); },
            curW === opts[i] ? 'on' : ''));
        });
      }
      if (c.dash) {
        var g3 = grp(), cd = val('dash') || '';
        [['', 'solid', S('solid', 'Solid')], ['dash', 'dashed', S('dashed', 'Dashed')], ['dot', 'dotted', S('dotted', 'Dotted')]].forEach(function (o) {
          g3.push(ibtn(icon(o[1]), o[2], function () { setStyle('dash', o[0] || null); }, cd === o[0] ? 'on' : ''));
        });
      }
      if (c.radius) {
        grp().push(ibtn(icon('rounded'), S('rounded', 'Rounded corners'), function () { setStyle('r', val('r') ? null : 14); }, val('r') ? 'on' : ''));
      }
      if (c.font) {
        var g4 = grp();
        var fi = find(hasText);
        var fsSel = h('select', { title: S('fontSize', 'Font size') });
        var cfs = fsOf(fi);
        var sizes = [['S', 14], ['M', 20], ['L', 28], ['XL', 40]];
        if (!sizes.some(function (s) { return s[1] === cfs; })) sizes.push([String(cfs), cfs]);
        sizes.forEach(function (s) {
          var o = h('option', { value: s[1] }, s[0]);
          if (s[1] === cfs) o.selected = true;
          fsSel.appendChild(o);
        });
        fsSel.addEventListener('change', function () { setStyle('fs', parseInt(fsSel.value, 10)); });
        fsSel.addEventListener('pointerdown', function (e) { e.stopPropagation(); });
        g4.push(fsSel);
        g4.push(ibtn('<b>B</b>', S('bold', 'Bold'), function () { setStyle('b', val('b') ? null : true); }, 'txt' + (val('b') ? ' on' : '')));
      }
      if (c.align) {
        var g5 = grp(), ca = alOf(find(function (it) { return caps(it).align; }));
        [['l', 'alignL', S('alignLeft', 'Align left')], ['c', 'alignC', S('alignCenter', 'Centre')], ['r', 'alignR', S('alignRight', 'Align right')]].forEach(function (o) {
          g5.push(ibtn(icon(o[1]), o[2], function () { setStyle('al', o[0]); }, ca === o[0] ? 'on' : ''));
        });
      }
      if (c.ends) {
        var g6 = grp();
        var ei = find(function (it) { return caps(it).ends; }), ends = endsOf(ei);
        [[0, S('start', 'Start')], [1, S('end', 'End')]].forEach(function (o) {
          var s = h('select', { title: o[1] });
          [['none', S('none', 'None')], ['arrow', S('arrow', 'Arrow')], ['dot', S('dot', 'Dot')]].forEach(function (v) {
            var op = h('option', { value: v[0] }, (o[0] ? '→ ' : '← ') + v[1]);
            if (ends[o[0]] === v[0]) op.selected = true;
            s.appendChild(op);
          });
          s.addEventListener('change', function () {
            var e2 = endsOf(ei).slice();
            e2[o[0]] = s.value;
            setStyle('ends', e2);
          });
          s.addEventListener('pointerdown', function (e) { e.stopPropagation(); });
          g6.push(s);
        });
        if (c.kind) {
          var ks = h('select', { title: S('connector', 'Connector') });
          var ck = val('k') || 'straight';
          [['straight', S('straight', 'Straight')], ['elbow', S('elbow', 'Elbow')], ['curve', S('curve', 'Curved')]].forEach(function (v) {
            var op = h('option', { value: v[0] }, v[1]);
            if (ck === v[0]) op.selected = true;
            ks.appendChild(op);
          });
          ks.addEventListener('change', function () { setStyle('k', ks.value === 'straight' ? null : ks.value); });
          ks.addEventListener('pointerdown', function (e) { e.stopPropagation(); });
          g6.push(ks);
        }
      }
      var op = val('op');
      var range = h('input', { type: 'range', min: '0.1', max: '1', step: '0.05', title: S('opacity', 'Opacity') });
      range.value = op === undefined ? 1 : op;
      range.addEventListener('change', function () {
        var v = parseFloat(range.value);
        setStyle('op', v >= 1 ? null : v);
      });
      range.addEventListener('pointerdown', function (e) { e.stopPropagation(); });
      range.addEventListener('keydown', function (e) { e.stopPropagation(); });
      grp().push(range);
    }

    if (!tg.cat && !locked && alignable().length >= 2) {
      grp().push(ibtn(icon('align'), S('align', 'Align and distribute'), function (e, el) { alignPop(el); }));
    }

    if (!tg.cat) {
      var g7 = grp();
      if (!locked) {
        g7.push(ibtn(icon('front'), S('front', 'Bring to front') + ' (Ctrl+Shift+])', function () { arrange('front'); }));
        g7.push(ibtn(icon('back'), S('back', 'Send to back') + ' (Ctrl+Shift+[)', function () { arrange('back'); }));
        if (items.length > 1 && !items.every(function (it) { return it.g && it.g === first.g; })) {
          g7.push(ibtn(icon('group'), S('group', 'Group') + ' (Ctrl+G)', function () { group(true); }));
        }
        if (items.some(function (it) { return it.g; })) {
          g7.push(ibtn(icon('ungroup'), S('ungroup', 'Ungroup') + ' (Ctrl+Shift+G)', function () { group(false); }));
        }
      }
      g7.push(ibtn(icon(locked ? 'lock' : 'unlock'), locked ? S('unlock', 'Unlock') : S('lock', 'Lock'), function () {
        setStyle('lock', locked ? null : true);
      }, locked ? 'on' : ''));
      if (!locked) g7.push(ibtn(icon('trash'), S('delete', 'Delete') + ' (Del)', function () { deleteSel(); }, 'danger'));
    }

    groups.filter(function (g) { return g.length; }).forEach(function (g, i) {
      if (i) styleBar.appendChild(h('div', { class: 'sep' }));
      g.forEach(function (el) { styleBar.appendChild(el); });
    });
    styleBar.style.display = styleBar.childNodes.length ? 'flex' : 'none';
  }

  function deleteSel() {
    if (!sel.length) return;
    if (deleteIds(sel.slice())) commit(); else refreshStyleBar();
  }

  // ----- strokes ---------------------------------------------------------------------------

  function segDist(p, a, b) {
    var dx = b.x - a.x, dy = b.y - a.y, l = dx * dx + dy * dy;
    if (!l) return Math.hypot(p.x - a.x, p.y - a.y);
    var t = Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / l));
    return Math.hypot(p.x - a.x - t * dx, p.y - a.y - t * dy);
  }

  function rdp(pts, eps) {
    if (pts.length < 3) return pts;
    var keep = [], stack = [[0, pts.length - 1]];
    keep[0] = keep[pts.length - 1] = true;
    while (stack.length) {
      var r = stack.pop(), a = r[0], b = r[1], md = 0, mi = -1;
      for (var i = a + 1; i < b; i++) {
        var d = segDist(pts[i], pts[a], pts[b]);
        if (d > md) { md = d; mi = i; }
      }
      if (md > eps && mi > 0) { keep[mi] = true; stack.push([a, mi], [mi, b]); }
    }
    return pts.filter(function (p, i) { return keep[i]; });
  }

  function smoothD(pts, ox, oy) {
    function f(n) { return String(r1(n)); }
    function pt(p) { return f(p.x - ox) + ' ' + f(p.y - oy); }
    var s = 'M' + pt(pts[0]), n = pts.length;
    if (n === 2) return s + 'L' + pt(pts[1]);
    for (var i = 0; i < n - 1; i++) {
      var p0 = pts[Math.max(i - 1, 0)], p1 = pts[i], p2 = pts[i + 1], p3 = pts[Math.min(i + 2, n - 1)];
      var c1 = { x: p1.x + (p2.x - p0.x) / 6, y: p1.y + (p2.y - p0.y) / 6 };
      var c2 = { x: p2.x - (p3.x - p1.x) / 6, y: p2.y - (p3.y - p1.y) / 6 };
      s += 'C' + pt(c1) + ' ' + pt(c2) + ' ' + pt(p2);
    }
    return s;
  }

  function finishStroke(d) {
    if (live) live.remove();
    live = null;
    var pts = d.pts;
    if (pts.length === 1) pts.push({ x: pts[0].x + 0.1, y: pts[0].y + 0.1 });
    pts = rdp(pts, 0.7 / view.z);
    var bb = boxOf(pts);
    var it = newItem('path', d.cat, { x: bb.x, y: bb.y, w: Math.max(bb.w, 1), h: Math.max(bb.h, 1) });
    if (d.cat === 'hl') it.hl = true;
    it.d = smoothD(pts, bb.x, bb.y);
    addItem(it);
    commit();
  }

  // ----- erasing ------------------------------------------------------------------------------

  function eraseAt(cx, cy, d) {
    var els = document.elementsFromPoint(cx, cy), ids = [];
    els.forEach(function (el) {
      var g = el.closest ? el.closest('[data-id]') : null;
      if (g && world.contains(g)) ids.push(g.getAttribute('data-id'));
    });
    if (ids.length && deleteIds(ids)) { d.changed = true; render(); }
  }

  // ----- hit testing ----------------------------------------------------------------------------

  function itemOf(target) {
    var g = target && target.closest ? target.closest('[data-id]') : null;
    return g && world.contains(g) ? g.getAttribute('data-id') : null;
  }

  /// The item under a point that connectors can attach to (any item with `any`).
  function itemUnder(cx, cy, exclude, any) {
    var els = document.elementsFromPoint(cx, cy);
    for (var i = 0; i < els.length; i++) {
      var id = itemOf(els[i]);
      if (!id || id === exclude) continue;
      var it = byId[id];
      if (it && (any || (it.t !== 'conn' && it.t !== 'line' && it.t !== 'arrow'))) return id;
    }
    return null;
  }

  // ----- pointer gestures --------------------------------------------------------------------------

  function startDrag(e, d) {
    drag = d;
    try { svg.setPointerCapture(e.pointerId); } catch (_) { /* ignore */ }
  }

  var DEFAULT_SIZE = { rect: [160, 100], ellipse: [140, 100], diamond: [140, 110], triangle: [130, 110], sticky: [180, 180], text: [240, 30], frame: [640, 400] };

  function onDown(e) {
    closePop();
    var ae = document.activeElement;
    if (ae && ae !== document.body && ae !== ta && ae.blur) ae.blur();
    if (editing) finishEdit();
    lastPointer = { x: e.clientX, y: e.clientY };
    var p = toWorld(e.clientX, e.clientY);
    if (e.button === 1 || (e.button === 0 && (spaceDown || tool === 'hand'))) {
      e.preventDefault();
      svg.classList.add('grabbing');
      startDrag(e, { kind: 'pan', cx: e.clientX, cy: e.clientY, vx: view.x, vy: view.y });
      return;
    }
    if (e.button !== 0) return;
    var hname = e.target.getAttribute ? e.target.getAttribute('data-h') : null;

    if (tool === 'select') {
      if (hname) { startHandle(e, hname, p); return; }
      var id = itemOf(e.target);
      if (id) {
        var ids = groupOf(id), d = { kind: 'move', p0: p, moved: false };
        if (e.shiftKey) {
          var on = sel.indexOf(id) >= 0;
          setSel(on ? sel.filter(function (x) { return ids.indexOf(x) < 0; }) : sel.concat(ids));
          if (on) return;
        } else if (sel.indexOf(id) < 0) {
          setSel(ids);
        } else if (sel.length > ids.length) {
          d.reduceTo = ids;
        }
        var movable = selItems().filter(function (it) { return !it.lock; });
        if (!movable.length) return;
        d.box = union(movable);
        var frames = movable.filter(function (it) { return it.t === 'frame'; });
        if (frames.length) {
          framedItems(frames, idSet(movable)).forEach(function (it) { if (!it.lock) movable.push(it); });
        }
        d.orig = {};
        movable.forEach(function (it) { d.orig[it.id] = clone(it); });
        startDrag(e, d);
        return;
      }
      startDrag(e, { kind: 'band', p0: p, p1: p, base: e.shiftKey ? sel.slice() : [] });
      if (!e.shiftKey) setSel([]);
      return;
    }

    if (tool === 'pen' || tool === 'hl') {
      var cat = tool;
      var st = toolStyle[cat];
      live = svgEl('path', {
        fill: 'none', stroke: st.s || ink, 'stroke-width': st.sw || (cat === 'hl' ? 20 : 3),
        'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'stroke-opacity': cat === 'hl' ? 0.45 : null,
        opacity: st.op
      });
      world.appendChild(live);
      startDrag(e, { kind: 'draw', cat: cat, pts: [p] });
      live.setAttribute('d', 'M' + r1(p.x) + ' ' + r1(p.y) + 'l0.1 0.1');
      return;
    }

    if (tool === 'eraser') {
      var de = { kind: 'erase', last: { x: e.clientX, y: e.clientY }, changed: false };
      startDrag(e, de);
      eraseAt(e.clientX, e.clientY, de);
      return;
    }

    if (CLOSED[tool] || tool === 'sticky' || tool === 'text' || tool === 'frame') {
      var q = snapP(p, e);
      var it = newItem(tool, TOOL_CAT[tool], { x: q.x, y: q.y, w: 0, h: 0 });
      if (tool === 'sticky' && !it.f) it.f = STICKY[0];
      addItem(it);
      startDrag(e, { kind: 'create', it: it, p0: q });
      render();
      return;
    }

    if (tool === 'line' || tool === 'arrow') {
      var q2 = snapP(p, e);
      var li = newItem(tool, tool, { pts: [q2.x, q2.y, q2.x, q2.y] });
      addItem(li);
      startDrag(e, { kind: 'line', it: li, p0: q2 });
      render();
      return;
    }

    if (tool === 'conn') {
      var tid = itemUnder(e.clientX, e.clientY, null);
      var a = tid ? { id: tid, p: anchorNear(byId[tid], p) } : { x: p.x, y: p.y };
      var ci = newItem('conn', 'conn', { a: a, b: { x: p.x, y: p.y } });
      addItem(ci);
      startDrag(e, { kind: 'connEnd', it: ci, which: 'b', created: true, p0: p });
      render();
    }
  }

  function startHandle(e, name, p) {
    var items = selItems().filter(function (it) { return !it.lock; });
    if (!items.length) return;
    if (name === 'p0' || name === 'p1') {
      startDrag(e, { kind: 'lineEnd', it: items[0], i: name === 'p0' ? 0 : 2 });
      return;
    }
    if (name === 'a' || name === 'b') {
      startDrag(e, { kind: 'connEnd', it: items[0], which: name });
      return;
    }
    var orig = {};
    items.forEach(function (it) { orig[it.id] = clone(it); });
    var single = items.length === 1 && sel.length === 1 ? items[0] : null;
    var box;
    if (single) {
      box = { x: single.x, y: single.y, w: single.w, h: single.h };
      if (single.t === 'text') box.h = textLayout(single).h;
    } else box = union(items);
    var c = { x: box.x + box.w / 2, y: box.y + box.h / 2 };
    if (name === 'rot') {
      startDrag(e, { kind: 'rotate', orig: orig, single: single, c: c, a0: Math.atan2(p.y - c.y, p.x - c.x) * 180 / Math.PI });
    } else {
      startDrag(e, { kind: 'resize', h: name, orig: orig, single: single, box: box, rot: single ? single.rot || 0 : 0 });
    }
  }

  /// New box for a resize in the box's own (unrotated) frame.
  function resizeBox(o, hn, p, keep) {
    var x0 = o.x, y0 = o.y, x1 = o.x + o.w, y1 = o.y + o.h;
    if (hn.indexOf('w') >= 0) x0 = Math.min(p.x, x1 - MIN);
    if (hn.indexOf('e') >= 0) x1 = Math.max(p.x, x0 + MIN);
    if (hn.indexOf('n') >= 0) y0 = Math.min(p.y, y1 - MIN);
    if (hn.indexOf('s') >= 0) y1 = Math.max(p.y, y0 + MIN);
    var nw = x1 - x0, nh = y1 - y0;
    if (keep && hn.length === 2 && o.w > 0 && o.h > 0) {
      var ratio = o.w / o.h;
      if (nw / nh > ratio) nw = nh * ratio; else nh = nw / ratio;
      if (hn.indexOf('w') >= 0) x0 = x1 - nw; else x1 = x0 + nw;
      if (hn.indexOf('n') >= 0) y0 = y1 - nh; else y1 = y0 + nh;
    }
    return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
  }

  function doResize(d, p, e) {
    var hn = d.h;
    if (d.single) {
      var o = d.orig[d.single.id], it = byId[o.id], rot = o.rot || 0;
      var ob = d.box, c = { x: ob.x + ob.w / 2, y: ob.y + ob.h / 2 };
      var lp = rotP(p.x, p.y, c.x, c.y, -rot);
      if (!rot) lp = snapP(lp, e);
      var keepDefault = o.t === 'image' || o.t === 'emoji' || o.t === 'sticker';
      var keep = keepDefault ? !e.shiftKey : e.shiftKey;
      var nb = resizeBox(ob, hn, lp, keep);
      var fx = hn.indexOf('w') >= 0 ? ob.x + ob.w : hn.indexOf('e') >= 0 ? ob.x : c.x;
      var fy = hn.indexOf('n') >= 0 ? ob.y + ob.h : hn.indexOf('s') >= 0 ? ob.y : c.y;
      var fw = rotP(fx, fy, c.x, c.y, rot);
      var nc = { x: nb.x + nb.w / 2, y: nb.y + nb.h / 2 };
      var v = rotP(fx - nc.x, fy - nc.y, 0, 0, rot);
      var cx = fw.x - v.x, cy = fw.y - v.y;
      it.x = cx - nb.w / 2;
      it.y = cy - nb.h / 2;
      it.w = nb.w;
      if (o.t !== 'text') it.h = nb.h;
      if (o.t === 'path') it.d = scaleD(o.d, nb.w / o.w, nb.h / o.h);
      return;
    }
    var B = d.box;
    var nb2 = resizeBox(B, hn, snapP(p, e), e.shiftKey);
    var sx = B.w ? nb2.w / B.w : 1, sy = B.h ? nb2.h / B.h : 1;
    var F = {
      x: hn.indexOf('w') >= 0 ? B.x + B.w : B.x,
      y: hn.indexOf('n') >= 0 ? B.y + B.h : B.y
    };
    function map(x, y) { return { x: F.x + (x - F.x) * sx, y: F.y + (y - F.y) * sy }; }
    Object.keys(d.orig).forEach(function (id) {
      var o2 = d.orig[id], it2 = byId[id];
      if (!it2) return;
      if (o2.t === 'line' || o2.t === 'arrow') {
        var a = map(o2.pts[0], o2.pts[1]), b = map(o2.pts[2], o2.pts[3]);
        it2.pts = [a.x, a.y, b.x, b.y];
      } else if (o2.t === 'conn') {
        ['a', 'b'].forEach(function (k) { if (o2[k] && !o2[k].id) it2[k] = map(o2[k].x, o2[k].y); });
      } else {
        var cc = map(o2.x + o2.w / 2, o2.y + o2.h / 2);
        it2.w = Math.max(1, o2.w * sx);
        if (o2.t !== 'text') it2.h = Math.max(1, o2.h * sy);
        it2.x = cc.x - it2.w / 2;
        it2.y = cc.y - (o2.t === 'text' ? o2.h : it2.h) / 2;
        if (o2.t === 'path') it2.d = scaleD(o2.d, sx, sy);
      }
      if (hasText(o2) && (o2.txt || o2.t === 'text')) {
        // Text keeps its proportions to the box.
        var nf = r1(fsOf(o2) * Math.sqrt(sx * sy));
        if (Math.abs(nf - fsOf(o2)) >= 0.1) it2.fs = Math.max(4, nf);
      }
    });
  }

  function doRotate(d, p, e) {
    var ang = Math.atan2(p.y - d.c.y, p.x - d.c.x) * 180 / Math.PI;
    if (d.single) {
      var r = ang + 90;
      if (e.shiftKey) r = Math.round(r / 15) * 15;
      byId[d.single.id].rot = r;
      return;
    }
    var delta = ang - d.a0;
    if (e.shiftKey) delta = Math.round(delta / 15) * 15;
    Object.keys(d.orig).forEach(function (id) {
      var o = d.orig[id], it = byId[id];
      if (!it) return;
      if (o.t === 'line' || o.t === 'arrow') {
        var a = rotP(o.pts[0], o.pts[1], d.c.x, d.c.y, delta), b = rotP(o.pts[2], o.pts[3], d.c.x, d.c.y, delta);
        it.pts = [a.x, a.y, b.x, b.y];
      } else if (o.t === 'conn') {
        ['a', 'b'].forEach(function (k) { if (o[k] && !o[k].id) it[k] = rotP(o[k].x, o[k].y, d.c.x, d.c.y, delta); });
      } else {
        var nc = rotP(o.x + o.w / 2, o.y + o.h / 2, d.c.x, d.c.y, delta);
        it.x = nc.x - o.w / 2;
        it.y = nc.y - o.h / 2;
        it.rot = (o.rot || 0) + delta;
      }
    });
  }

  function angleSnap(o, q) {
    var a = Math.atan2(q.y - o.y, q.x - o.x), l = dist(o, q), s = Math.PI / 12;
    a = Math.round(a / s) * s;
    return { x: o.x + Math.cos(a) * l, y: o.y + Math.sin(a) * l };
  }

  function onMove(e) {
    lastPointer = { x: e.clientX, y: e.clientY };
    var d = drag;
    if (!d) {
      if (tool === 'conn') {
        var hid = itemUnder(e.clientX, e.clientY, null);
        if (hid !== hoverId) { hoverId = hid; renderOverlay(); }
      }
      return;
    }
    var p = toWorld(e.clientX, e.clientY);
    switch (d.kind) {
      case 'pan':
        view.x = d.vx + e.clientX - d.cx;
        view.y = d.vy + e.clientY - d.cy;
        applyView();
        return;
      case 'move': {
        var dx = p.x - d.p0.x, dy = p.y - d.p0.y;
        if (!d.moved && Math.hypot(dx, dy) * view.z < 3) return;
        if (snapOn && !e.altKey) {
          dx = snap(d.box.x + dx) - d.box.x;
          dy = snap(d.box.y + dy) - d.box.y;
        }
        d.moved = true;
        Object.keys(d.orig).forEach(function (id) { if (byId[id]) moveItem(byId[id], d.orig[id], dx, dy); });
        render();
        return;
      }
      case 'band': {
        d.p1 = p;
        renderOverlay();
        return;
      }
      case 'draw': {
        var last = d.pts[d.pts.length - 1];
        if (dist(last, p) * view.z < 1.5) return;
        d.pts.push(p);
        live.setAttribute('d', polyD(d.pts));
        return;
      }
      case 'erase': {
        var steps = Math.ceil(Math.hypot(e.clientX - d.last.x, e.clientY - d.last.y) / 4);
        for (var i = 1; i <= steps; i++) {
          eraseAt(d.last.x + (e.clientX - d.last.x) * i / steps, d.last.y + (e.clientY - d.last.y) * i / steps, d);
        }
        d.last = { x: e.clientX, y: e.clientY };
        return;
      }
      case 'create': {
        var q = snapP(p, e), it = d.it;
        var w = Math.abs(q.x - d.p0.x), hh = Math.abs(q.y - d.p0.y);
        if (e.shiftKey) w = hh = Math.max(w, hh);
        it.x = q.x < d.p0.x ? d.p0.x - w : d.p0.x;
        it.y = q.y < d.p0.y ? d.p0.y - hh : d.p0.y;
        it.w = w;
        it.h = it.t === 'text' ? 0 : hh;
        render();
        return;
      }
      case 'line': {
        var q2 = snapP(p, e);
        if (e.shiftKey) q2 = angleSnap(d.p0, q2);
        d.it.pts[2] = q2.x;
        d.it.pts[3] = q2.y;
        render();
        return;
      }
      case 'lineEnd': {
        var it2 = d.it, other = d.i === 0 ? { x: it2.pts[2], y: it2.pts[3] } : { x: it2.pts[0], y: it2.pts[1] };
        var q3 = snapP(p, e);
        if (e.shiftKey) q3 = angleSnap(other, q3);
        it2.pts[d.i] = q3.x;
        it2.pts[d.i + 1] = q3.y;
        render();
        return;
      }
      case 'connEnd': {
        var c = d.it, oth = c[d.which === 'a' ? 'b' : 'a'];
        var tid = itemUnder(e.clientX, e.clientY, c.id);
        if (tid && !(oth && oth.id === tid)) {
          c[d.which] = { id: tid, p: anchorNear(byId[tid], p) };
          hoverId = tid;
        } else {
          c[d.which] = { x: p.x, y: p.y };
          hoverId = null;
        }
        render();
        return;
      }
      case 'resize':
        doResize(d, p, e);
        render();
        return;
      case 'rotate':
        doRotate(d, p, e);
        render();
        return;
    }
  }

  function onUp(e) {
    var d = drag;
    if (!d) return;
    drag = null;
    try { svg.releasePointerCapture(e.pointerId); } catch (_) { /* ignore */ }
    svg.classList.remove('grabbing');
    switch (d.kind) {
      case 'pan':
        break;
      case 'move':
        if (d.moved) commit();
        else if (d.reduceTo) setSel(d.reduceTo);
        break;
      case 'band': {
        var b = boxOf([d.p0, d.p1]), ids = d.base.slice();
        if (b.w * view.z > 2 || b.h * view.z > 2) {
          scene.items.forEach(function (it) {
            var a = aabb(it);
            if (a.x >= b.x && a.y >= b.y && a.x + a.w <= b.x + b.w && a.y + a.h <= b.y + b.h) ids = ids.concat(groupOf(it.id));
          });
        }
        setSel(ids);
        break;
      }
      case 'draw':
        finishStroke(d);
        break;
      case 'erase':
        if (d.changed) commit();
        break;
      case 'create': {
        var it = d.it, tiny = it.w * view.z < 6 && (it.t === 'text' || it.h * view.z < 6);
        var ds = DEFAULT_SIZE[it.t] || [120, 80];
        if (tiny) {
          it.w = ds[0];
          if (it.t === 'text') { it.x = d.p0.x; it.y = d.p0.y - fsOf(it) * 0.7; } else {
            it.h = ds[1];
            it.x = d.p0.x - ds[0] / 2;
            it.y = d.p0.y - ds[1] / 2;
          }
        }
        if (it.t === 'text') it.w = Math.max(it.w, 60);
        if (it.t === 'frame' && !it.txt) {
          it.txt = S('frame', 'Frame') + ' ' + scene.items.filter(function (x) { return x.t === 'frame'; }).length;
        }
        setTool('select');
        setSel([it.id]);
        if (it.t === 'text') { render(); startEdit(it.id); }
        else if (it.t === 'sticky') { commit(); startEdit(it.id); }
        else commit();
        break;
      }
      case 'line': {
        var li = d.it;
        if (dist({ x: li.pts[0], y: li.pts[1] }, { x: li.pts[2], y: li.pts[3] }) * view.z < 6) {
          li.pts[2] = li.pts[0] + 140;
          li.pts[3] = li.pts[1];
        }
        setTool('select');
        setSel([li.id]);
        commit();
        break;
      }
      case 'lineEnd':
        commit();
        break;
      case 'connEnd': {
        hoverId = null;
        var c = d.it;
        if (d.created) {
          var r = route(c);
          if (dist(r.A, r.B) * view.z < 6 && !(c.a.id && c.b.id)) {
            deleteIds([c.id], true);
            render();
            break;
          }
          setTool('select');
          setSel([c.id]);
        }
        commit();
        break;
      }
      case 'resize':
      case 'rotate':
        commit();
        break;
    }
    render();
    flush();
  }

  svg.addEventListener('pointerdown', onDown);
  svg.addEventListener('pointermove', onMove);
  svg.addEventListener('pointerup', onUp);
  svg.addEventListener('pointercancel', onUp);
  svg.addEventListener('pointerleave', function () {
    if (!drag && hoverId) { hoverId = null; renderOverlay(); }
  });
  svg.addEventListener('contextmenu', function (e) { e.preventDefault(); });
  svg.addEventListener('dblclick', function (e) {
    if (tool !== 'select') return;
    // The target is the board itself while it has the pointer captured.
    var id = itemOf(e.target) || itemUnder(e.clientX, e.clientY, null, true);
    if (id) {
      var it = byId[id];
      if (it && it.g && sel.length > 1) setSel([id]);
      startEdit(id);
      return;
    }
    var p = snapP(toWorld(e.clientX, e.clientY), e);
    var t = newItem('text', 'text', { x: p.x, y: p.y - fsOf({ t: 'text', fs: toolStyle.text.fs }) * 0.7, w: 240, h: 30 });
    addItem(t);
    setSel([t.id]);
    render();
    startEdit(t.id);
  });

  svg.addEventListener('wheel', function (e) {
    e.preventDefault();
    var r = svg.getBoundingClientRect();
    var unit = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? r.height : 1;
    if (e.ctrlKey || e.metaKey) {
      zoomAt(view.z * Math.exp(-e.deltaY * unit * 0.0025), e.clientX - r.left, e.clientY - r.top);
      return;
    }
    var dx = e.deltaX * unit, dy = e.deltaY * unit;
    if (e.shiftKey && !dx) { dx = dy; dy = 0; }
    view.x -= dx;
    view.y -= dy;
    applyView();
  }, { passive: false });

  // Files are dropped through GTK (which calls QC.addImages); the page must
  // not navigate to them.
  document.addEventListener('dragover', function (e) { e.preventDefault(); });
  document.addEventListener('drop', function (e) { e.preventDefault(); });
  window.addEventListener('resize', function () { renderOverlay(); placeEditor(); });

  // ----- keyboard ---------------------------------------------------------------------------------

  document.addEventListener('keydown', function (e) {
    var tag = e.target && e.target.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;
    var ctrl = e.ctrlKey || e.metaKey, k = e.key, kl = k.length === 1 ? k.toLowerCase() : k;
    if (k === ' ') {
      if (!spaceDown) { spaceDown = true; svg.classList.add('panning'); }
      e.preventDefault();
      return;
    }
    if (ctrl && !e.altKey) {
      var handled = true;
      if (kl === 'z') post({ type: e.shiftKey ? 'redo' : 'undo' });
      else if (kl === 'y') post({ type: 'redo' });
      else if (kl === 'a') { setTool('select'); setSel(scene.items.map(function (it) { return it.id; })); }
      else if (kl === 'd') { if (sel.length) insertClones(copyPayload().items, null, 20); }
      else if (kl === 'c' || kl === 'x') {
        if (sel.length) {
          post({ type: 'canvas-copy', payload: copyPayload(), cut: kl === 'x' });
          if (kl === 'x') deleteSel();
        }
      } else if (kl === 'v') {
        var r = svg.getBoundingClientRect(), lp = lastPointer;
        var inside = lp && lp.x >= r.left && lp.y >= r.top && lp.x <= r.right && lp.y <= r.bottom;
        post({ type: 'canvas-paste', x: inside ? lp.x : null, y: inside ? lp.y : null });
      } else if (kl === 'g') group(!e.shiftKey);
      else if (k === ']' || k === '}') { if (sel.length) arrange(e.shiftKey ? 'front' : 'fwd'); }
      else if (k === '[' || k === '{') { if (sel.length) arrange(e.shiftKey ? 'back' : 'bwd'); }
      else if (k === '0') zoomCentered(1);
      else if (k === '=' || k === '+') zoomCentered(view.z * 1.25);
      else if (k === '-') zoomCentered(view.z / 1.25);
      else handled = false;
      if (handled) e.preventDefault();
      return;
    }
    if (e.altKey) return;
    if (k === 'Delete' || k === 'Backspace') { deleteSel(); e.preventDefault(); }
    else if (k === 'Escape') {
      closePop();
      if (sel.length) setSel([]); else setTool('select');
    } else if (k === 'ArrowLeft' || k === 'ArrowRight' || k === 'ArrowUp' || k === 'ArrowDown') {
      if (!sel.length) return;
      var s = e.shiftKey ? 10 : 1;
      nudge(k === 'ArrowLeft' ? -s : k === 'ArrowRight' ? s : 0, k === 'ArrowUp' ? -s : k === 'ArrowDown' ? s : 0);
      e.preventDefault();
    } else if (k === 'PageUp') { if (sel.length) arrange('fwd'); e.preventDefault(); }
    else if (k === 'PageDown') { if (sel.length) arrange('bwd'); e.preventDefault(); }
    else if (k === 'Enter' || k === 'F2') {
      if (sel.length === 1) { startEdit(sel[0]); e.preventDefault(); }
    } else if (kl === 's') {
      setTool(shapeKind);
      libraryPop(shapeBtn);
    } else if (TOOL_KEYS[kl]) {
      setTool(TOOL_KEYS[kl]);
    }
  });

  document.addEventListener('keyup', function (e) {
    if (e.key === ' ') { spaceDown = false; svg.classList.remove('panning'); }
  });
  window.addEventListener('blur', function () { spaceDown = false; svg.classList.remove('panning'); });

  // ----- theme ---------------------------------------------------------------------------------------

  function readTheme() {
    var cs = getComputedStyle(document.documentElement);
    ink = cs.getPropertyValue('--ink').trim() || '#1f2328';
    canvasBg = cs.getPropertyValue('--canvas').trim() || '#ffffff';
  }

  window.setTheme = function (t, vars) {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.cssText = vars || '';
    readTheme();
    clearCache();
    tplBox.textContent = '';
    render();
    refreshStyleBar();
  };

  // ----- API for Rust -------------------------------------------------------------------------------

  window.QC = {
    update: function (sc) {
      if (!sc || typeof sc !== 'object') sc = { canvas: 1, items: [] };
      if (!Array.isArray(sc.items)) sc.items = [];
      var s = JSON.stringify(sc);
      if (s === lastJSON) return;
      if (drag || editing) { pending = sc; return; }
      scene = sc;
      lastJSON = s;
      lastContent = contentKey();
      index();
      if (!loaded) {
        loaded = true;
        var v = sc.view;
        if (v && typeof v.z === 'number' && v.z > 0) view = { x: +v.x || 0, y: +v.y || 0, z: Math.max(ZMIN, Math.min(ZMAX, v.z)) };
        else fit(1);
      }
      if (document.activeElement !== titleInput) titleInput.value = scene.title || '';
      sel = sel.filter(function (id) { return byId[id]; });
      applyView();
      render();
      refreshStyleBar();
    },

    addImages: function (cx, cy, list) {
      if (!list || !list.length) return;
      var at = cx === null || cx === undefined ? viewCenter() : toWorld(cx, cy);
      var n = list.length, done = 0, made = [];
      list.forEach(function (entry, i) {
        var img = new Image();
        var fin = function (w, hh) {
          var s = Math.min(1, 800 / Math.max(w, hh));
          w *= s; hh *= s;
          made[i] = { id: rid(), t: 'image', src: entry.src, x: at.x - w / 2 + i * 24, y: at.y - hh / 2 + i * 24, w: w, h: hh };
          byId[made[i].id] = made[i];
          if (++done === n) {
            made.forEach(function (it) { scene.items.push(it); });
            index();
            setTool('select');
            setSel(made.map(function (it) { return it.id; }));
            commit();
          }
        };
        img.onload = function () { fin(img.naturalWidth || 320, img.naturalHeight || 240); };
        img.onerror = function () { fin(320, 240); };
        img.src = assetUrl(entry.src);
      });
    },

    /// Zooms around the centre: step > 0 in, < 0 out, 0 to 100 %.
    zoomBy: function (step) {
      if (step === 0) zoomCentered(1);
      else zoomCentered(step > 0 ? view.z * 1.25 : view.z / 1.25);
    },

    addText: function (cx, cy, text) {
      text = String(text || '').replace(/\r\n?/g, '\n').replace(/\s+$/, '');
      if (!text) return;
      var at = cx === null || cx === undefined ? viewCenter() : toWorld(cx, cy);
      var it = newItem('text', 'text', { x: 0, y: 0, w: 100, h: 30, txt: text });
      var fs = fsOf(it);
      it.w = Math.min(480, Math.max(60, textWidth(text.split('\n'), fs, it.b) + 12));
      it.h = textLayout(it).h;
      it.x = at.x - it.w / 2;
      it.y = at.y - it.h / 2;
      addItem(it);
      setTool('select');
      setSel([it.id]);
      commit();
    },

    pasteItems: function (cx, cy, payload) {
      if (!payload || !payload.items) return;
      var at = cx === null || cx === undefined ? viewCenter() : toWorld(cx, cy);
      setTool('select');
      insertClones(payload.items, at, null);
    },

    exportSvg: function () {
      index();
      // With a selection, only that (and what selected frames hold).
      var items = sel.length ? selWithContents() : scene.items.slice();
      if (sel.length) {
        var ids = idSet(items);
        scene.items.forEach(function (it) {
          if (it.t === 'conn' && !ids[it.id] && it.a && it.b && ids[it.a.id] && ids[it.b.id]) items.push(it);
        });
        ids = idSet(items);
        items = scene.items.filter(function (it) { return ids[it.id]; });
      }
      items = zOrder(items);
      var bb = items.length ? union(items) : contentBox(), pad = 24;
      var x = Math.floor(bb.x - pad), y = Math.floor(bb.y - pad);
      var w = Math.ceil(bb.w + 2 * pad), hh = Math.ceil(bb.h + 2 * pad);
      var ser = new XMLSerializer();
      var parts = items.map(function (it) { return ser.serializeToString(draw(it, true)); });
      var out = '<svg xmlns="http://www.w3.org/2000/svg" width="' + w + '" height="' + hh + '" viewBox="' +
        x + ' ' + y + ' ' + w + ' ' + hh + '">' +
        '<defs><filter id="qc-shadow" x="-10%" y="-10%" width="125%" height="135%"><feDropShadow dx="0" dy="2" stdDeviation="3" flood-color="#000" flood-opacity=".18"/></filter></defs>' +
        '<rect x="' + x + '" y="' + y + '" width="' + w + '" height="' + hh + '" fill="' + canvasBg + '"/>' +
        parts.join('') + '</svg>';
      return JSON.stringify({ svg: out, width: w, height: hh, background: canvasBg });
    }
  };

  // ----- start --------------------------------------------------------------------------------------------

  buildTools();
  buildZoom();
  readTheme();
  setTool('select');
  applyView();
  render();
})();
