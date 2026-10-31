// Suggestions while a filter is typed: names, operators and connectors from what the filter knows,
// for the connection filter also the values of the capture. The text before the cursor says what
// can come next. Enter still applies the filter; only a line chosen with the arrow keys is taken.
(function () {
  'use strict';
  var inputs = document.querySelectorAll('[data-filter-input]');
  if (!inputs.length) { return; }

  var CONNECTORS = ['and', 'or'];
  var OPERATORS = /^(==|=|eq|!=|ne|<|lt|<=|le|>|gt|>=|ge|contains|in)$/;
  var LIMIT = 60;
  var VALUES_DELAY = 120;

  // ---- what the filter knows -------------------------------------------------------------------
  var catalogs = {};
  function catalog(kind) {
    if (!catalogs[kind]) {
      catalogs[kind] = fetch('/api/filter/names' + (kind ? '?kind=' + kind : ''), { credentials: 'same-origin' })
        .then(function (r) { if (!r.ok) { throw new Error(r.status); } return r.json(); })
        .then(function (c) {
          c.byName = {};
          c.names.forEach(function (n) { c.byName[n.n] = n; });
          return c;
        })
        .catch(function (e) { delete catalogs[kind]; throw e; });
    }
    return catalogs[kind];
  }

  // ---- the text before the cursor --------------------------------------------------------------
  // the same words as the tokenizer of the filter (src/filter/mod.rs)
  function tokens(text) {
    var list = [], i = 0;
    while (i < text.length) {
      var c = text[i];
      if (/[\s,]/.test(c)) { i++; continue; }
      if (c === '"') {
        var j = i + 1;
        while (j < text.length && text[j] !== '"') { j += text[j] === '\\' ? 2 : 1; }
        if (j >= text.length) {
          list.push({ type: 'open', text: text.slice(i), start: i, end: text.length });
          return list;
        }
        list.push({ type: 'string', text: text.slice(i, j + 1), start: i, end: j + 1 });
        i = j + 1;
        continue;
      }
      var rest = text.slice(i);
      var match = /^(==|!=|<=|>=|&&|\|\||[=<>!(){}])/.exec(rest);
      if (match) {
        list.push({ type: 'symbol', text: match[1], start: i, end: i + match[1].length });
      } else {
        match = /^[^\s"(){}=!<>&|,]+/.exec(rest);
        if (!match) { i++; continue; }
        list.push({ type: 'word', text: match[0], start: i, end: i + match[0].length });
      }
      i += match[0].length;
    }
    return list;
  }

  // Follows the grammar over the finished words: `term` (a name), `operator`, `brace` (after `in`),
  // `value`, `set` (the values of `in`) or `next` (and, or)
  function context(before, known) {
    var list = tokens(before);
    var last = list[list.length - 1];
    var partial = '', start = before.length;
    if (last && last.end === before.length && (last.type === 'word' || last.type === 'open')) {
      list.pop();
      partial = last.text;
      start = last.start;
    }
    var state = 'term', field = null, depth = 0;
    list.forEach(function (token) {
      var word = token.type === 'word' ? token.text.toLowerCase() : token.text;
      var connector = word === 'and' || word === 'or' || word === '&&' || word === '||';
      if (state === 'term') {
        if (word === '(') { depth++; return; }
        if (word === 'not' || word === '!') { return; }
        field = known[word] && known[word].k ? known[word] : null;
        state = field ? 'operator' : 'next';
      } else if (state === 'operator' && OPERATORS.test(word)) {
        state = word === 'in' ? 'brace' : 'value';
      } else if (state === 'brace') {
        state = word === '{' ? 'set' : 'next';
      } else if (state === 'value') {
        state = 'next';
      } else if (state === 'set') {
        if (word === '}') { state = 'next'; }
      } else if (connector) {
        state = 'term';
      } else {
        if (word === ')') { depth--; }
        state = 'next';
      }
    });
    return { state: state, field: field, partial: partial, start: start, depth: depth };
  }

  // Lines that start with the text come first, then those with a part that does, then the rest
  function rank(name, text) {
    var i = name.indexOf(text);
    if (i < 0) { return -1; }
    if (i === 0) { return 0; }
    return name[i - 1] === '.' || name[i - 1] === '_' ? 1 : 2;
  }

  function lines(ctx, cat) {
    var text = ctx.partial.toLowerCase();
    var plain = function (word) { return { name: word, insert: word }; };
    var starting = function (words) {
      return words.filter(function (w) { return w.indexOf(text) === 0; }).map(plain);
    };
    if (ctx.state === 'term') {
      var found = [[], [], []];
      cat.names.concat({ n: 'not', h: cat.not }).forEach(function (n) {
        var r = rank(n.n, text);
        if (r >= 0) { found[r].push({ name: n.n, insert: n.n, hint: n.h }); }
      });
      return found[0].concat(found[1], found[2]).slice(0, LIMIT);
    }
    if (ctx.state === 'operator') {
      return starting(cat.kinds[ctx.field.k].ops).map(function (l) {
        if (l.name === 'in') { l.insert = 'in {'; }
        return l;
      }).concat(starting(CONNECTORS));
    }
    if (ctx.state === 'brace') { return [plain('{')]; }
    if (ctx.state === 'set') { return starting(['}']); }
    if (ctx.state === 'next') {
      return starting(CONNECTORS).concat(ctx.depth > 0 ? starting([')']) : []);
    }
    return [];
  }

  // ---- the list --------------------------------------------------------------------------------
  var list = document.createElement('ul');
  list.className = 'suggest';
  list.id = 'filter-suggestions';
  list.setAttribute('role', 'listbox');
  list.hidden = true;

  var field = null;     // the input the list belongs to
  var shown = [];       // its lines
  var chosen = -1;      // the line chosen with the arrow keys
  var context_ = null;  // where the lines apply
  var ticket = 0;       // answers to older questions are dropped
  var timer = null;

  function close() {
    ticket++;
    clearTimeout(timer);
    list.hidden = true;
    shown = [];
    chosen = -1;
    if (field) {
      field.setAttribute('aria-expanded', 'false');
      field.removeAttribute('aria-activedescendant');
    }
  }

  function mark() {
    Array.prototype.forEach.call(list.querySelectorAll('[role=option]'), function (el) {
      var on = +el.getAttribute('data-line') === chosen;
      el.setAttribute('aria-selected', String(on));
      if (on) {
        el.scrollIntoView({ block: 'nearest' });
        field.setAttribute('aria-activedescendant', el.id);
      }
    });
    if (chosen < 0) { field.removeAttribute('aria-activedescendant'); }
  }

  function span(parent, cls, text) {
    var el = document.createElement('span');
    el.className = cls;
    el.textContent = text;
    parent.appendChild(el);
  }

  // what comes from the capture is only ever set as text
  function draw(note) {
    list.replaceChildren();
    if (note) {
      var li = document.createElement('li');
      li.className = 'suggest-note';
      li.textContent = note;
      list.appendChild(li);
    }
    shown.forEach(function (line, i) {
      var li = document.createElement('li');
      li.id = 'filter-suggestion-' + i;
      li.setAttribute('role', 'option');
      li.setAttribute('aria-selected', 'false');
      li.setAttribute('data-line', i);
      span(li, 'suggest-name', line.name);
      if (line.hint) { span(li, 'suggest-hint', line.hint); }
      list.appendChild(li);
    });
    var form = field.form;
    if (list.parentNode !== form) { form.appendChild(list); }
    list.hidden = !note && !shown.length;
    field.setAttribute('aria-expanded', String(!list.hidden));
    if (list.hidden) { return; }
    list.style.top = field.offsetTop + field.offsetHeight + 4 + 'px';
    list.style.minWidth = field.offsetWidth + 'px';
    list.style.left = Math.max(0, Math.min(field.offsetLeft, form.clientWidth - list.offsetWidth)) + 'px';
    list.scrollTop = 0;
  }

  function values(ctx, note, mine) {
    var q = ctx.partial.replace(/^"/, '');
    clearTimeout(timer);
    timer = setTimeout(function () {
      fetch('/api/filter/values?field=' + encodeURIComponent(ctx.field.n) + '&q=' + encodeURIComponent(q),
        { credentials: 'same-origin' })
        .then(function (r) { if (!r.ok) { throw new Error(r.status); } return r.json(); })
        .then(function (found) {
          if (mine !== ticket) { return; }
          var more = shown;
          shown = found.map(function (v) {
            return { name: v.text, insert: v.insert, hint: String(v.count) };
          }).concat(more);
          chosen = -1;
          draw(note);
        })
        .catch(function () { /* the note stays */ });
    }, VALUES_DELAY);
  }

  function update() {
    var input = field;
    var mine = ++ticket;
    clearTimeout(timer);
    var kind = input.getAttribute('data-filter-kind') || '';
    var before = input.value.slice(0, input.selectionStart);
    catalog(kind).then(function (cat) {
      if (mine !== ticket) { return; }
      var ctx = context(before, cat.byName);
      context_ = ctx;
      shown = lines(ctx, cat);
      chosen = -1;
      // a name typed to its end needs no line of its own
      if (shown.length === 1 && shown[0].insert === ctx.partial) { shown = []; }
      var wants = ctx.state === 'value' || ctx.state === 'set';
      var note = wants ? cat.kinds[ctx.field.k].hint : '';
      draw(note);
      if (wants && kind === 'connections') { values(ctx, note, mine); }
    }).catch(function () { /* without the list the field works as before */ });
  }

  function take(line) {
    var input = field;
    var text = input.value;
    var caret = input.selectionStart;
    var head = text.slice(0, context_.start) + line.insert + ' ';
    input.value = head + text.slice(caret).replace(/^\s+/, '');
    input.setSelectionRange(head.length, head.length);
    // for the check of the filter, and the list goes on with what can come next
    input.dispatchEvent(new Event('input', { bubbles: true }));
  }

  list.addEventListener('mousedown', function (e) {
    // the field keeps the focus
    e.preventDefault();
    var el = e.target.closest ? e.target.closest('[role=option]') : null;
    if (el) { take(shown[+el.getAttribute('data-line')]); }
  });

  function onKey(e) {
    if (e.key === 'Escape' && !list.hidden) {
      e.preventDefault();
      close();
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      if (list.hidden) { update(); return; }
      if (!shown.length) { return; }
      e.preventDefault();
      var step = e.key === 'ArrowDown' ? 1 : -1;
      chosen = chosen < 0 ? (step > 0 ? 0 : shown.length - 1) : (chosen + step + shown.length) % shown.length;
      mark();
    } else if (e.key === 'Enter' && chosen >= 0) {
      e.preventDefault();
      take(shown[chosen]);
    } else if (e.key === 'Tab' && !e.shiftKey && !list.hidden && shown.length && (chosen >= 0 || context_.partial)) {
      e.preventDefault();
      take(shown[Math.max(chosen, 0)]);
    }
  }

  Array.prototype.forEach.call(inputs, function (input) {
    input.setAttribute('role', 'combobox');
    input.setAttribute('aria-autocomplete', 'list');
    input.setAttribute('aria-expanded', 'false');
    input.setAttribute('aria-controls', list.id);
    var open = function () { field = input; update(); };
    input.addEventListener('input', open);
    input.addEventListener('click', open);
    input.addEventListener('focus', function () { if (!input.value) { open(); } });
    input.addEventListener('blur', function () { if (field === input) { close(); } });
    input.addEventListener('keydown', function (e) { field = input; onKey(e); });
    // the cursor moved: other things can come next
    input.addEventListener('keyup', function (e) {
      if (e.key === 'ArrowLeft' || e.key === 'ArrowRight' || e.key === 'Home' || e.key === 'End') { open(); }
    });
  });
})();
