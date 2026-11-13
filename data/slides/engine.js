/* QuickNotes presentation engine: shows the slides built by src/slides.rs.
 *
 * Modes: preview (editor side pane), audience, presenter, export (stand-alone
 * HTML file) and print (PDF). Audience and presenter view run in separate
 * web views; every navigation is sent to the app (window.webkit
 * .messageHandlers.qn), which forwards it to the other view (QN.receive).
 */
(function () {
  'use strict';

  var QN = (window.QN = {});
  var deck = null;
  var mode = 'preview';
  var L = {};
  var opts = {};
  var state = { slide: 0, step: 0, blank: '' };
  var player = null;

  function post(msg) {
    try {
      window.webkit.messageHandlers.qn.postMessage(JSON.stringify(msg));
    } catch (e) {
      /* not inside QuickNotes (exported file) */
    }
  }

  function el(tag, cls, parent, text) {
    var e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text !== undefined) e.textContent = text;
    if (parent) parent.appendChild(e);
    return e;
  }

  function clamp(v, lo, hi) {
    return Math.max(lo, Math.min(hi, v));
  }

  function count() {
    return deck && deck.slides ? deck.slides.length : 0;
  }

  // ----- slides -------------------------------------------------------------

  /** Creates the <section> of slide i and marks its steps. */
  function buildSection(i) {
    var t = document.createElement('template');
    t.innerHTML = deck.slides[i].html.trim();
    var sec = t.content.firstElementChild;
    if (window.qnRender) qnRender(sec);
    sec.qnSteps = prepareSteps(sec);
    return sec;
  }

  /** Elements that appear one after another, in document order. */
  function prepareSteps(sec) {
    var defAnim = sec.dataset.stepAnim || 'fade';
    var defMs = sec.dataset.stepMs || '400';
    var boxes = sec.querySelectorAll('[data-qn-step], [data-qn-steps]');
    Array.prototype.forEach.call(boxes, function (box) {
      var targets = [];
      if (box.hasAttribute('data-qn-step')) {
        targets.push(box);
      } else {
        Array.prototype.forEach.call(box.children, function (child) {
          if (child.tagName === 'UL' || child.tagName === 'OL') {
            Array.prototype.forEach.call(child.children, function (li) {
              if (li.tagName === 'LI') targets.push(li);
            });
          } else {
            targets.push(child);
          }
        });
      }
      targets.forEach(function (t) {
        t.classList.add('qn-step');
        t.dataset.anim = box.dataset.anim || defAnim;
        t.dataset.group = box.dataset.group || '';
        t.style.setProperty('--qn-dur', (box.dataset.ms || defMs) + 'ms');
        t.style.setProperty('--qn-delay', (box.dataset.delay || '0') + 'ms');
        if (box.hasAttribute('data-dim')) t.dataset.dim = '1';
      });
    });
    return Array.prototype.slice.call(sec.querySelectorAll('.qn-step'));
  }

  /** Shows the first n steps; `play` is the index of the step to animate. */
  function applySteps(sec, n, play, noDim) {
    var steps = sec.qnSteps;
    steps.forEach(function (s, i) {
      s.classList.toggle('qn-shown', i < n);
      var dimmed = false;
      if (s.dataset.dim && !noDim && i < n - 1) {
        for (var j = i + 1; j < n; j++) {
          if (steps[j].dataset.group === s.dataset.group) {
            dimmed = true;
            break;
          }
        }
      }
      s.classList.toggle('qn-dimmed', dimmed);
      s.classList.remove('qn-play');
      if (i === play && s.dataset.anim !== 'none') {
        void s.offsetWidth; // restart the animation
        s.classList.add('qn-play');
      }
    });
  }

  function stepDuration(s) {
    return (parseFloat(s.style.getPropertyValue('--qn-dur')) || 0) + (parseFloat(s.style.getPropertyValue('--qn-delay')) || 0);
  }

  // ----- viewport: fixed-size slide scaled to its box -------------------------

  function Viewport(parent, cls) {
    this.root = el('div', 'qn-viewport ' + (cls || ''), parent);
    this.frame = el('div', 'qn-frame', this.root);
    this.setSize();
    var self = this;
    if (window.ResizeObserver) new ResizeObserver(function () { self.fit(); }).observe(this.root);
    window.addEventListener('resize', function () { self.fit(); });
  }
  Viewport.prototype.setSize = function () {
    this.frame.style.width = deck.w + 'px';
    this.frame.style.height = deck.h + 'px';
    this.frame.style.setProperty('--qn-h', deck.h + 'px');
    if (this.root.classList.contains('qn-ratio')) this.root.style.aspectRatio = deck.w + ' / ' + deck.h;
    this.fit();
  };
  Viewport.prototype.fit = function () {
    var w = this.root.clientWidth, h = this.root.clientHeight;
    if (!w || !h) return;
    var s = Math.min(w / deck.w, h / deck.h);
    this.frame.style.transform = 'translate(-50%, -50%) scale(' + s + ')';
  };

  // ----- player: all slides of the deck with transitions ---------------------

  function Player(vp, animate) {
    this.vp = vp;
    this.animate = animate;
    this.secs = [];
    this.cur = -1;
    this.step = 0;
    this.anim = null;
    this.timer = 0;
    var self = this;
    this.secs = deck.slides.map(function (_, i) {
      var s = buildSection(i);
      self.vp.frame.appendChild(s);
      if (window.qnDraw) qnDraw(s);
      return s;
    });
  }
  Player.prototype.steps = function (i) {
    return this.secs[i] ? this.secs[i].qnSteps.length : 0;
  };
  Player.prototype.finish = function () {
    clearTimeout(this.timer);
    (this.anim || []).forEach(function (s) {
      s.classList.remove('qn-out', 'qn-enter', 'qn-leave', 'qn-rev');
      delete s.dataset.trans;
    });
    this.anim = null;
  };
  Player.prototype.show = function (slide, step, animate) {
    if (!this.secs.length) return;
    animate = animate && this.animate;
    this.finish();
    var prev = this.cur;
    var sec = this.secs[slide];
    var playStep = -1;
    if (slide !== prev) {
      sec.classList.add('qn-current');
      if (prev >= 0 && this.secs[prev]) {
        var old = this.secs[prev];
        old.classList.remove('qn-current');
        if (animate) {
          var forward = slide > prev;
          this.transition(old, sec, forward ? deck.slides[slide].t : deck.slides[prev].t, forward);
        }
      }
      this.cur = slide;
    } else if (animate && step === this.step + 1) {
      playStep = step - 1;
    }
    applySteps(sec, step, playStep);
    this.step = step;
  };
  Player.prototype.transition = function (old, next, t, forward) {
    if (!t || t.name === 'none' || !t.ms) return;
    [old, next].forEach(function (s) {
      s.style.setProperty('--qn-tms', t.ms + 'ms');
      s.dataset.trans = t.name;
    });
    old.classList.add('qn-out');
    if (forward) {
      next.classList.add('qn-enter');
      old.classList.add('qn-leave');
    } else {
      old.classList.add('qn-enter', 'qn-rev');
      next.classList.add('qn-leave', 'qn-rev');
    }
    this.anim = [old, next];
    var self = this;
    this.timer = setTimeout(function () { self.finish(); }, t.ms + 80);
  };

  /** A single slide at a given step, without animations (next-slide view). */
  function StaticView(vp) {
    this.vp = vp;
    this.key = '';
  }
  StaticView.prototype.render = function (slide, step) {
    var key = slide + ':' + step;
    if (key === this.key) return;
    this.key = key;
    this.vp.frame.textContent = '';
    if (slide < 0 || slide >= count()) return;
    var sec = buildSection(slide);
    sec.classList.add('qn-current');
    this.vp.frame.appendChild(sec);
    if (window.qnDraw) qnDraw(sec);
    applySteps(sec, step < 0 ? sec.qnSteps.length : step, -1);
  };

  // ----- navigation (audience, presenter, export) ------------------------------

  function go(slide, step, animate, remote) {
    var n = count();
    if (!n) return;
    slide = clamp(slide, 0, n - 1);
    step = clamp(step, 0, player.steps(slide));
    state.slide = slide;
    state.step = step;
    player.show(slide, step, animate);
    if (mode === 'presenter') updatePresenter();
    if (mode === 'export') {
      try { history.replaceState(null, '', '#' + (slide + 1)); } catch (e) { /* file:// */ }
    }
    if (!remote) post({ type: 'state', slide: slide, step: step, blank: state.blank });
  }

  function next() {
    if (state.blank) return setBlank('');
    if (state.step < player.steps(state.slide)) go(state.slide, state.step + 1, true);
    else if (state.slide + 1 < count()) go(state.slide + 1, 0, true);
  }

  function prev() {
    if (state.blank) return setBlank('');
    if (state.step > 0) go(state.slide, state.step - 1, false);
    else if (state.slide > 0) go(state.slide - 1, player.steps(state.slide - 1), true);
  }

  var blankEl = null;
  function setBlank(kind, remote) {
    state.blank = kind || '';
    if (blankEl) {
      blankEl.classList.toggle('qn-on', !!state.blank);
      blankEl.classList.toggle('qn-white', state.blank === 'white');
    }
    if (mode === 'presenter') updatePresenter();
    if (!remote) post({ type: 'state', slide: state.slide, step: state.step, blank: state.blank });
  }

  QN.receive = function (msg) {
    if (typeof msg === 'string') msg = JSON.parse(msg);
    if (!player) return;
    if (msg.type === 'state') {
      if ((msg.blank || '') !== state.blank) setBlank(msg.blank, true);
      if (msg.slide !== state.slide || msg.step !== state.step) {
        var forward = msg.slide > state.slide || (msg.slide === state.slide && msg.step > state.step);
        go(msg.slide, msg.step, forward || msg.slide !== state.slide, true);
      }
    } else if (msg.type === 'timer' && mode === 'presenter') {
      timerAction(msg.action);
    } else if (msg.type === 'notes' && mode === 'presenter') {
      notesSize(msg.delta);
    }
  };

  var typed = '';
  var gotoEl = null;
  var helpEl = null;

  function onKey(e) {
    if (e.ctrlKey || e.altKey || e.metaKey) return;
    var k = e.key;
    if (/^[0-9]$/.test(k)) {
      typed = (typed + k).slice(-4);
      if (gotoEl) {
        gotoEl.textContent = L.goto + ' ' + typed;
        gotoEl.classList.add('qn-on');
      }
      e.preventDefault();
      return;
    }
    if (typed && k === 'Enter') {
      go(parseInt(typed, 10) - 1, 0, true);
      typed = '';
      if (gotoEl) gotoEl.classList.remove('qn-on');
      e.preventDefault();
      return;
    }
    if (typed) {
      typed = '';
      if (gotoEl) gotoEl.classList.remove('qn-on');
      if (k === 'Escape') return e.preventDefault();
    }
    switch (k) {
      case 'ArrowRight': case 'ArrowDown': case 'PageDown': case ' ': case 'Enter': case 'n': case 'N':
        next(); break;
      case 'ArrowLeft': case 'ArrowUp': case 'PageUp': case 'Backspace':
        prev(); break;
      case 'Home':
        go(0, 0, true); break;
      case 'End':
        go(count() - 1, player.steps(count() - 1), true); break;
      case 'b': case 'B': case '.':
        setBlank(state.blank === 'black' ? '' : 'black'); break;
      case 'w': case 'W': case ',':
        setBlank(state.blank === 'white' ? '' : 'white'); break;
      case 'p': case 'P':
        post({ type: 'toggle-presenter' }); break;
      case 's': case 'S':
        post({ type: 'swap' }); break;
      case 'f': case 'F':
        if (mode === 'export') toggleBrowserFullscreen();
        else post({ type: 'fullscreen' });
        break;
      case 't': case 'T':
        if (mode === 'presenter') timerAction('toggle');
        else post({ type: 'timer', action: 'toggle' });
        break;
      case 'r': case 'R':
        if (mode === 'presenter') timerAction('reset');
        else post({ type: 'timer', action: 'reset' });
        break;
      case '+':
        if (mode === 'presenter') notesSize(2);
        else post({ type: 'notes', delta: 2 });
        break;
      case '-':
        if (mode === 'presenter') notesSize(-2);
        else post({ type: 'notes', delta: -2 });
        break;
      case 'h': case 'H': case '?': case 'F1':
        if (helpEl) helpEl.classList.toggle('qn-on');
        break;
      case 'Escape':
        if (helpEl && helpEl.classList.contains('qn-on')) helpEl.classList.remove('qn-on');
        else if (mode === 'export') { if (document.fullscreenElement) document.exitFullscreen(); }
        else post({ type: 'exit' });
        break;
      default:
        return;
    }
    e.preventDefault();
  }

  function toggleBrowserFullscreen() {
    if (document.fullscreenElement) document.exitFullscreen();
    else if (document.documentElement.requestFullscreen) document.documentElement.requestFullscreen();
  }

  function buildHelp() {
    helpEl = el('div', 'qn-help', document.body);
    var table = el('table', '', helpEl);
    (L.keys || []).forEach(function (row) {
      if (mode === 'export' && /^[PST]\b/.test(row[0])) return;
      var tr = el('tr', '', table);
      el('td', '', tr, row[0]);
      el('td', '', tr, row[1]);
    });
    helpEl.addEventListener('click', function () { helpEl.classList.remove('qn-on'); });
  }

  // ----- audience / export ------------------------------------------------------

  function setupAudience() {
    var vp = new Viewport(document.body, 'qn-full');
    player = new Player(vp, true);
    blankEl = el('div', 'qn-blank', document.body);
    gotoEl = el('div', 'qn-goto', document.body);
    buildHelp();
    vp.root.addEventListener('click', function (e) {
      if (e.target.closest && e.target.closest('a')) return;
      next();
    });
    blankEl.addEventListener('click', function () { setBlank(''); });
    document.addEventListener('contextmenu', function (e) {
      e.preventDefault();
      prev();
    });
    // Hide the mouse pointer when it is not moved.
    var hideTimer = 0;
    document.addEventListener('mousemove', function () {
      document.body.classList.remove('qn-nocursor');
      clearTimeout(hideTimer);
      hideTimer = setTimeout(function () { document.body.classList.add('qn-nocursor'); }, 2000);
    });
    document.body.classList.add('qn-nocursor');
    var start = opts.start || 0;
    if (mode === 'export') {
      var h = parseInt((location.hash || '').slice(1), 10);
      if (h > 0) start = h - 1;
    }
    go(start, 0, false, true);
  }

  // ----- presenter ---------------------------------------------------------------

  var P = {};
  var timer = { start: Date.now(), paused: 0 };

  function setupPresenter() {
    var root = el('div', 'qn-presenter', document.body);
    var bar = el('div', 'qn-bar', root);
    P.timer = el('div', 'qn-timer', bar, '0:00');
    P.timer.title = L.elapsed;
    P.timer.addEventListener('click', function () { timerAction('toggle'); });
    P.remaining = el('div', 'qn-remaining', bar);
    P.clock = el('div', 'qn-clock', bar);
    el('div', 'qn-spacer', bar);
    P.count = el('div', 'qn-count', bar);
    el('div', 'qn-spacer', bar);
    function button(text, title, fn) {
      var b = el('button', '', bar, text);
      b.title = title;
      b.addEventListener('click', function (e) {
        fn();
        e.currentTarget.blur();
      });
      return b;
    }
    button('A−', L.smaller, function () { notesSize(-2); });
    button('A+', L.larger, function () { notesSize(2); });
    button('◄', L.prev, prev);
    button('►', L.nextBtn, next);
    P.blankBtn = button('■', L.blank, function () { setBlank(state.blank ? '' : 'black'); });
    if (opts.screens > 1) button('⇄', L.swap, function () { post({ type: 'swap' }); });
    button('?', L.help, function () { helpEl.classList.toggle('qn-on'); });
    button('✕', L.exit, function () { post({ type: 'exit' }); });

    var main = el('div', 'qn-main', root);
    var left = el('div', 'qn-left', main);
    var cur = el('div', 'qn-cur', left);
    P.state = el('div', 'qn-state', cur);
    P.nextLabel = el('div', 'qn-label', left, L.next);
    P.nextBox = el('div', 'qn-next', left);
    P.notes = el('div', 'qn-notes', main);

    player = new Player(new Viewport(cur), true);
    P.next = new StaticView(new Viewport(P.nextBox));
    P.end = el('div', 'qn-end', P.nextBox, L.end);
    P.vpCur = cur;
    buildHelp();

    var size = 28;
    try { size = parseInt(localStorage.getItem('qn-notes-size'), 10) || 28; } catch (e) { /* no storage */ }
    document.documentElement.style.setProperty('--qn-notes-size', size + 'px');

    cur.addEventListener('click', next);
    setInterval(tick, 250);
    tick();
    go(opts.start || 0, 0, false, true);
  }

  function updatePresenter() {
    var n = count();
    var s = state.slide;
    var steps = player.steps(s);
    P.count.textContent = L.slide + ' ' + (s + 1) + ' / ' + n + (steps ? '  ·  ' + state.step + '/' + steps : '');
    if (state.step < steps) {
      P.next.render(s, state.step + 1);
      P.nextLabel.textContent = L.nextStep + ' ' + (state.step + 1) + '/' + steps;
      P.end.style.display = 'none';
    } else if (s + 1 < n) {
      P.next.render(s + 1, -1);
      var title = deck.slides[s + 1].title;
      P.nextLabel.textContent = L.nextSlide + (title ? ': ' + title : '');
      P.end.style.display = 'none';
    } else {
      P.next.render(-1, 0);
      P.nextLabel.textContent = L.next;
      P.end.style.display = '';
    }
    var notes = deck.slides[s].notes;
    var html = notes || '<p class="qn-nonotes">' + L.noNotes + '</p>';
    if (P.notesHtml !== html) {
      P.notesHtml = html;
      P.notes.innerHTML = html;
      P.notes.scrollTop = 0;
    }
    P.state.textContent = state.blank === 'white' ? L.white : L.black;
    P.state.classList.toggle('qn-on', !!state.blank);
    P.blankBtn.classList.toggle('qn-active', !!state.blank);
  }

  function notesSize(delta) {
    var cur = parseInt(getComputedStyle(document.documentElement).getPropertyValue('--qn-notes-size'), 10) || 28;
    var size = clamp(cur + delta, 14, 72);
    document.documentElement.style.setProperty('--qn-notes-size', size + 'px');
    try { localStorage.setItem('qn-notes-size', String(size)); } catch (e) { /* no storage */ }
  }

  function timerAction(action) {
    var now = Date.now();
    if (action === 'toggle') {
      if (timer.paused) {
        timer.start += now - timer.paused;
        timer.paused = 0;
      } else {
        timer.paused = now;
      }
    } else if (action === 'reset') {
      timer.start = now;
      if (timer.paused) timer.paused = now;
    }
    tick();
  }

  function fmt(sec) {
    sec = Math.floor(Math.abs(sec));
    var h = Math.floor(sec / 3600), m = Math.floor((sec % 3600) / 60), s = sec % 60;
    var mm = h ? String(m).padStart(2, '0') : String(m);
    return (h ? h + ':' : '') + mm + ':' + String(s).padStart(2, '0');
  }

  function tick() {
    if (!P.timer) return;
    var elapsed = ((timer.paused || Date.now()) - timer.start) / 1000;
    P.timer.textContent = fmt(elapsed) + (timer.paused ? ' ⏸' : '');
    P.timer.classList.toggle('qn-paused', !!timer.paused);
    if (deck.duration) {
      var left = deck.duration - elapsed;
      P.remaining.textContent = left >= 0 ? fmt(left) + ' ' + L.remaining : '+' + fmt(left) + ' ' + L.overtime;
      P.remaining.classList.toggle('qn-over', left < 0);
      P.remaining.classList.toggle('qn-warn', left >= 0 && left < Math.min(120, deck.duration * 0.1));
    }
    var d = new Date();
    P.clock.textContent = String(d.getHours()).padStart(2, '0') + ':' + String(d.getMinutes()).padStart(2, '0');
  }

  // ----- preview (editor side pane) ------------------------------------------------

  var pv = null;

  function setupPreview() {
    pv = { index: -1, html: null, timer: 0 };
    pv.root = el('div', 'qn-preview', document.body);
    pv.box = el('div', '', pv.root);
    var bar = el('div', 'qn-preview-bar', pv.root);
    var prevBtn = el('button', '', bar, '‹');
    prevBtn.title = L.prev;
    prevBtn.addEventListener('click', function () { post({ type: 'goto', slide: pv.index - 1 }); });
    pv.counter = el('span', '', bar);
    var nextBtn = el('button', '', bar, '›');
    nextBtn.title = L.nextBtn;
    nextBtn.addEventListener('click', function () { post({ type: 'goto', slide: pv.index + 1 }); });
    el('span', 'qn-spacer', bar);
    pv.info = el('span', '', bar);
    var playBtn = el('button', '', bar, '►');
    playBtn.title = L.play;
    playBtn.addEventListener('click', playPreview);
    pv.notesBox = el('div', 'qn-preview-notes', pv.root);
    el('div', 'qn-label', pv.notesBox, L.notes);
    pv.notes = el('div', '', pv.notesBox);
  }

  QN.update = function (data, index) {
    deck = data;
    var style = document.getElementById('qn-user-style');
    if (style && style.textContent !== (deck.style || '')) style.textContent = deck.style || '';
    if (!pv) setupPreview();
    if (!pv.vp) {
      pv.vp = new Viewport(pv.box, 'qn-ratio');
    } else {
      pv.vp.setSize();
    }
    pv.html = null;
    renderPreview(index);
  };

  QN.showSlide = function (index) {
    if (deck && pv) renderPreview(index);
  };

  function renderPreview(index) {
    clearTimeout(pv.timer);
    var n = count();
    index = clamp(index, 0, Math.max(0, n - 1));
    pv.index = index;
    var html = n ? deck.slides[index].html : '';
    if (pv.html !== html) {
      pv.html = html;
      pv.vp.frame.textContent = '';
      pv.sec = null;
      if (n) {
        var sec = buildSection(index);
        sec.classList.add('qn-current');
        pv.vp.frame.appendChild(sec);
        if (window.qnDraw) qnDraw(sec);
        applySteps(sec, sec.qnSteps.length, -1);
        pv.sec = sec;
      } else {
        el('div', 'qn-empty', pv.vp.frame, L.noSlides).style.cssText = 'position:absolute;inset:0;background:#fff;font-size:28px;padding:200px 80px;';
      }
    }
    pv.counter.textContent = n ? L.slide + ' ' + (index + 1) + ' / ' + n : '';
    var t = n ? deck.slides[index].t : null;
    var steps = pv.sec ? pv.sec.qnSteps.length : 0;
    pv.info.textContent = n ? (t && t.name !== 'none' ? t.name + ' ' + t.ms + 'ms' : '') + (steps ? '  ·  ' + (steps === 1 ? L.oneStep : String(L.steps).replace('{}', steps)) : '') : '';
    var notes = n ? deck.slides[index].notes : '';
    pv.notes.innerHTML = notes;
    pv.notesBox.style.display = notes ? '' : 'none';
  }

  /** Plays the transition and the steps of the previewed slide. */
  function playPreview() {
    var sec = pv.sec;
    if (!sec) return;
    clearTimeout(pv.timer);
    var t = deck.slides[pv.index].t;
    applySteps(sec, 0, -1);
    sec.classList.remove('qn-enter');
    delete sec.dataset.trans;
    var wait = 300;
    if (t && t.name !== 'none' && t.ms) {
      void sec.offsetWidth;
      sec.style.setProperty('--qn-tms', t.ms + 'ms');
      sec.dataset.trans = t.name;
      sec.classList.add('qn-enter');
      wait = t.ms + 300;
    }
    var i = 0;
    var total = sec.qnSteps.length;
    function step() {
      if (i >= total) return;
      applySteps(sec, i + 1, i);
      var d = stepDuration(sec.qnSteps[i]);
      i++;
      pv.timer = setTimeout(step, d + 450);
    }
    pv.timer = setTimeout(function () {
      sec.classList.remove('qn-enter');
      delete sec.dataset.trans;
      step();
    }, wait);
  }

  QN.setTheme = window.setTheme = function (theme) {
    document.documentElement.dataset.theme = theme;
    if (window.qnRetheme) qnRetheme();
  };

  // ----- print -----------------------------------------------------------------

  function setupPrint() {
    deck.slides.forEach(function (_, i) {
      var page = el('div', 'qn-print-page', document.body);
      page.style.width = deck.w + 'px';
      page.style.height = deck.h + 'px';
      page.style.setProperty('--qn-h', deck.h + 'px');
      var sec = buildSection(i);
      page.appendChild(sec);
      if (window.qnDraw) qnDraw(sec);
      applySteps(sec, sec.qnSteps.length, -1, true);
    });
  }

  // ----- start -----------------------------------------------------------------

  QN.init = function (m, data, labels, options) {
    mode = m;
    L = labels || {};
    opts = options || {};
    deck = data;
    if (mode === 'preview') return;
    if (!count()) {
      el('div', 'qn-empty', document.body, L.noSlides).style.color = '#999';
      if (mode !== 'print') document.addEventListener('keydown', function (e) { if (e.key === 'Escape') post({ type: 'exit' }); });
      return;
    }
    if (mode === 'print') return setupPrint();
    if (mode === 'presenter') setupPresenter();
    else setupAudience();
    document.addEventListener('keydown', onKey);
    post({ type: 'ready' });
  };
})();
