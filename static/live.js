// Live values of the capture: polls /api/live every second and keeps the tiles ([data-live]), the
// notification badge, the state and the traffic chart (#live-chart, uPlot) current. Parts of the page
// marked [data-fragment] are reloaded every two seconds, but not while the pointer is over them or a
// field in them has the focus. New notifications play a short sound made with the Web Audio API.
(function () {
  'use strict';
  var T = window.openpacketmonText;
  var F = window.openpacketmonFormat;
  var POLL_MS = 1000, FRAGMENT_MS = 2000;

  // ---- sounds ------------------------------------------------------------------------------
  var audio = null;
  function context() {
    if (!audio) {
      var Ctx = window.AudioContext || window.webkitAudioContext;
      if (!Ctx) { return null; }
      audio = new Ctx();
    }
    if (audio.state === 'suspended') { audio.resume(); }
    return audio;
  }
  // Browsers only allow sound after the first click or key press on the page.
  document.addEventListener('pointerdown', context, { once: true });
  document.addEventListener('keydown', context, { once: true });

  // Plays one of the notification sounds ("gulp", "pop", "swhoosh") at `volume` (0–100).
  function play(kind, volume) {
    var ctx = context();
    if (!ctx || kind === 'none' || !volume) { return; }
    var shapes = {
      gulp: { from: 520, to: 180, length: 0.18, type: 'sine' },
      pop: { from: 900, to: 700, length: 0.07, type: 'triangle' },
      swhoosh: { from: 250, to: 1400, length: 0.35, type: 'sawtooth' }
    };
    var s = shapes[kind] || shapes.pop;
    var now = ctx.currentTime;
    var osc = ctx.createOscillator(), gain = ctx.createGain();
    osc.type = s.type;
    osc.frequency.setValueAtTime(s.from, now);
    osc.frequency.exponentialRampToValueAtTime(s.to, now + s.length);
    var peak = 0.4 * volume / 100 * (s.type === 'sawtooth' ? 0.35 : 1);
    gain.gain.setValueAtTime(0.0001, now);
    gain.gain.exponentialRampToValueAtTime(peak, now + 0.01);
    gain.gain.exponentialRampToValueAtTime(0.0001, now + s.length);
    osc.connect(gain).connect(ctx.destination);
    osc.start(now);
    osc.stop(now + s.length + 0.02);
  }
  window.openpacketmonSound = play;

  // ---- chart -------------------------------------------------------------------------------
  var chartEl = document.getElementById('live-chart');
  var compact = chartEl && chartEl.hasAttribute('data-compact');
  var plot = null, emptyEl = null;
  var samples = [], start = 0, unit = 'bytes', key = null;

  function css(name) { return getComputedStyle(document.documentElement).getPropertyValue(name).trim(); }

  // Series values of the chosen unit: [timestamps, in, out]
  function columns() {
    var ts = [], inc = [], out = [];
    var factor = unit === 'bits' ? 8 : 1, i0 = unit === 'packets' ? 3 : 1;
    for (var i = 0; i < samples.length; i++) {
      var s = samples[i];
      ts.push(start + s[0]);
      inc.push(s[i0] * factor);
      out.push(s[i0 + 1] * factor);
    }
    return [ts, inc, out];
  }

  function size() {
    var w = Math.max(chartEl.clientWidth, 200);
    return { width: w, height: compact ? Math.max(window.innerHeight - chartEl.getBoundingClientRect().top - 12, 120) : 260 };
  }

  function build() {
    if (plot) { plot.destroy(); plot = null; }
    if (!window.uPlot) { return; }
    var fg = css('--muted'), grid = css('--border');
    var inColor = css('--accent'), outColor = css('--ok');
    var fmt = function (u, v) { return F.rate(unit, v); };
    var opts = {
      width: size().width,
      height: size().height,
      legend: { show: !compact },
      cursor: { drag: { x: !compact, y: false } },
      series: [
        { label: T('chart-time'), value: function (u, ts) { return ts === null ? '--' : F.day(ts) + ' ' + F.clock(ts, true); } },
        { label: T('chart-in'), stroke: inColor, fill: inColor + '22', width: 2, value: fmt },
        { label: T('chart-out'), stroke: outColor, fill: outColor + '22', width: 2, value: fmt }
      ],
      axes: [
        { stroke: fg, grid: { stroke: grid }, ticks: { stroke: grid },
          values: function (u, splits, ai, space, incr) { return splits.map(function (ts) { return F.clock(ts, incr < 60); }); } },
        { stroke: fg, grid: { stroke: grid }, ticks: { stroke: grid }, size: compact ? 70 : 90,
          values: function (u, vals) { return vals.map(function (v) { return F.rate(unit, v); }); } }
      ]
    };
    plot = new uPlot(opts, columns(), chartEl);
  }

  function draw(reset) {
    if (!chartEl) { return; }
    if (!samples.length) {
      if (plot) { plot.destroy(); plot = null; }
      if (!emptyEl) {
        emptyEl = document.createElement('p');
        emptyEl.className = 'muted';
        emptyEl.textContent = chartEl.getAttribute('data-empty') || '';
        chartEl.appendChild(emptyEl);
      }
      return;
    }
    if (emptyEl) { emptyEl.remove(); emptyEl = null; }
    if (!plot || reset) { build(); } else { plot.setData(columns()); }
  }

  window.addEventListener('resize', function () { if (plot) { plot.setSize(size()); } });
  window.addEventListener('openpacketmon:theme', function () { if (plot) { build(); } });

  // ---- values ------------------------------------------------------------------------------
  function setText(sel, text) {
    var els = document.querySelectorAll(sel);
    for (var i = 0; i < els.length; i++) { if (els[i].textContent !== text) { els[i].textContent = text; } }
  }

  var lastSound = null, wasRunning = null;

  function apply(d) {
    var tot = d.totals, factor = d.unit === 'bits' ? 8 : 1;
    var inc = d.unit === 'packets' ? tot.in_packets : tot.in_bytes * factor;
    var out = d.unit === 'packets' ? tot.out_packets : tot.out_bytes * factor;
    setText('[data-live="total"]', F.amount(d.unit, inc + out));
    setText('[data-live="in"]', F.amount(d.unit, inc));
    setText('[data-live="out"]', F.amount(d.unit, out));
    setText('[data-live="connections"]', F.count(tot.connections));
    setText('[data-live="hosts"]', F.count(tot.hosts));
    if (tot.dropped !== null) { setText('[data-live="dropped"]', F.count(tot.dropped)); }

    var badges = document.querySelectorAll('[data-live-unread]');
    for (var b = 0; b < badges.length; b++) {
      // up to two digits, so the badge fits next to the long label; the number is in the tooltip
      badges[b].textContent = d.unread > 99 ? '99+' : d.unread;
      badges[b].title = d.unread;
      badges[b].hidden = d.unread === 0;
    }
    var state = d.error ? 'state-error' : d.finished ? 'state-finished' : d.frozen ? 'state-paused' : 'state-running';
    setText('[data-live-state]', T(state));
    var errors = document.querySelectorAll('[data-live-error]');
    for (var e = 0; e < errors.length; e++) {
      errors[e].hidden = !d.error;
      errors[e].textContent = d.error || '';
    }

    if (lastSound !== null && d.sound[0] !== lastSound) { play(d.sound[1], d.volume); }
    lastSound = d.sound[0];

    // started or stopped elsewhere (another tab, the command line): show the page anew
    if (wasRunning !== null && wasRunning !== d.running) { location.reload(); return; }
    wasRunning = d.running;

    var reset = d.reset || d.unit !== unit;
    if (d.reset) { samples = d.samples; } else { samples = samples.concat(d.samples); }
    if (samples.length > 3600) { samples = samples.slice(samples.length - 3600); }
    start = d.start;
    unit = d.unit;
    key = { capture: d.capture, generation: d.generation };
    draw(reset);
    var last = samples[samples.length - 1];
    if (last) {
      var i0 = unit === 'packets' ? 3 : 1, f = unit === 'bits' ? 8 : 1;
      setText('[data-live-rate]', T('rate-now', F.rate(unit, last[i0] * f), F.rate(unit, last[i0 + 1] * f)));
    }
  }

  var busy = false;
  function poll() {
    if (busy || document.hidden) { return; }
    busy = true;
    var url = '/api/live';
    if (key && samples.length) {
      url += '?capture=' + key.capture + '&generation=' + key.generation + '&since=' + samples[samples.length - 1][0];
    }
    fetch(url, { credentials: 'same-origin', headers: { Accept: 'application/json' } })
      .then(function (r) { if (!r.ok) { throw new Error(r.status); } return r.json(); })
      .then(apply)
      .catch(function () { /* server stopped or signed out: keep the last values */ })
      .then(function () { busy = false; });
  }
  poll();
  setInterval(poll, POLL_MS);

  // ---- fragments ---------------------------------------------------------------------------
  var fragments = document.querySelectorAll('[data-fragment]');
  if (!fragments.length || !window.DOMParser) { return; }
  var hovered = null;
  document.addEventListener('pointerover', function (e) { hovered = e.target.closest ? e.target.closest('[data-fragment]') : null; });
  document.addEventListener('pointerleave', function () { hovered = null; });

  function refreshFragments() {
    if (document.hidden) { return; }
    var active = document.activeElement;
    fetch(location.href, { credentials: 'same-origin' })
      .then(function (r) { if (!r.ok) { throw new Error(r.status); } return r.text(); })
      .then(function (html) {
        var doc = new DOMParser().parseFromString(html, 'text/html');
        // new parts may be shorter for a moment: the page stays where it was scrolled to
        var scrolled = window.scrollY;
        var current = document.querySelectorAll('[data-fragment]');
        for (var i = 0; i < current.length; i++) {
          var el = current[i];
          if (el === hovered || (active && el.contains(active) && active !== document.body)) { continue; }
          var fresh = doc.querySelector('[data-fragment="' + el.getAttribute('data-fragment') + '"]');
          if (fresh) {
            el.replaceWith(document.importNode(fresh, true));
          }
        }
        window.dispatchEvent(new Event('openpacketmon:fragments'));
        if (window.scrollY !== scrolled) { window.scrollTo(window.scrollX, scrolled); }
      })
      .catch(function () { /* try again next time */ });
  }
  setInterval(refreshFragments, FRAGMENT_MS);
})();
