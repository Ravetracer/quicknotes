// QuickNotes Kanban board. The model comes from Rust (`QK.update(board)`);
// every change sends the whole board back (`{type: 'kanban', board}`), where it
// is written to the Markdown source. Drag and drop uses pointer events.
(function () {
  'use strict';
  var S = window.QK_STRINGS || {};
  var PALETTE = window.QK_PALETTE || ['#62a0ea'];
  var app = document.getElementById('app');
  var state = null;
  var lastJSON = '';
  var pending = null;
  var busy = 0;          // > 0 while editing or dragging: incoming updates wait
  var menu = null;
  var activeDrag = null;

  function post(msg) {
    if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.qn) {
      window.webkit.messageHandlers.qn.postMessage(JSON.stringify(msg));
    }
  }

  function commit() {
    lastJSON = JSON.stringify(state);
    pending = null;
    post({ type: 'kanban', board: state });
    render();
  }

  /// Applies an update that arrived while editing or dragging.
  function flush() {
    if (!busy && pending) {
      var b = pending;
      pending = null;
      window.QK.update(b);
    }
  }

  window.QK = {
    update: function (board) {
      var s = JSON.stringify(board);
      if (s === lastJSON) return;
      if (busy) { pending = board; return; }
      state = board;
      lastJSON = s;
      render();
    }
  };
  window.setTheme = function (t, vars) {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.cssText = vars || '';
  };

  // ----- DOM helpers -----------------------------------------------------

  function h(tag, attrs) {
    var el = document.createElement(tag);
    if (attrs) {
      Object.keys(attrs).forEach(function (k) {
        if (attrs[k] !== undefined && attrs[k] !== null) el.setAttribute(k, attrs[k]);
      });
    }
    for (var i = 2; i < arguments.length; i++) {
      var c = arguments[i];
      if (c === null || c === undefined) continue;
      el.append(c instanceof Node ? c : document.createTextNode(String(c)));
    }
    return el;
  }

  function button(label, onClick, cls, title) {
    var b = h('button', { class: cls || 'btn', title: title, type: 'button' }, label);
    b.addEventListener('click', function (e) { e.stopPropagation(); onClick(e); });
    b.addEventListener('pointerdown', function (e) { e.stopPropagation(); });
    return b;
  }

  function icon(label, onClick, cls, title) {
    return button(label, onClick, 'icon' + (cls ? ' ' + cls : ''), title);
  }

  function emptyCells() {
    return state.columns.map(function () { return []; });
  }

  function cardCount(lane) {
    return lane.cells.reduce(function (n, c) { return n + c.length; }, 0);
  }

  // ----- rendering -------------------------------------------------------

  function render() {
    closeMenu();
    if (!state) return;
    var st = state;
    var n = st.columns.length;
    app.textContent = '';

    var total = 0, done = 0;
    st.lanes.forEach(function (l) {
      l.cells.forEach(function (c) {
        c.forEach(function (card) { total++; if (card.done) done++; });
      });
    });

    var title = h('div', { class: 'title', 'data-placeholder': S.boardTitle, title: S.rename }, st.title);
    title.addEventListener('click', function () {
      editInline(title, st.title, function (v) { st.title = v; });
    });
    var bar = h('div', { class: 'bar' }, title,
      total ? h('span', { class: 'stats' }, '✓ ' + done + ' / ' + total) : null,
      button('+ ' + S.addColumn, addColumn),
      button('+ ' + S.addLane, addLane));
    app.append(bar);
    if (!n) {
      app.append(h('div', { class: 'empty' }, S.noColumns));
      return;
    }

    var grid = h('div', { class: 'grid', style: '--n:' + n });
    st.columns.forEach(function (col, i) {
      var count = st.lanes.reduce(function (a, l) { return a + l.cells[i].length; }, 0);
      var over = col.wip > 0 && count > col.wip;
      var name = h('span', { class: 'name', title: col.title }, col.title);
      var head = h('div', { class: 'colhead' + (over ? ' over-limit' : ''), style: '--c:' + col.color, 'data-col': i },
        name,
        h('span', { class: 'count' + (over ? ' over' : '') }, col.wip > 0 ? count + ' / ' + col.wip : String(count)),
        icon('⋯', function (e) { columnMenu(i, e.currentTarget); }, null, S.rename));
      head.addEventListener('pointerdown', function (e) { startColumnDrag(e, i, head); });
      grid.append(head);
    });

    var showLanes = st.lanes.length > 1 || st.lanes.some(function (l) { return l.title; });
    st.lanes.forEach(function (lane, j) {
      var wrap = h('div', { class: 'lane' + (lane.collapsed ? ' collapsed' : ''), style: 'display:contents', 'data-lane': j });
      if (showLanes) {
        var lname = h('span', { class: 'name' }, lane.title || '—');
        var head = h('div', { class: 'lanehead', 'data-lane': j },
          icon('▾', function () { lane.collapsed = !lane.collapsed; commit(); }, 'twisty', S.collapse),
          lname,
          h('span', { class: 'count' }, String(cardCount(lane))),
          icon('⋯', function (e) { laneMenu(j, e.currentTarget); }, null, S.rename));
        head.addEventListener('pointerdown', function (e) { startLaneDrag(e, j, head); });
        wrap.append(head);
      }
      lane.cells.forEach(function (cards, i) {
        var cell = h('div', { class: 'cell', 'data-lane': j, 'data-col': i, style: '--c:' + st.columns[i].color });
        cards.forEach(function (card, k) { cell.append(cardElement(card, j, i, k)); });
        cell.append(button('+ ' + S.addCard, function () { addCard(j, i); }, 'add'));
        wrap.append(cell);
      });
      if (showLanes && j < st.lanes.length - 1) wrap.append(h('div', { class: 'lanesep' }));
      grid.append(wrap);
    });
    app.append(grid);
  }

  function cardElement(card, j, i, k) {
    var el = h('div', { class: 'card' + (card.done ? ' done' : ''), 'data-lane': j, 'data-col': i, 'data-idx': k });
    var check = h('input', { type: 'checkbox', class: 'check' });
    check.checked = !!card.done;
    check.addEventListener('pointerdown', function (e) { e.stopPropagation(); });
    check.addEventListener('click', function (e) { e.stopPropagation(); });
    check.addEventListener('change', function () { card.done = check.checked; commit(); });
    var body = h('div', { class: 'body' }, h('div', { class: 't' }, card.title || ' '));
    if (card.desc) body.append(h('div', { class: 'd' }, card.desc));
    var del = icon('×', function () { state.lanes[j].cells[i].splice(k, 1); commit(); }, 'x', S.delete);
    el.append(check, body, del);
    el.addEventListener('pointerdown', function (e) { startCardDrag(e, el, j, i, k); });
    return el;
  }

  // ----- editing ---------------------------------------------------------

  function autosize(t) {
    t.style.height = 'auto';
    t.style.height = t.scrollHeight + 4 + 'px';
  }

  function editCard(el, j, i, k, isNew) {
    var card = state.lanes[j].cells[i][k];
    var area = h('textarea', { class: 'editor', rows: 2, placeholder: S.cardHint, title: S.cardHint });
    area.value = card.title + (card.desc ? '\n' + card.desc : '');
    el.replaceWith(area);
    busy++;
    var finished = false;
    function finish(save) {
      if (finished) return;
      finished = true;
      var lines = area.value.split('\n');
      var t = lines.shift().trim();
      var d = lines.join('\n').replace(/\s+$/, '');
      var empty = !t && !d.trim();
      if (save && !(empty && isNew)) {
        if (empty) {
          state.lanes[j].cells[i].splice(k, 1);
        } else {
          card.title = t;
          card.desc = d;
        }
        busy--;
        commit();
        flush();
      } else {
        if (isNew) state.lanes[j].cells[i].splice(k, 1);
        busy--;
        render();
        flush();
      }
    }
    area.addEventListener('input', function () { autosize(area); });
    area.addEventListener('keydown', function (e) {
      e.stopPropagation();
      if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); finish(true); }
      else if (e.key === 'Escape') { e.preventDefault(); finish(false); }
    });
    area.addEventListener('blur', function () { finish(true); });
    area.addEventListener('pointerdown', function (e) { e.stopPropagation(); });
    autosize(area);
    area.focus();
    if (!isNew) area.setSelectionRange(card.title.length, card.title.length);
  }

  function editInline(el, value, apply) {
    var input = h('input', { class: 'inline', type: 'text' });
    input.value = value || '';
    el.replaceWith(input);
    busy++;
    var finished = false;
    function finish(save) {
      if (finished) return;
      finished = true;
      busy--;
      var v = input.value.trim();
      if (save && v !== (value || '')) {
        apply(v);
        commit();
      } else {
        render();
      }
      flush();
    }
    input.addEventListener('keydown', function (e) {
      e.stopPropagation();
      if (e.key === 'Enter') { e.preventDefault(); finish(true); }
      else if (e.key === 'Escape') { e.preventDefault(); finish(false); }
    });
    input.addEventListener('blur', function () { finish(true); });
    input.addEventListener('pointerdown', function (e) { e.stopPropagation(); });
    input.focus();
    input.select();
  }

  function renameColumn(i) {
    var el = app.querySelector('.colhead[data-col="' + i + '"] .name');
    if (el) editInline(el, state.columns[i].title, function (v) { state.columns[i].title = v; });
  }

  function renameLane(j) {
    var el = app.querySelector('.lanehead[data-lane="' + j + '"] .name');
    if (el) editInline(el, state.lanes[j].title, function (v) { state.lanes[j].title = v; });
  }

  function addCard(j, i) {
    var cards = state.lanes[j].cells[i];
    cards.push({ title: '', desc: '', done: false });
    if (state.lanes[j].collapsed) state.lanes[j].collapsed = false;
    render();
    var el = app.querySelector('.card[data-lane="' + j + '"][data-col="' + i + '"][data-idx="' + (cards.length - 1) + '"]');
    if (el) editCard(el, j, i, cards.length - 1, true);
  }

  function addColumn() {
    var n = state.columns.length;
    state.columns.push({ title: S.newColumn, color: PALETTE[n % PALETTE.length], wip: 0 });
    state.lanes.forEach(function (l) { l.cells.push([]); });
    commit();
    renameColumn(n);
    var head = app.querySelector('.colhead[data-col="' + n + '"]');
    if (head) head.scrollIntoView({ inline: 'end', block: 'nearest' });
  }

  function laneName(n) {
    return (S.newLane || 'Swimlane {}').replace('{}', n);
  }

  function addLane() {
    if (state.lanes.length === 1 && !state.lanes[0].title) state.lanes[0].title = laneName(1);
    state.lanes.push({ title: laneName(state.lanes.length + 1), collapsed: false, cells: emptyCells() });
    commit();
    renameLane(state.lanes.length - 1);
  }

  function moveColumn(from, to) {
    if (to < 0 || to >= state.columns.length || to === from) return;
    var c = state.columns.splice(from, 1)[0];
    state.columns.splice(to, 0, c);
    state.lanes.forEach(function (l) {
      var cell = l.cells.splice(from, 1)[0];
      l.cells.splice(to, 0, cell);
    });
  }

  function moveLane(from, to) {
    if (to < 0 || to >= state.lanes.length || to === from) return;
    var l = state.lanes.splice(from, 1)[0];
    state.lanes.splice(to, 0, l);
  }

  // ----- menus -----------------------------------------------------------

  function closeMenu() {
    if (menu) { menu.remove(); menu = null; }
  }

  function openMenu(anchor, items) {
    closeMenu();
    menu = h('div', { class: 'menu' });
    items.forEach(function (it) { if (it) menu.append(it); });
    menu.addEventListener('pointerdown', function (e) { e.stopPropagation(); });
    document.body.append(menu);
    var r = anchor.getBoundingClientRect();
    var mw = menu.offsetWidth, mh = menu.offsetHeight;
    var x = Math.min(r.right - mw, window.innerWidth - mw - 8);
    var y = r.bottom + 4;
    if (y + mh > window.innerHeight - 8) y = Math.max(8, r.top - mh - 4);
    menu.style.left = Math.max(8, x) + 'px';
    menu.style.top = y + 'px';
  }

  function item(label, onClick, opts) {
    opts = opts || {};
    var b = h('button', { class: 'item' + (opts.danger ? ' danger' : ''), type: 'button' }, label);
    if (opts.disabled) b.setAttribute('disabled', '');
    b.addEventListener('click', function (e) { e.stopPropagation(); if (!opts.disabled) onClick(b); });
    return b;
  }

  /// A delete item that asks for a second click.
  function deleteItem(onDelete, needsConfirm) {
    var armed = false;
    return item(S.delete, function (b) {
      if (!needsConfirm || armed) { onDelete(); return; }
      armed = true;
      b.textContent = S.confirmDelete;
    }, { danger: true });
  }

  function columnMenu(i, anchor) {
    var col = state.columns[i];
    var swatches = h('div', { class: 'swatches', title: S.color });
    PALETTE.forEach(function (c) {
      var s = h('button', { class: 'swatch' + (c === col.color ? ' on' : ''), type: 'button', style: 'background:' + c, title: c });
      s.addEventListener('click', function () { col.color = c; commit(); });
      swatches.append(s);
    });
    var custom = h('label', { class: 'swatch custom', title: S.color });
    var picker = h('input', { type: 'color' });
    picker.value = /^#[0-9a-fA-F]{6}$/.test(col.color) ? col.color : '#62a0ea';
    picker.addEventListener('change', function () { col.color = picker.value; commit(); });
    custom.append(picker);
    swatches.append(custom);
    var wip = h('input', { type: 'number', min: 0, step: 1 });
    wip.value = col.wip || 0;
    wip.addEventListener('keydown', function (e) { e.stopPropagation(); if (e.key === 'Enter') wip.blur(); });
    wip.addEventListener('change', function () { col.wip = Math.max(0, parseInt(wip.value, 10) || 0); commit(); });
    var count = state.lanes.reduce(function (a, l) { return a + l.cells[i].length; }, 0);
    openMenu(anchor, [
      item(S.rename, function () { closeMenu(); renameColumn(i); }),
      h('hr'),
      h('div', { class: 'row' }, S.color),
      swatches,
      h('div', { class: 'row' }, S.wip, wip),
      h('hr'),
      item('← ' + S.moveLeft, function () { moveColumn(i, i - 1); commit(); }, { disabled: i === 0 }),
      item('→ ' + S.moveRight, function () { moveColumn(i, i + 1); commit(); }, { disabled: i === state.columns.length - 1 }),
      h('hr'),
      deleteItem(function () {
        state.columns.splice(i, 1);
        state.lanes.forEach(function (l) { l.cells.splice(i, 1); });
        commit();
      }, count > 0)
    ]);
  }

  function laneMenu(j, anchor) {
    var lane = state.lanes[j];
    openMenu(anchor, [
      item(S.rename, function () { closeMenu(); renameLane(j); }),
      h('hr'),
      item('↑ ' + S.moveUp, function () { moveLane(j, j - 1); commit(); }, { disabled: j === 0 }),
      item('↓ ' + S.moveDown, function () { moveLane(j, j + 1); commit(); }, { disabled: j === state.lanes.length - 1 }),
      h('hr'),
      deleteItem(function () {
        state.lanes.splice(j, 1);
        if (!state.lanes.length) state.lanes.push({ title: '', collapsed: false, cells: emptyCells() });
        commit();
      }, cardCount(lane) > 0)
    ]);
  }

  // ----- drag and drop ---------------------------------------------------

  /// Tracks a press on `el`: below the threshold it is a click, beyond it a drag.
  function track(e, handlers) {
    if (e.button !== 0 || busy) return;
    if (e.target.closest('button, input, textarea')) return;
    e.preventDefault();
    closeMenu();
    var sx = e.clientX, sy = e.clientY;
    var dragging = false;
    var scroll = { x: 0, y: 0, raf: 0 };
    function autoscroll() {
      if (scroll.x || scroll.y) window.scrollBy(scroll.x, scroll.y);
      scroll.raf = requestAnimationFrame(autoscroll);
    }
    function move(ev) {
      if (!dragging) {
        if (Math.abs(ev.clientX - sx) + Math.abs(ev.clientY - sy) < 6) return;
        dragging = true;
        busy++;
        document.body.classList.add('dragging');
        handlers.begin(ev);
        scroll.raf = requestAnimationFrame(autoscroll);
      }
      var edge = 48;
      scroll.x = ev.clientX < edge ? -12 : ev.clientX > window.innerWidth - edge ? 12 : 0;
      scroll.y = ev.clientY < edge ? -12 : ev.clientY > window.innerHeight - edge ? 12 : 0;
      handlers.move(ev);
    }
    function stop() {
      window.removeEventListener('pointermove', move, true);
      window.removeEventListener('pointerup', up, true);
      window.removeEventListener('pointercancel', cancel, true);
      cancelAnimationFrame(scroll.raf);
      document.body.classList.remove('dragging');
      activeDrag = null;
    }
    function up(ev) {
      stop();
      if (!dragging) { if (handlers.click) handlers.click(); return; }
      busy--;
      handlers.drop(ev);
      flush();
    }
    function cancel() {
      stop();
      if (!dragging) return;
      busy--;
      handlers.cancel();
      flush();
    }
    activeDrag = { cancel: cancel };
    window.addEventListener('pointermove', move, true);
    window.addEventListener('pointerup', up, true);
    window.addEventListener('pointercancel', cancel, true);
  }

  function ghostOf(el, ev) {
    var r = el.getBoundingClientRect();
    var g = el.cloneNode(true);
    g.classList.add('ghost');
    g.classList.remove('placeholder');
    g.style.width = r.width + 'px';
    g.style.left = r.left + 'px';
    g.style.top = r.top + 'px';
    document.body.append(g);
    return { el: g, dx: ev.clientX - r.left, dy: ev.clientY - r.top };
  }

  function moveGhost(g, ev) {
    g.el.style.left = ev.clientX - g.dx + 'px';
    g.el.style.top = ev.clientY - g.dy + 'px';
  }

  function startCardDrag(e, el, j, i, k) {
    var ghost = null, lastCell = null;
    track(e, {
      click: function () { editCard(el, j, i, k, false); },
      begin: function (ev) {
        ghost = ghostOf(el, ev);
        el.classList.add('placeholder');
      },
      move: function (ev) {
        moveGhost(ghost, ev);
        var t = document.elementFromPoint(ev.clientX, ev.clientY);
        var cell = t && t.closest('.cell');
        if (!cell) return;
        if (lastCell !== cell) {
          if (lastCell) lastCell.classList.remove('target');
          cell.classList.add('target');
          lastCell = cell;
        }
        var before = null;
        var cards = cell.querySelectorAll('.card');
        for (var n = 0; n < cards.length; n++) {
          if (cards[n] === el) continue;
          var r = cards[n].getBoundingClientRect();
          if (ev.clientY < r.top + r.height / 2) { before = cards[n]; break; }
        }
        var ref = before || cell.querySelector('.add');
        if (el.nextSibling !== ref || el.parentNode !== cell) cell.insertBefore(el, ref);
      },
      drop: function () {
        ghost.el.remove();
        var cell = el.parentNode;
        var tj = +cell.dataset.lane, ti = +cell.dataset.col;
        var index = Array.prototype.indexOf.call(cell.querySelectorAll('.card'), el);
        var card = state.lanes[j].cells[i].splice(k, 1)[0];
        state.lanes[tj].cells[ti].splice(index, 0, card);
        commit();
      },
      cancel: function () {
        if (ghost) ghost.el.remove();
        render();
      }
    });
  }

  function marker(rect, vertical) {
    var m = document.querySelector('.marker') || document.body.appendChild(h('div', { class: 'marker' }));
    if (vertical) {
      m.style.left = rect.x - 2 + 'px'; m.style.top = rect.top + 'px';
      m.style.width = '4px'; m.style.height = rect.height + 'px';
    } else {
      m.style.left = rect.left + 'px'; m.style.top = rect.y - 2 + 'px';
      m.style.width = rect.width + 'px'; m.style.height = '4px';
    }
  }

  function clearMarker() {
    var m = document.querySelector('.marker');
    if (m) m.remove();
  }

  /// Index where a dragged item lands among `els` (by their midpoints).
  function slot(els, pos, vertical) {
    for (var n = 0; n < els.length; n++) {
      var r = els[n].getBoundingClientRect();
      if (pos < (vertical ? r.left + r.width / 2 : r.top + r.height / 2)) return n;
    }
    return els.length;
  }

  /// Calls `f` on the second of two clicks on the same thing.
  var lastClick = { key: '', time: 0 };
  function doubleClick(key, f) {
    var now = Date.now();
    if (lastClick.key === key && now - lastClick.time < 450) {
      lastClick.key = '';
      f();
    } else {
      lastClick = { key: key, time: now };
    }
  }

  function startColumnDrag(e, i, head) {
    var ghost = null, target = i;
    track(e, {
      click: function () { doubleClick('c' + i, function () { renameColumn(i); }); },
      begin: function (ev) { ghost = ghostOf(head, ev); head.style.opacity = '.4'; },
      move: function (ev) {
        moveGhost(ghost, ev);
        var heads = app.querySelectorAll('.colhead');
        target = slot(heads, ev.clientX, true);
        var grid = app.querySelector('.grid').getBoundingClientRect();
        var x = target < heads.length ? heads[target].getBoundingClientRect().left - 5
          : heads[heads.length - 1].getBoundingClientRect().right + 5;
        marker({ x: x, top: grid.top, height: Math.min(grid.height, window.innerHeight - grid.top) }, true);
      },
      drop: function () {
        ghost.el.remove();
        clearMarker();
        moveColumn(i, target > i ? target - 1 : target);
        commit();
      },
      cancel: function () { if (ghost) ghost.el.remove(); clearMarker(); render(); }
    });
  }

  function startLaneDrag(e, j, head) {
    var ghost = null, target = j;
    track(e, {
      click: function () { doubleClick('l' + j, function () { renameLane(j); }); },
      begin: function (ev) { ghost = ghostOf(head, ev); head.style.opacity = '.4'; },
      move: function (ev) {
        moveGhost(ghost, ev);
        var heads = app.querySelectorAll('.lanehead');
        target = slot(heads, ev.clientY, false);
        var grid = app.querySelector('.grid').getBoundingClientRect();
        var y;
        if (target < heads.length) {
          y = heads[target].getBoundingClientRect().top;
        } else {
          y = grid.bottom;
        }
        marker({ left: Math.max(grid.left, 0), y: y, width: Math.min(grid.width, window.innerWidth) }, false);
      },
      drop: function () {
        ghost.el.remove();
        clearMarker();
        moveLane(j, target > j ? target - 1 : target);
        commit();
      },
      cancel: function () { if (ghost) ghost.el.remove(); clearMarker(); render(); }
    });
  }

  // ----- global keys and clicks -----------------------------------------

  document.addEventListener('pointerdown', function () { closeMenu(); });
  window.addEventListener('scroll', function () { closeMenu(); }, true);
  document.addEventListener('keydown', function (e) {
    var tag = e.target && e.target.tagName;
    if (e.key === 'Escape') {
      if (activeDrag) activeDrag.cancel();
      closeMenu();
      return;
    }
    if (tag === 'INPUT' || tag === 'TEXTAREA') return;
    var ctrl = e.ctrlKey || e.metaKey;
    if (ctrl && !e.altKey && (e.key === 'z' || e.key === 'Z')) {
      e.preventDefault();
      post({ type: e.shiftKey ? 'redo' : 'undo' });
    } else if (ctrl && !e.altKey && (e.key === 'y' || e.key === 'Y')) {
      e.preventDefault();
      post({ type: 'redo' });
    }
  });
})();
