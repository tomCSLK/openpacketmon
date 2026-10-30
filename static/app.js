// Shell behaviour: theme toggle (system -> light -> dark), active navigation entry, filter check, selections.
(function () {
  'use strict';
  var root = document.documentElement;
  var T = window.openpacketmonText;
  var LABELS = { system: T('theme-system'), light: T('theme-light'), dark: T('theme-dark') };

  function stored() {
    try { return localStorage.getItem('openpacketmon-theme') || 'system'; } catch (e) { return 'system'; }
  }
  function apply(mode) {
    if (mode === 'light' || mode === 'dark') { root.setAttribute('data-theme', mode); } else { root.removeAttribute('data-theme'); }
    try {
      if (mode === 'system') { localStorage.removeItem('openpacketmon-theme'); } else { localStorage.setItem('openpacketmon-theme', mode); }
    } catch (e) { /* not remembered, still applied */ }
    var buttons = document.querySelectorAll('[data-theme-toggle]');
    for (var i = 0; i < buttons.length; i++) {
      buttons[i].setAttribute('data-mode', mode);
      var label = buttons[i].querySelector('.label');
      if (label) { label.textContent = LABELS[mode]; }
      buttons[i].setAttribute('aria-label', T('theme-toggle', LABELS[mode]));
    }
    window.dispatchEvent(new Event('openpacketmon:theme'));
  }

  var toggles = document.querySelectorAll('[data-theme-toggle]');
  for (var i = 0; i < toggles.length; i++) {
    toggles[i].addEventListener('click', function () {
      var next = { system: 'light', light: 'dark', dark: 'system' }[stored()] || 'system';
      apply(next);
    });
  }
  apply(stored());
  if (window.matchMedia) {
    var mq = window.matchMedia('(prefers-color-scheme: dark)');
    var onChange = function () { if (stored() === 'system') { window.dispatchEvent(new Event('openpacketmon:theme')); } };
    if (mq.addEventListener) { mq.addEventListener('change', onChange); }
  }

  // Highlight the navigation entry of the current section (longest matching prefix).
  var links = document.querySelectorAll('.nav a[data-match]');
  var path = window.location.pathname, best = null, bestLen = -1;
  for (var j = 0; j < links.length; j++) {
    var prefixes = links[j].getAttribute('data-match').split(' ');
    for (var k = 0; k < prefixes.length; k++) {
      var p = prefixes[k];
      var hit = p === '/' ? path === '/' : path === p || path.indexOf(p + '/') === 0 || path.indexOf(p) === 0;
      if (hit && p.length > bestLen) { best = links[j]; bestLen = p.length; }
    }
  }
  if (best) { best.setAttribute('aria-current', 'page'); }

  // "Back" on error pages: the browser's history keeps what was typed into the form; the link
  // (previous page or overview) is the fallback without history.
  var back = document.querySelector('[data-back]');
  if (back && window.history.length > 1) {
    back.addEventListener('click', function (e) {
      e.preventDefault();
      window.history.back();
    });
  }

  // "Back" of the page heads: a step through the pages of this tab, also where no browser bar offers
  // it (the window of openpacketmon-app). Every history entry notes how many entries lead to it and
  // how many steps go to another page: a form that returns to its page (favorite, unit, pause) adds
  // an entry with the same address, and going there would look like nothing happened.
  var NAV_KEY = 'openpacketmon-nav';
  var here = window.location.pathname + window.location.search;
  function navRemember(replacing) {
    try {
      sessionStorage.setItem(NAV_KEY, JSON.stringify({ depth: nav.depth, skip: nav.skip, url: here, replacing: replacing }));
    } catch (e) { /* every page then counts as the first one: the button uses its address */ }
  }
  function navLast() {
    try { return JSON.parse(sessionStorage.getItem(NAV_KEY)); } catch (e) { return null; }
  }
  var nav = window.history.state && window.history.state.depth ? window.history.state : null;
  if (!nav) {
    var last = navLast();
    if (!last) {
      nav = { depth: 1, skip: 1 };
    } else if (last.replacing) {
      nav = { depth: last.depth, skip: last.skip };
    } else {
      nav = { depth: last.depth + 1, skip: last.url === here ? last.skip + 1 : 1 };
    }
    window.history.replaceState(nav, '');
  }
  // also after the browser restored the page without loading it
  window.addEventListener('pageshow', function () { navRemember(false); });

  var navBack = document.querySelector('[data-nav-back]');
  if (navBack) {
    var earlier = nav.depth > nav.skip && window.history.length > 1;
    var nowhere = !earlier && !navBack.getAttribute('href');
    if (nowhere) { navBack.setAttribute('aria-disabled', 'true'); }
    navBack.addEventListener('click', function (e) {
      if (earlier || nowhere) { e.preventDefault(); }
      if (earlier) { window.history.go(-nav.skip); }
    });
    // Alt+Left as in a browser, with the same steps
    document.addEventListener('keydown', function (e) {
      if (e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey && e.key === 'ArrowLeft') {
        e.preventDefault();
        navBack.click();
      }
    });
  }

  // Links that change the view of a page (next packet, text or hex) leave no step of their own.
  document.addEventListener('click', function (e) {
    var link = e.target.closest ? e.target.closest('a[data-replace]') : null;
    if (!link || e.button !== 0 || e.ctrlKey || e.metaKey || e.shiftKey) { return; }
    e.preventDefault();
    navRemember(true);
    window.location.replace(link.href);
  });

  // A display filter turns red or green while it is typed, the message says what is wrong
  // (packets and statistics; `data-filter-kind="connections"` for the connection filter).
  var inputs = document.querySelectorAll('[data-filter-input]');
  var message = document.querySelector('[data-filter-error]');
  Array.prototype.forEach.call(message ? inputs : [], function (input) {
    var timer = null;
    input.addEventListener('input', function () {
      clearTimeout(timer);
      timer = setTimeout(function () {
        var text = input.value;
        if (!text.trim()) {
          input.classList.remove('valid', 'invalid');
          message.hidden = true;
          return;
        }
        var kind = input.getAttribute('data-filter-kind');
        fetch('/api/filter?' + (kind ? 'kind=' + kind + '&' : '') + 'f=' + encodeURIComponent(text),
          { credentials: 'same-origin' })
          .then(function (r) { return r.json(); })
          .then(function (error) {
            if (input.value !== text) { return; }
            input.classList.toggle('invalid', !!error);
            input.classList.toggle('valid', !error);
            message.hidden = !error;
            message.textContent = error || '';
          })
          .catch(function () { /* checked again on submit */ });
      }, 200);
    });
  });

  // The statistics of the connections take what is in the filter field, applied or not.
  var carry = document.querySelector('[data-filter-carry]');
  if (carry && inputs.length) {
    carry.addEventListener('click', function () {
      var text = inputs[0].value.trim();
      carry.href = carry.pathname + (text ? '?c=' + encodeURIComponent(text) : '');
    });
  }

  // Selections that apply at once (rows per page).
  var autos = document.querySelectorAll('select[data-autosubmit]');
  for (var a = 0; a < autos.length; a++) {
    autos[a].addEventListener('change', function (e) { e.target.form.submit(); });
  }
})();
