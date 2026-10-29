// Formatting of live values in the page's language, the same way as src/web/format.rs on the server:
// counts with thousands separators, volumes in decimal units, 24-hour clock in the local time zone.
(function () {
  'use strict';
  var T = window.openpacketmonText;

  function num(v, decimals) {
    var s = v.toFixed(decimals);
    if (s.indexOf('.') >= 0) { s = s.replace(/0+$/, '').replace(/\.$/, ''); }
    return s.replace('.', T('decimal'));
  }

  function count(v) {
    var s = String(Math.round(v)), out = '';
    for (var i = 0; i < s.length; i++) {
      if (i > 0 && (s.length - i) % 3 === 0) { out += T('thousands'); }
      out += s.charAt(i);
    }
    return out;
  }

  function scaled(v, units) {
    var a = Math.abs(v);
    if (a >= 1e12) { return num(v / 1e12, 2) + ' T' + units; }
    if (a >= 1e9) { return num(v / 1e9, 2) + ' G' + units; }
    if (a >= 1e6) { return num(v / 1e6, 2) + ' M' + units; }
    if (a >= 1e3) { return num(v / 1e3, 1) + ' k' + units; }
    return Math.round(v) + ' ' + units;
  }

  function bytes(v) { return scaled(v, 'B'); }
  function bits(v) { return scaled(v, 'bit'); }

  // An amount in the chosen unit: `v` is packets for "packets", bytes for "bytes", bits for "bits".
  function amount(unit, v) {
    if (v === null || v === undefined) { return '–'; }
    if (unit === 'packets') { return count(v); }
    return unit === 'bits' ? bits(v) : bytes(v);
  }

  // A rate per second, e.g. "1,2 MB/s" or "35 packets/s".
  function rate(unit, v) {
    if (v === null || v === undefined) { return '–'; }
    if (unit === 'packets') { return T('packets-per-second', num(v, v < 10 ? 1 : 0)); }
    return amount(unit, v) + '/s';
  }

  function two(n) { return (n < 10 ? '0' : '') + n; }
  function clock(ts, withSeconds) {
    var d = new Date(ts * 1000);
    return two(d.getHours()) + ':' + two(d.getMinutes()) + (withSeconds ? ':' + two(d.getSeconds()) : '');
  }
  function day(ts) {
    var d = new Date(ts * 1000);
    return T('day').replace('DD', two(d.getDate())).replace('MM', two(d.getMonth() + 1)).replace('YYYY', d.getFullYear());
  }

  window.openpacketmonFormat = { num: num, count: count, bytes: bytes, bits: bits, amount: amount, rate: rate, clock: clock, day: day };
})();
