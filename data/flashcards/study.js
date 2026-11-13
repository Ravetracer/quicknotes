// Study window of QuickNotes flash cards. Data: window.QF_DATA (see
// src/flashcards.rs). Answers are sent to the app as
// {type: 'rate', key, rating 1-4, ms, text, practice}.
(function () {
  var D = window.QF_DATA, L = D.labels;
  var cards = D.cards;
  var queue = cards.map(function (_, i) { return i; });
  var first = {};      // key -> right at the first try
  var wrong = [];      // card indices answered with "again"
  var cur = -1, flipped = false, started = 0, began = Date.now(), studied = 0;

  function post(msg) {
    if (window.webkit && window.webkit.messageHandlers.qn) window.webkit.messageHandlers.qn.postMessage(JSON.stringify(msg));
  }
  function el(tag, cls, parent, html) {
    var e = document.createElement(tag);
    if (cls) e.className = cls;
    if (html != null) e.innerHTML = html;
    if (parent) parent.appendChild(e);
    return e;
  }
  function text(s, v) { return String(s).replace('{}', v); }
  function clock(ms) {
    var s = Math.round(ms / 1000);
    return Math.floor(s / 60) + ':' + ('0' + (s % 60)).slice(-2);
  }

  var app = document.getElementById('app');
  var header = el('header', null, app);
  el('span', 'title', header).textContent = D.title + (D.practice ? ' · ' + L.practice : '');
  var bar = el('div', 'progress', header);
  var fill = el('i', null, bar);
  var counter = el('span', null, header);
  var timer = el('span', null, header);
  var main = el('main', null, app);
  var footer = el('footer', null, app);
  var buttons = el('div', 'buttons', footer);
  el('div', 'keys', footer).textContent = L.keys;
  setInterval(function () { timer.textContent = clock(Date.now() - began); }, 1000);
  timer.textContent = clock(0);

  function updateProgress() {
    var left = queue.length + (cur >= 0 && !flipped ? 1 : 0);
    var done = Math.max(0, cards.length - queue.length - 1);
    fill.style.width = (cards.length ? Math.min(100, 100 * done / cards.length) : 100) + '%';
    counter.textContent = text(L.left, left);
  }

  function button(label, cls, onClick, hint) {
    var b = el('button', cls, buttons);
    b.textContent = label;
    if (hint) el('small', null, b).textContent = hint;
    b.addEventListener('click', onClick);
    return b;
  }

  function show() {
    buttons.textContent = '';
    main.textContent = '';
    if (!queue.length) return finish();
    cur = queue.shift();
    flipped = false;
    started = Date.now();
    var c = cards[cur];
    var box = el('div', 'card', main);
    if (c.deck) el('div', 'deck', box).textContent = c.deck;
    el('div', 'front', box, c.front);
    if (c.kind === 'choice') {
      var list = el('ul', 'choices', box);
      c.choices.forEach(function (ch, i) {
        var li = el('li', null, list);
        var label = el('label', null, li);
        var input = el('input', null, label);
        input.type = c.multiple ? 'checkbox' : 'radio';
        input.name = 'choice';
        el('span', 'num', label).textContent = i + 1;
        el('span', null, label, ch[0]);
        el('span', 'mark', label);
      });
      button(L.check, null, flip).focus();
    } else {
      box.classList.add('clickable');
      box.addEventListener('click', function () { flip(); });
      button(L.show, null, flip).focus();
    }
    if (window.qnRender) qnRender(box);
    updateProgress();
  }

  function flip() {
    if (flipped || cur < 0) return;
    flipped = true;
    var c = cards[cur];
    var box = main.querySelector('.card');
    box.classList.remove('clickable');
    var right = null;
    if (c.kind === 'choice') {
      right = true;
      box.querySelectorAll('ul.choices li').forEach(function (li, i) {
        var input = li.querySelector('input');
        var correct = c.choices[i][1];
        if (correct) li.classList.add('ok');
        else if (input.checked) li.classList.add('bad');
        if (input.checked !== correct) right = false;
        input.disabled = true;
      });
      el('div', 'result ' + (right ? 'ok' : 'bad'), box).textContent = right ? L.right : L.wrong;
    }
    if (c.back && c.back.trim()) {
      var back = el('div', 'back', box, c.back);
      if (window.qnRender) qnRender(back);
    }
    buttons.textContent = '';
    var classes = ['again', 'outline', null, 'easy'];
    var focus = null;
    L.ratings.forEach(function (label, i) {
      var b = button((i + 1) + ' · ' + label, classes[i], function () { rate(i + 1); }, c.next[i]);
      if ((right === false && i === 0) || (right !== false && i === 2)) focus = b;
    });
    if (focus) focus.focus();
    updateProgress();
  }

  function rate(r) {
    if (!flipped || cur < 0) return;
    var c = cards[cur];
    post({ type: 'rate', key: c.key, rating: r, ms: Date.now() - started, text: c.text, practice: !!D.practice });
    if (!(c.key in first)) { first[c.key] = r > 1; studied++; }
    if (r === 1) {
      queue.splice(Math.min(queue.length, 3), 0, cur);
      if (wrong.indexOf(cur) < 0) wrong.push(cur);
    }
    cur = -1;
    show();
  }

  function finish() {
    cur = -1;
    fill.style.width = '100%';
    counter.textContent = '';
    var box = el('div', 'card summary', main);
    if (!cards.length) {
      el('p', null, box).textContent = L.noCards;
    } else {
      el('h1', null, box).textContent = L.done;
      var right = Object.keys(first).filter(function (k) { return first[k]; }).length;
      var s = L.summary.replace('{}', studied).replace('{}', right).replace('{}', clock(Date.now() - began));
      el('p', null, box).textContent = s;
    }
    if (wrong.length) {
      button(L.repeat, null, function () {
        D.practice = true;
        queue = wrong.slice();
        wrong = [];
        show();
      }).focus();
    }
    var close = button(L.close, wrong.length ? 'outline' : null, function () { post({ type: 'close' }); });
    if (!wrong.length) close.focus();
  }

  document.addEventListener('keydown', function (e) {
    if (e.ctrlKey || e.altKey || e.metaKey) return;
    if (e.key === 'Escape') { post({ type: 'close' }); return; }
    if (e.key === 'F11') { e.preventDefault(); post({ type: 'fullscreen' }); return; }
    if (cur < 0) return;
    var c = cards[cur];
    if (!flipped) {
      if (e.key === ' ' || e.key === 'Enter') { e.preventDefault(); flip(); return; }
      var n = parseInt(e.key, 10);
      if (c.kind === 'choice' && n >= 1 && n <= c.choices.length) {
        var input = main.querySelectorAll('ul.choices input')[n - 1];
        input.checked = c.multiple ? !input.checked : true;
        e.preventDefault();
      }
      return;
    }
    if (e.key >= '1' && e.key <= '4') { e.preventDefault(); rate(+e.key); }
    else if (e.key === ' ') { e.preventDefault(); rate(3); }
  });

  show();
})();
