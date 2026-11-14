/* QuickNotes GM screen: shows the adventure model from src/adventure.rs
 * (GM.update(model)) and sends actions back ({type: "gm", action: …}) and
 * line jumps ({type: "line"}). The page skeleton (with its inputs) is built
 * once; the parts that depend on the model are rebuilt on every update. */
(function () {
  'use strict';
  var S = window.GM_STRINGS || {};
  var model = null;
  var shownChapter = -1;
  var running = false;
  var since = 0;
  var collapsed = {};

  function post(msg) {
    if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.qn) {
      window.webkit.messageHandlers.qn.postMessage(JSON.stringify(msg));
    }
  }
  function act(action, extra) {
    var msg = { type: 'gm', action: action };
    for (var k in extra || {}) msg[k] = extra[k];
    post(msg);
  }
  function roll(notation, who, label) { act('roll', { notation: notation, who: who || '', label: label || '' }); }
  function el(tag, cls, text) {
    var e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text !== undefined && text !== null) e.textContent = text;
    return e;
  }
  function button(text, title, onclick, cls) {
    var b = el('button', cls, text);
    if (title) b.title = title;
    b.addEventListener('click', onclick);
    return b;
  }
  function amount() {
    var v = parseInt(ui.amount.value, 10);
    return isNaN(v) || v < 1 ? 1 : v;
  }
  function inline(text) {
    var esc = String(text).replace(/&/g, '&amp;').replace(/</g, '&lt;');
    return esc.replace(/\*\*(.+?)\*\*/g, '<b>$1</b>').replace(/~~(.+?)~~/g, '<s>$1</s>');
  }
  var DICE = /^\s*(check\s+\d+(\/\d+){0,2}(\s+[-+]?\d+){0,2}|(\d*[dDwW](\d+|%|F|\{[^}]+\})([a-z]{1,2}\d*|!|[<>]=?\d+)*)(\s*[-+]\s*(\d*[dDwW](\d+|%|F)|\d+))*)\s*$/;

  // ----- the skeleton --------------------------------------------------------
  var ui = {};
  function build() {
    var root = document.getElementById('gm');
    var bar = el('div', 'bar');
    var nav = el('div', 'group');
    nav.appendChild(button('◀', S.prevChapter, function () { go(-1); }));
    ui.chapters = el('select');
    ui.chapters.title = S.chapters;
    ui.chapters.addEventListener('change', function () {
      var c = model && model.chapters[ui.chapters.value];
      if (c) post({ type: 'line', line: c.line });
    });
    nav.appendChild(ui.chapters);
    nav.appendChild(button('▶', S.nextChapter, function () { go(1); }));
    bar.appendChild(nav);

    var play = el('div', 'group');
    play.appendChild(el('span', 'label', '⏱'));
    ui.clock = el('span', 'clock');
    ui.clock.title = S.playTime;
    play.appendChild(ui.clock);
    ui.play = button('▶', S.start, togglePlay);
    play.appendChild(ui.play);
    bar.appendChild(play);

    var world = el('div', 'group');
    world.appendChild(el('span', 'label', '🌍'));
    ui.world = el('span', 'world');
    ui.world.title = S.setTime;
    ui.world.addEventListener('click', editWorld);
    world.appendChild(ui.world);
    [['+10′', 10], ['+1h', 60], [S.rest + ' +8h', 480], [S.day + ' +1', 1440]].forEach(function (b) {
      world.appendChild(button(b[0], S.worldTime, function () { act('time', { minutes: b[1] }); }));
    });
    bar.appendChild(world);
    root.appendChild(bar);

    var layout = el('div', 'layout');
    ui.chapter = el('div', 'chapter');
    layout.appendChild(ui.chapter);
    var side = el('div', 'side');

    var party = el('div', 'panel');
    var ph = el('h2', null, S.party);
    ph.appendChild(el('span', 'spacer'));
    ph.appendChild(el('span', null, S.amount));
    ui.amount = el('input', 'amount');
    ui.amount.type = 'number';
    ui.amount.min = '1';
    ui.amount.value = '1';
    ph.appendChild(ui.amount);
    party.appendChild(ph);
    ui.party = el('div');
    party.appendChild(ui.party);
    side.appendChild(party);

    var dice = el('div', 'panel');
    dice.appendChild(el('h2', null, S.dice));
    ui.result = el('div', 'result');
    dice.appendChild(ui.result);
    var grid = el('div', 'dice');
    ui.count = el('input', 'amount');
    ui.count.type = 'number';
    ui.count.min = '1';
    ui.count.value = '1';
    ui.count.title = S.count;
    ['d4', 'd6', 'd8', 'd10', 'd12', 'd20', 'd%', 'dF'].forEach(function (d) {
      grid.appendChild(button(d, null, function () {
        var n = parseInt(ui.count.value, 10) || 1;
        roll((n > 1 ? n : '') + d);
      }));
    });
    dice.appendChild(grid);
    var row = el('div', 'line');
    row.appendChild(ui.count);
    row.appendChild(button(S.advantage, '2d20kh1', function () { roll('2d20kh1', '', S.advantage); }));
    row.appendChild(button(S.disadvantage, '2d20kl1', function () { roll('2d20kl1', '', S.disadvantage); }));
    dice.appendChild(row);
    var line = el('div', 'line');
    ui.notation = el('input');
    ui.notation.placeholder = S.notation;
    ui.notation.addEventListener('keydown', function (e) { if (e.key === 'Enter') rollInput(); });
    line.appendChild(ui.notation);
    line.appendChild(button(S.roll, null, rollInput, 'primary'));
    dice.appendChild(line);
    ui.custom = el('div', 'line');
    dice.appendChild(ui.custom);
    side.appendChild(dice);

    var init = el('div', 'panel');
    ui.initHead = el('h2', null, S.initiative);
    init.appendChild(ui.initHead);
    ui.init = el('div');
    init.appendChild(ui.init);
    side.appendChild(init);

    var log = el('div', 'panel');
    log.appendChild(el('h2', null, S.log));
    var nl = el('div', 'line');
    ui.note = el('input');
    ui.note.placeholder = S.note;
    ui.note.addEventListener('keydown', function (e) {
      if (e.key === 'Enter' && ui.note.value.trim()) {
        act('note', { text: ui.note.value });
        ui.note.value = '';
      }
    });
    nl.appendChild(ui.note);
    log.appendChild(nl);
    ui.log = el('div', 'log');
    log.appendChild(ui.log);
    side.appendChild(log);

    layout.appendChild(side);
    root.appendChild(layout);
    setInterval(tick, 1000);
  }

  function rollInput() {
    var v = ui.notation.value.trim();
    if (v) roll(v);
  }
  function go(step) {
    if (!model) return;
    var c = model.chapters[model.current + step];
    if (c) post({ type: 'line', line: c.line });
  }

  // ----- timers -------------------------------------------------------------------
  function clockText(secs) {
    var h = Math.floor(secs / 3600), m = Math.floor(secs % 3600 / 60), s = Math.floor(secs % 60);
    return h + ':' + (m < 10 ? '0' : '') + m + ':' + (s < 10 ? '0' : '') + s;
  }
  function elapsed() { return running ? Math.floor((Date.now() - since) / 1000) : 0; }
  function tick() {
    if (!model) return;
    var e = elapsed();
    // Saved every minute, so little is lost when the app is closed.
    if (e >= 60) {
      act('played', { seconds: e });
      since += e * 1000;
      e = 0;
    }
    ui.clock.textContent = clockText(model.played + e);
  }
  function togglePlay() {
    if (running) {
      var e = elapsed();
      running = false;
      if (e > 0) act('played', { seconds: e });
    } else {
      running = true;
      since = Date.now();
    }
    ui.play.textContent = running ? '⏸' : '▶';
    ui.play.title = running ? S.pause : S.start;
    tick();
  }
  function editWorld() {
    var input = el('input');
    input.value = '';
    input.placeholder = '1492-3-15 14:30';
    input.size = 16;
    ui.world.replaceWith(input);
    input.focus();
    function done(save) {
      if (save && input.value.trim()) act('settime', { time: input.value.trim() });
      input.replaceWith(ui.world);
    }
    input.addEventListener('keydown', function (e) {
      if (e.key === 'Enter') done(true);
      if (e.key === 'Escape') done(false);
    });
    input.addEventListener('blur', function () { done(false); });
  }

  // ----- sheets -----------------------------------------------------------------------
  function hpControl(name, hp) {
    var box = el('span', 'hp');
    if (!hp) return box;
    box.appendChild(button('−', S.hpMinus, function (e) { e.stopPropagation(); act('hp', { name: name, delta: -amount() }); }));
    var out = el('span', 'bar-out');
    var inner = el('span', 'bar-in');
    var ratio = hp[1] > 0 ? Math.max(0, Math.min(1, hp[0] / hp[1])) : 0;
    inner.style.width = (ratio * 100) + '%';
    inner.style.display = 'block';
    if (ratio <= 0) inner.className += ' down';
    else if (ratio < 0.5) inner.className += ' low';
    out.appendChild(inner);
    box.appendChild(out);
    box.appendChild(el('span', null, hp[0] + '/' + hp[1]));
    box.appendChild(button('+', S.hpPlus, function (e) { e.stopPropagation(); act('hp', { name: name, delta: amount() }); }));
    return box;
  }

  function sheetCard(i, open) {
    var s = model.sheets[i];
    var key = s.name + '@' + s.line;
    var card = el('div', 'sheet ' + s.kind + ((collapsed[key] === undefined ? !open : collapsed[key]) ? ' collapsed' : ''));
    var head = el('div', 'head');
    var name = el('span', 'name', s.name + (s.count > 1 ? ' ×' + s.count : ''));
    name.addEventListener('click', function () {
      collapsed[key] = !card.classList.contains('collapsed');
      card.classList.toggle('collapsed');
    });
    head.appendChild(name);
    head.appendChild(el('span', 'kind', s.kind));
    s.conditions.forEach(function (c) { head.appendChild(el('span', 'cond', c)); });
    head.appendChild(el('span', 'spacer'));
    if (s.count === 1) head.appendChild(hpControl(s.name, s.hp));
    var jump = button('↗', S.gotoLine, function () { post({ type: 'line', line: s.line }); });
    head.appendChild(jump);
    card.appendChild(head);
    var rows = el('div', 'rows');
    s.fields.forEach(function (f) {
      var r = el('div', 'row');
      r.appendChild(el('span', 'k', f.label));
      r.appendChild(el('span', null, f.value));
      f.rolls.forEach(function (b) {
        var chip = el('span', 'chip', '🎲 ' + b.label);
        chip.title = b.notation;
        chip.addEventListener('click', function () { roll(b.notation, s.name, b.label); });
        r.appendChild(chip);
      });
      rows.appendChild(r);
    });
    var init = el('div', 'row');
    var chip = el('span', 'chip', '🎲 ' + S.initiative + ' (' + s.init + ')');
    chip.addEventListener('click', function () { roll(s.init, s.name, S.initiative); });
    init.appendChild(chip);
    rows.appendChild(init);
    card.appendChild(rows);
    return card;
  }

  // ----- update ---------------------------------------------------------------------------
  function render() {
    if (!model) return;
    // Chapters.
    ui.chapters.innerHTML = '';
    model.chapters.forEach(function (c, i) {
      var o = el('option', null, c.title);
      o.value = i;
      ui.chapters.appendChild(o);
    });
    ui.chapters.value = model.current;
    var chapter = model.chapters[model.current];
    var scroll = shownChapter === model.current ? ui.chapter.scrollTop : 0;
    ui.chapter.innerHTML = '';
    if (chapter) {
      var h = el('h1', null, chapter.title);
      h.title = S.gotoLine;
      h.style.cursor = 'pointer';
      h.addEventListener('click', function () { post({ type: 'line', line: chapter.line }); });
      ui.chapter.appendChild(h);
      var body = el('div');
      body.innerHTML = chapter.html;
      body.querySelectorAll('.sheet-slot').forEach(function (slot) {
        slot.replaceWith(sheetCard(parseInt(slot.dataset.sheet, 10), true));
      });
      body.querySelectorAll('code').forEach(function (c) {
        if (DICE.test(c.textContent)) {
          c.classList.add('roll');
          c.title = S.roll;
          c.addEventListener('click', function () { roll(c.textContent.trim()); });
        }
      });
      ui.chapter.appendChild(body);
    }
    ui.chapter.scrollTop = scroll;
    shownChapter = model.current;

    // Party.
    ui.party.innerHTML = '';
    model.sheets.forEach(function (s, i) {
      if (s.kind === 'player') ui.party.appendChild(sheetCard(i, false));
    });

    // Timers.
    ui.world.textContent = model.world || '—';
    tick();

    // Dice.
    var last = model.log.filter(function (l) { return l.indexOf('→ **') >= 0; })[0];
    ui.result.innerHTML = '';
    if (last) {
      var m = /→ \*\*(.+?)\*\*(.*?)\((.*)\)\s*$/.exec(last);
      var big = el('div', 'big');
      big.innerHTML = inline(m ? m[1] + m[2] : last);
      var small = el('div', 'small');
      small.innerHTML = inline(m ? last.replace(/^\[[^\]]*\]\s*/, '').split(' → ')[0] + ' · ' + m[3] : '');
      ui.result.appendChild(big);
      ui.result.appendChild(small);
    }
    ui.custom.innerHTML = '';
    model.dice.forEach(function (d) {
      ui.custom.appendChild(button(d.name, d.faces.join(', '), function () {
        var n = parseInt(ui.count.value, 10) || 1;
        roll((n > 1 ? n : '') + 'd{' + d.name + '}');
      }));
    });

    // Initiative.
    ui.initHead.textContent = S.initiative + (model.order.length ? ' · ' + S.round + ' ' + model.round : '');
    ui.init.innerHTML = '';
    if (!model.order.length) {
      ui.init.appendChild(el('div', 'hint', S.noInit));
      var line = el('div', 'line');
      line.appendChild(button(S.rollInit, null, function () { act('initiative', { chapter: model.current }); }, 'primary'));
      ui.init.appendChild(line);
    } else {
      model.order.forEach(function (c, i) {
        var r = el('div', 'init-row' + (i === model.turn ? ' active' : ''));
        r.appendChild(el('span', 'n', c.init));
        r.appendChild(el('span', null, c.name));
        r.appendChild(hpControl(c.name, c.hp));
        r.appendChild(button('×', S.remove, function () { act('remove', { name: c.name }); }, 'x'));
        ui.init.appendChild(r);
      });
      var nav = el('div', 'line');
      nav.appendChild(button('◀', S.prev, function () { act('previous'); }));
      nav.appendChild(button(S.next + ' ▶', null, function () { act('next'); }, 'primary'));
      nav.appendChild(button(S.end, null, function () { act('endcombat'); }));
      ui.init.appendChild(nav);
    }

    // Log.
    ui.log.innerHTML = '';
    model.log.forEach(function (l) {
      var d = el('div', /^— .* —$/.test(l) ? 'day' : null);
      d.innerHTML = inline(l);
      ui.log.appendChild(d);
    });
  }

  window.GM = { update: function (m) { model = m; render(); } };
  window.setTheme = function (t, vars) {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.cssText = vars || '';
  };
  build();
})();
