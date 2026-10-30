// Packet list and packet details: a click on a row opens the packet; on the details page a field
// marks its bytes in the hex dump, and a byte selects the smallest field that contains it.
(function () {
  'use strict';

  // ---- list: whole rows are links ---------------------------------------------------------------
  document.addEventListener('click', function (e) {
    var row = e.target.closest ? e.target.closest('tr[data-href]') : null;
    if (!row) { return; }
    var link = e.target.closest('a, button, input, select');
    if (!link && window.getSelection && String(window.getSelection())) { return; }
    // the way back leads to this packet, not to the end of the list
    window.history.replaceState(window.history.state, '', new URL(row.getAttribute('data-href'), window.location.href).searchParams.get('back'));
    if (!link) { window.location.href = row.getAttribute('data-href'); }
  });

  // ---- list: colors, filter check and the live list ---------------------------------------------
  function store(key, value) {
    try {
      if (value === undefined) { return localStorage.getItem(key); }
      localStorage.setItem(key, value);
    } catch (e) { /* storage blocked: the default applies */ }
    return null;
  }

  var table = document.querySelector('[data-packets-table]');
  var colors = document.querySelector('[data-colors]');
  if (table && colors) {
    colors.checked = store('openpacketmon.colors') !== 'off';
    table.classList.toggle('colored', colors.checked);
    colors.addEventListener('change', function () {
      table.classList.toggle('colored', colors.checked);
      store('openpacketmon.colors', colors.checked ? 'on' : 'off');
    });
  }

  // "go to packet": Ctrl+G puts the cursor there, the packet found is scrolled into view
  var gotoInput = document.querySelector('[data-goto-input]');
  if (gotoInput) {
    document.addEventListener('keydown', function (e) {
      if ((e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && (e.key === 'g' || e.key === 'G')) {
        e.preventDefault();
        gotoInput.focus();
        gotoInput.select();
      }
    });
  }

  // ---- list: one long list like a packet analyzer --------------------------------------------
  // Only the rows in view are in the page: the scroller holds a spacer as high as all rows, the
  // table stays in view (sticky) and shows the rows at the scroll position, loaded in blocks from
  // /api/packets. At the end of the list it follows the new packets of a running capture.
  var list = document.querySelector('[data-packets]');
  if (list) {
    var T = window.openpacketmonText;
    var scroller = list.querySelector('[data-packets-scroller]');
    var view = scroller.querySelector('.vlist-view');
    var spacer = list.querySelector('[data-packets-spacer]');
    var tbody = list.querySelector('[data-packets-rows]');
    var head = list.querySelector('[data-packets-head]');
    var foot = list.querySelector('[data-packets-foot]');
    var empty = list.querySelector('[data-packets-empty]');
    var toggle = list.querySelector('[data-live-toggle]');
    var note = document.querySelector('[data-goto-note]');
    var filter = list.getAttribute('data-filter') || '';
    var connections = list.getAttribute('data-connections') || '';
    var running = list.hasAttribute('data-running');
    var LIVE_MS = 300;
    // browsers limit the height of an element; beyond it the scroll position is scaled
    var MAX_PX = 8000000;
    var rowH = 26;
    var total = 0;
    var first = 0;          // position of the first row shown
    var follow = !list.hasAttribute('data-goto');
    var paused = false;
    var target = null;      // number of the packet marked by "go to packet"
    var cache = {};         // position -> row data, of the last block
    var busy = false;
    var again = false;

    var fmt = window.openpacketmonFormat;
    var count = function (n) { return fmt && fmt.count ? fmt.count(n) : String(n); };

    var visible = function () {
      var thead = view.querySelector('thead');
      var h = scroller.clientHeight - (thead ? thead.offsetHeight : 0);
      return Math.max(1, Math.floor(h / rowH));
    };
    var maxFirst = function () { return Math.max(0, total - visible()); };
    // scroll range in pixels and the factor from pixels to rows
    var range = function () { return Math.min(maxFirst() * rowH, MAX_PX); };
    var firstAt = function (top) {
      var r = range();
      return r <= 0 ? 0 : Math.min(maxFirst(), Math.round(top / r * maxFirst()));
    };
    var topOf = function (row) {
      var m = maxFirst();
      return m <= 0 ? 0 : Math.round(row / m * range());
    };

    var layout = function () {
      spacer.style.height = range() + 'px';
    };

    var scrollTo = function (row) {
      first = Math.max(0, Math.min(maxFirst(), row));
      scroller.scrollTop = topOf(first);
    };

    var backFor = function (n) {
      return encodeURIComponent('/packets?' + (connections ? 'c=' + encodeURIComponent(connections) + '&' : '')
        + (filter ? 'f=' + encodeURIComponent(filter) + '&' : '') + 'goto=' + n);
    };

    var cell = function (tr, text, cls, title) {
      var td = document.createElement('td');
      if (cls) { td.className = cls; }
      if (title) { td.title = title; }
      td.textContent = text;
      tr.appendChild(td);
      return td;
    };

    var draw = function () {
      var n = visible();
      var frag = document.createDocumentFragment();
      var shown = 0;
      for (var i = first; i < first + n && i < total; i++) {
        var r = cache[i];
        var tr = document.createElement('tr');
        if (!r) { tr.className = 'loading'; cell(tr, '', 'right num'); for (var k = 0; k < 7; k++) { cell(tr, ''); } frag.appendChild(tr); continue; }
        var href = '/packet?n=' + r.n + '&back=' + backFor(r.n);
        tr.setAttribute('data-href', href);
        var cls = (r.color ? 'c-' + r.color : '') + (r.n === target ? ' target' : '');
        if (cls.trim()) { tr.className = cls.trim(); }
        var a = document.createElement('a');
        a.href = href;
        a.textContent = r.n;
        cell(tr, '', 'right num').appendChild(a);
        cell(tr, r.time, 'right num nowrap');
        var src = cell(tr, '', 'clip', r.src);
        src.appendChild(document.createElement('code')).textContent = r.src;
        cell(tr, '→', r.out === true ? 'dir out' : 'dir',
          r.out === true ? T('direction-out') : (r.out === false ? T('direction-in') : ''));
        var dst = cell(tr, '', 'clip', r.dst);
        dst.appendChild(document.createElement('code')).textContent = r.dst;
        cell(tr, r.proto, 'nowrap');
        cell(tr, r.len, 'right num');
        cell(tr, r.info, 'clip', r.info);
        frag.appendChild(tr);
        shown++;
      }
      tbody.replaceChildren(frag);
      if (empty) { empty.hidden = total > 0; }
      if (head) {
        head.textContent = total === 0 ? T('packets-none')
          : T('packets-range', count(first + 1), count(Math.min(total, first + n)), count(total));
      }
      if (toggle) { toggle.classList.toggle('paused', paused || !follow); toggle.setAttribute('aria-pressed', String(!paused && follow)); }
      return shown;
    };

    // loads the rows in view (and some around them); `opts.goto` finds a packet first
    var load = function (opts) {
      opts = opts || {};
      if (busy) { again = true; return; }
      busy = true;
      var n = visible();
      var params = [];
      if (filter) { params.push('f=' + encodeURIComponent(filter)); }
      if (connections) { params.push('c=' + encodeURIComponent(connections)); }
      if (opts.goto) {
        params.push('goto=' + opts.goto);
        params.push('count=' + n);
      } else if (follow) {
        params.push('count=' + n);
      } else {
        var from = Math.max(0, first - n);
        params.push('from=' + from, 'count=' + (n * 3));
        var top = cache[first];
        if (top) { params.push('anchor=' + top.n); }
      }
      fetch('/api/packets?' + params.join('&'), { credentials: 'same-origin', headers: { Accept: 'application/json' } })
        .then(function (r) { if (!r.ok) { throw new Error(r.status); } return r.json(); })
        .then(function (d) {
          var anchorWas = cache[first] ? first : null;
          total = d.total;
          cache = {};
          for (var i = 0; i < d.rows.length; i++) { cache[d.from + i] = d.rows[i]; }
          layout();
          if (opts.goto) {
            if (d.goto) {
              target = d.goto[1];
              follow = false;
              scrollTo(d.goto[0] - Math.floor(n / 2));
              if (note) {
                note.hidden = d.goto[1] === opts.goto;
                note.textContent = T('packets-goto-other', opts.goto, d.goto[1]);
              }
            } else if (note) {
              note.hidden = false;
              note.textContent = T('packets-goto-none', opts.goto);
            }
          } else if (follow) {
            scrollTo(maxFirst());
          } else if (d.anchor !== null && d.anchor !== undefined && anchorWas !== null && d.anchor !== anchorWas) {
            // older packets were dropped from the store: keep the same packets in view
            scrollTo(first + d.anchor - anchorWas);
          }
          if (foot && d.foot) { foot.textContent = d.foot; }
          if (!d.running && running) {
            running = false;
            if (toggle) { toggle.hidden = true; }
          }
          // the block does not cover the rows in view (scrolled meanwhile): load again
          if (draw() < Math.min(visible(), total - first)) { again = true; }
        })
        .catch(function () { /* server stopped or signed out: try again */ })
        .then(function () {
          busy = false;
          if (again) { again = false; load(); }
        });
    };

    scroller.addEventListener('scroll', function () {
      // the position set by the list itself (following, go to packet)
      if (Math.abs(scroller.scrollTop - topOf(first)) < 1) { return; }
      var row = firstAt(scroller.scrollTop);
      // at the end the list follows the new packets again; scrolled up it stays put
      follow = !paused && running && row >= maxFirst();
      if (row === first) { return; }
      first = row;
      if (draw() < Math.min(visible(), total - first)) { load(); }
    });

    if (toggle) {
      toggle.addEventListener('click', function () {
        if (paused || !follow) {
          paused = false;
          follow = true;
          load();
        } else {
          paused = true;
          follow = false;
          draw();
        }
      });
    }

    var goto = function (n) {
      if (!(n > 0)) { return; }
      load({ goto: n });
    };
    var gotoForm = list.querySelector('[data-goto-form]');
    if (gotoForm) {
      gotoForm.addEventListener('submit', function (e) {
        e.preventDefault();
        goto(+gotoForm.querySelector('[data-goto-input]').value);
        scroller.focus();
      });
    }

    // keys of a packet analyzer: Ctrl+Home/End first and last packet, Page Up/Down and arrows scroll
    document.addEventListener('keydown', function (e) {
      if (e.target.closest && e.target.closest('input, select, textarea')) { return; }
      var n = visible();
      var step = null;
      if ((e.ctrlKey || e.metaKey) && e.key === 'Home') { step = -total; }
      else if ((e.ctrlKey || e.metaKey) && e.key === 'End') { step = total; }
      else if (e.target === scroller && e.key === 'PageDown') { step = n - 1; }
      else if (e.target === scroller && e.key === 'PageUp') { step = -(n - 1); }
      else if (e.target === scroller && e.key === 'ArrowDown') { step = 1; }
      else if (e.target === scroller && e.key === 'ArrowUp') { step = -1; }
      if (step === null) { return; }
      e.preventDefault();
      scrollTo(first + step);
      follow = !paused && running && first >= maxFirst();
      if (draw() < Math.min(visible(), total - first)) { load(); }
    });

    window.addEventListener('resize', function () { layout(); scrollTo(first); load(); });

    // the row height follows the font; measured once with a row in the table
    var probe = document.createElement('tr');
    probe.innerHTML = '<td>0</td>';
    tbody.appendChild(probe);
    rowH = probe.offsetHeight || rowH;
    tbody.removeChild(probe);

    var start = +list.getAttribute('data-goto');
    if (start > 0) { goto(start); } else { load(); }
    setInterval(function () {
      if (running && !paused && !document.hidden) { load(); }
    }, LIVE_MS);
  }

  // ---- details: fields and bytes ----------------------------------------------------------------
  var hex = document.querySelector('.hex');
  if (!hex) { return; }
  var bytes = [];
  Array.prototype.forEach.call(hex.querySelectorAll('[data-i]'), function (el) {
    var i = +el.getAttribute('data-i');
    (bytes[i] = bytes[i] || []).push(el);
  });
  var fields = Array.prototype.slice.call(document.querySelectorAll('.layers [data-range]'));

  function range(el) {
    var r = el.getAttribute('data-range').split('-');
    return [+r[0], +r[1]];
  }

  var marked = [];
  var selected = null;

  function mark(el) {
    marked.forEach(function (b) { b.classList.remove('hl'); });
    marked = [];
    if (!el) { return; }
    var r = range(el);
    for (var i = r[0]; i < r[1]; i++) {
      (bytes[i] || []).forEach(function (b) { b.classList.add('hl'); marked.push(b); });
    }
  }

  function select(el) {
    if (selected) { selected.classList.remove('selected'); }
    selected = el;
    if (el) { el.classList.add('selected'); }
    mark(el);
  }

  var layers = document.querySelector('.layers');
  layers.addEventListener('mouseover', function (e) {
    var el = e.target.closest('[data-range]');
    if (el) { mark(el); }
  });
  layers.addEventListener('mouseleave', function () { mark(selected); });
  layers.addEventListener('click', function (e) {
    var el = e.target.closest('.field');
    if (el) { select(el === selected ? null : el); }
  });

  hex.addEventListener('click', function (e) {
    var b = e.target.closest('[data-i]');
    if (!b) { return; }
    var i = +b.getAttribute('data-i');
    var best = null;
    var bestLen = Infinity;
    fields.forEach(function (el) {
      var r = range(el);
      if (r[0] <= i && i < r[1] && r[1] - r[0] < bestLen) { best = el; bestLen = r[1] - r[0]; }
    });
    if (!best) { return; }
    var layer = best.closest('details');
    if (layer) { layer.open = true; }
    select(best.tagName === 'SUMMARY' ? null : best);
    if (best.tagName === 'SUMMARY') { mark(best); }
    best.scrollIntoView({ block: 'nearest' });
  });
})();
