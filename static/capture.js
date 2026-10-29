// Capture page: shows the fields of the chosen source and the activity of every network adapter
// (packets per second of the last 30 seconds, from /api/previews), so the busy one is easy to find.
(function () {
  'use strict';
  var form = document.querySelector('[data-capture-form]');
  if (!form) { return; }
  var F = window.openpacketmonFormat;

  function showSource() {
    var checked = form.querySelector('input[name="source"]:checked');
    var source = checked ? checked.value : 'device';
    var parts = form.querySelectorAll('[data-source]');
    for (var i = 0; i < parts.length; i++) {
      parts[i].hidden = parts[i].getAttribute('data-source').split(' ').indexOf(source) < 0;
    }
  }
  var radios = form.querySelectorAll('input[name="source"]');
  for (var r = 0; r < radios.length; r++) { radios[r].addEventListener('change', showSource); }
  showSource();

  // The number of stored packets only matters while packets are stored.
  function showPacketsMax() {
    var checked = form.querySelector('input[name="packets"]:checked');
    var max = form.querySelector('[data-packets-max]');
    if (max) { max.hidden = checked && checked.value === 'off'; }
  }
  var stores = form.querySelectorAll('input[name="packets"]');
  for (var s = 0; s < stores.length; s++) { stores[s].addEventListener('change', showPacketsMax); }
  showPacketsMax();

  // Clicking a row selects its adapter.
  var rows = form.querySelectorAll('table.adapters tbody tr');
  for (var k = 0; k < rows.length; k++) {
    rows[k].addEventListener('click', function (e) {
      var radio = this.querySelector('input[type="radio"]');
      if (radio && e.target !== radio) { radio.checked = true; }
    });
  }

  function path(values) {
    var max = Math.max.apply(null, values.concat([1]));
    var n = 30, d = '';
    var offset = n - values.length;
    for (var i = 0; i < values.length; i++) {
      var x = (offset + i) * 120 / (n - 1);
      var y = 23 - values[i] / max * 21;
      d += (i ? 'L' : 'M') + x.toFixed(1) + ' ' + y.toFixed(1);
    }
    return d;
  }

  function update(previews) {
    for (var i = 0; i < previews.length; i++) {
      var p = previews[i];
      var svg = form.querySelector('svg[data-adapter="' + CSS.escape(p.name) + '"] path');
      if (svg) { svg.setAttribute('d', path(p.packets)); }
      var total = form.querySelector('[data-adapter-packets="' + CSS.escape(p.name) + '"]');
      if (total) { total.textContent = F.count(p.total); }
    }
  }

  function poll() {
    if (document.hidden) { return; }
    fetch('/api/previews', { credentials: 'same-origin', headers: { Accept: 'application/json' } })
      .then(function (r) { if (!r.ok) { throw new Error(r.status); } return r.json(); })
      .then(update)
      .catch(function () { /* try again */ });
  }
  poll();
  setInterval(poll, 1000);
})();
