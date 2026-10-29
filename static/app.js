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

  // A display filter turns red or green while it is typed, the message says what is wrong
  // (packets and statistics; `data-filter-kind="connections"` for the connection filter).
  var input = document.querySelector('[data-filter-input]');
  var message = document.querySelector('[data-filter-error]');
  if (input && message) {
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
  }


  // Selections that apply at once (rows per page).
  var autos = document.querySelectorAll('select[data-autosubmit]');
  for (var a = 0; a < autos.length; a++) {
    autos[a].addEventListener('change', function (e) { e.target.form.submit(); });
  }
})();
