// Shows the timestamps the server renders as <time class="ts" datetime="...Z"> in the browser's time zone,
// 24-hour clock, date order of the page's language. The UTC text from the server stays as tooltip (and as
// the fallback without JS). Runs again when live.js has reloaded parts of the page.
(function () {
  'use strict';
  var T = window.openpacketmonText;
  function two(n) { return (n < 10 ? '0' : '') + n; }
  function local(d) {
    return T('date').replace('DD', two(d.getDate())).replace('MM', two(d.getMonth() + 1)).replace('YYYY', d.getFullYear()) +
      ' ' + two(d.getHours()) + ':' + two(d.getMinutes()) + ':' + two(d.getSeconds());
  }
  function sameDay(a, b) {
    return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
  }
  // In tables a time of today shows only the clock, the full date stays in the tooltip.
  function convert() {
    var now = new Date();
    var times = document.querySelectorAll('time.ts:not([data-local])');
    for (var i = 0; i < times.length; i++) {
      var el = times[i];
      var d = new Date(el.getAttribute('datetime'));
      if (isNaN(d.getTime())) { continue; }
      el.title = local(d) + ' (' + el.textContent + ')';
      el.textContent = el.closest('td') && sameDay(d, now) ? local(d).slice(-8) : local(d);
      el.setAttribute('data-local', '');
    }
  }
  convert();
  window.addEventListener('openpacketmon:fragments', convert);
})();
