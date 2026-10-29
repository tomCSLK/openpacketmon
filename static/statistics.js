// Statistics page: draws the traffic of the shown packets (#stats-data, packets and bytes per second
// in steps) with uPlot, in bits or packets per second. When live.js reloads the data, the chart takes
// the new values and keeps a zoom.
(function () {
  'use strict';
  var T = window.openpacketmonText;
  var F = window.openpacketmonFormat;
  var plot = null, plotStep = 0, unit = 'bits';

  function css(name) { return getComputedStyle(document.documentElement).getPropertyValue(name).trim(); }

  function data() {
    var el = document.getElementById('stats-data');
    try { return el ? JSON.parse(el.textContent) : null; } catch (e) { return null; }
  }

  function draw() {
    var chartEl = document.getElementById('stats-chart');
    if (plot) { plot.destroy(); plot = null; }
    if (!chartEl || !window.uPlot) { return; }
    chartEl.textContent = '';
    var d = data();
    if (!d || !d.packets || !d.packets.length) {
      var empty = document.createElement('p');
      empty.className = 'muted';
      empty.textContent = chartEl.getAttribute('data-empty') || '';
      chartEl.appendChild(empty);
      return;
    }
    plotStep = d.step;
    var fg = css('--muted'), grid = css('--border'), color = css('--accent');
    var fmt = function (u, v) { return F.rate(unit, v); };
    var withSeconds = d.step < 60;
    // ticks on whole steps of the data, at least a second apart
    var incrs = [1, 2, 5, 10, 15, 30, 60, 120, 300, 600, 900, 1800, 3600, 7200, 21600, 43200, 86400]
      .filter(function (i) { return i >= d.step; });
    plot = new uPlot({
      width: Math.max(chartEl.clientWidth, 200),
      height: 240,
      cursor: { drag: { x: true, y: false } },
      scales: { y: { range: function (u, min, max) { return [0, max > 0 ? max * 1.1 : 1]; } } },
      series: [
        { label: T('chart-time'), value: function (u, ts) { return ts === null ? '--' : F.day(ts) + ' ' + F.clock(ts, true); } },
        { label: T('stats-traffic'), stroke: color, fill: color + '22', width: 2, value: fmt, paths: uPlot.paths && uPlot.paths.stepped ? uPlot.paths.stepped({ align: 1 }) : undefined }
      ],
      axes: [
        { stroke: fg, grid: { stroke: grid }, ticks: { stroke: grid }, incrs: incrs, space: 70,
          values: function (u, splits) { return splits.map(function (ts) { return F.clock(ts, withSeconds); }); } },
        { stroke: fg, grid: { stroke: grid }, ticks: { stroke: grid }, size: 90,
          values: function (u, vals) { return vals.map(function (v) { return F.rate(unit, v); }); } }
      ]
    }, series(d), chartEl);
  }

  // times and values of the chart
  function series(d) {
    var xs = [], ys = [];
    for (var i = 0; i < d.packets.length; i++) {
      xs.push(d.start + i * d.step);
      ys.push(unit === 'packets' ? d.packets[i] : d.bytes[i] * 8);
    }
    // the last step ends one step later, so even a single step has a width
    xs.push(d.start + d.packets.length * d.step);
    ys.push(ys[ys.length - 1]);
    return [xs, ys];
  }

  // new data from live.js: the chart stays (no jump of the page) and keeps a zoom
  function update() {
    var d = data();
    if (!plot || !d || !d.packets || !d.packets.length || d.step !== plotStep) { draw(); return; }
    var x = plot.scales.x, old = plot.data[0];
    var zoomed = old.length && (x.min > old[0] || x.max < old[old.length - 1]);
    var keep = zoomed ? { min: x.min, max: x.max } : null;
    plot.setData(series(d));
    if (keep) { plot.setScale('x', keep); }
  }

  // the buttons are replaced with the page part, so the clicks are taken at the document
  function syncButtons() {
    var buttons = document.querySelectorAll('[data-stats-unit] button[data-unit]');
    for (var i = 0; i < buttons.length; i++) {
      buttons[i].setAttribute('aria-pressed', String(buttons[i].getAttribute('data-unit') === unit));
    }
  }
  document.addEventListener('click', function (e) {
    var button = e.target.closest ? e.target.closest('[data-stats-unit] button[data-unit]') : null;
    if (!button) { return; }
    unit = button.getAttribute('data-unit');
    syncButtons();
    draw();
  });

  function init() { syncButtons(); draw(); }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
  window.addEventListener('openpacketmon:fragments', update);
  window.addEventListener('openpacketmon:theme', draw);
  window.addEventListener('resize', function () {
    var chartEl = document.getElementById('stats-chart');
    if (plot && chartEl) { plot.setSize({ width: Math.max(chartEl.clientWidth, 200), height: 240 }); }
  });
})();
