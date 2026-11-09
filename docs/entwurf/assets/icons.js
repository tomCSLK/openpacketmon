/* Entwurf openpacketmon: SVG-Symbole im Stil von Lucide (ISC), für den Entwurf nachgezeichnet.
   Die Symbole der Navigation sind dieselben Zeichnungen wie heute in resources/icons/.
   m-a … m-e sind die Logo-Entwürfe (eigene Formen); m-sec und m-net sind die Zeichen von opensecmon und
   opennetmon (gewählt: C), nur zum Vergleich. */
(function () {
  const P = {
    // Navigation (wie resources/icons/)
    overview: '<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/>',
    connections: '<polyline points="4 7 20 7"/><polyline points="16 3 20 7 16 11"/><polyline points="20 17 4 17"/><polyline points="8 13 4 17 8 21"/>',
    packets: '<path d="M12 3 20.5 7.5v9L12 21l-8.5-4.5v-9z"/><path d="M3.5 7.5 12 12l8.5-4.5"/><path d="M12 12v9"/>',
    statistics: '<path d="M4 20V10"/><path d="M10 20V4"/><path d="M16 20v-7"/><path d="M22 20H2"/>',
    hosts: '<circle cx="12" cy="12" r="9"/><path d="M3 12h18"/><path d="M12 3a14 14 0 0 1 0 18a14 14 0 0 1 0-18"/>',
    services: '<rect x="3" y="4" width="18" height="6" rx="1.5"/><rect x="3" y="14" width="18" height="6" rx="1.5"/><path d="M7 7h.01"/><path d="M7 17h.01"/>',
    programs: '<rect x="3" y="4" width="18" height="16" rx="2"/><polyline points="7 10 10 12.5 7 15"/><path d="M12 15h4"/>',
    bell: '<path d="M6 16V11a6 6 0 0 1 12 0v5l2 2H4z"/><path d="M10 20a2 2 0 0 0 4 0"/>',
    capture: '<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="3.5"/>',
    settings: '<path d="M4 7h16"/><circle cx="9" cy="7" r="2.2"/><path d="M4 17h16"/><circle cx="15" cy="17" r="2.2"/>',
    back: '<polyline points="15 5 8 12 15 19"/>',
    star: '<path d="M12 3.5l2.6 5.3 5.9.9-4.25 4.1 1 5.8L12 16.9l-5.25 2.7 1-5.8L3.5 9.7l5.9-.9z"/>',
    // allgemein
    search: '<circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>',
    filter: '<path d="M3 4h18l-7 8.5V19l-4 2v-8.5z"/>',
    check: '<path d="M20 6 9 17l-5-5"/>',
    'check-circle': '<circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/>',
    'x-circle': '<circle cx="12" cy="12" r="10"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/>',
    'help-circle': '<circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><path d="M12 17h.01"/>',
    alert: '<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/>',
    info: '<circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/>',
    pause: '<rect x="14" y="4" width="4" height="16" rx="1"/><rect x="6" y="4" width="4" height="16" rx="1"/>',
    play: '<polygon points="6 3 20 12 6 21 6 3"/>',
    stop: '<rect x="5" y="5" width="14" height="14" rx="2"/>',
    activity: '<path d="M22 12h-4l-3 9L9 3l-3 9H2"/>',
    'chev-down': '<path d="m6 9 6 6 6-6"/>',
    'chev-left': '<path d="m15 18-6-6 6-6"/>',
    'chev-right': '<path d="m9 18 6-6-6-6"/>',
    'arrow-up': '<path d="m5 12 7-7 7 7"/><path d="M12 19V5"/>',
    'arrow-down': '<path d="M12 5v14"/><path d="m19 12-7 7-7-7"/>',
    'arrow-right': '<path d="M5 12h14"/><path d="m12 5 7 7-7 7"/>',
    'corner': '<polyline points="15 10 20 15 15 20"/><path d="M4 4v7a4 4 0 0 0 4 4h12"/>',
    sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/>',
    moon: '<path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/>',
    x: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
    keyboard: '<rect width="20" height="16" x="2" y="4" rx="2"/><path d="M6 8h.01"/><path d="M10 8h.01"/><path d="M14 8h.01"/><path d="M18 8h.01"/><path d="M8 12h.01"/><path d="M12 12h.01"/><path d="M16 12h.01"/><path d="M7 16h10"/>',
    refresh: '<path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M8 16H3v5"/>',
    'arrow-up-circle': '<circle cx="12" cy="12" r="10"/><path d="m16 12-4-4-4 4"/><path d="M12 16V8"/>',
    download: '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><path d="M12 15V3"/>',
    copy: '<rect width="14" height="14" x="8" y="8" rx="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/>',
    file: '<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/>',
    folder: '<path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>',
    more: '<circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/><circle cx="5" cy="12" r="1"/>',
    trash: '<path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/>',
    external: '<path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>',
    globe: '<circle cx="12" cy="12" r="10"/><path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20"/><path d="M2 12h20"/>',
    adapter: '<rect x="3" y="5" width="18" height="14" rx="2"/><path d="M8 15v-3h1.5v-2h5v2H16v3"/>',
    radio: '<path d="M4.9 19.1a10 10 0 0 1 0-14.2"/><path d="M7.8 16.2a6 6 0 0 1 0-8.4"/><circle cx="12" cy="12" r="2"/><path d="M16.2 7.8a6 6 0 0 1 0 8.4"/><path d="M19.1 4.9a10 10 0 0 1 0 14.2"/>',
    bot: '<rect x="4" y="8" width="16" height="12" rx="2"/><path d="M12 8V4"/><circle cx="12" cy="3.5" r="1"/><path d="M9 13v2"/><path d="M15 13v2"/>',
    database: '<ellipse cx="12" cy="5" rx="8" ry="3"/><path d="M4 5v14c0 1.66 3.58 3 8 3s8-1.34 8-3V5"/><path d="M4 12c0 1.66 3.58 3 8 3s8-1.34 8-3"/>',
    display: '<rect x="2.5" y="4" width="19" height="13" rx="2"/><path d="M8 21h8"/><path d="M12 17v4"/>',
    volume: '<path d="M11 5 6 9H2v6h4l5 4z"/><path d="M15.5 8.5a5 5 0 0 1 0 7"/><path d="M19 5a10 10 0 0 1 0 14"/>',
    layers: '<path d="m12 2 9.5 5L12 12 2.5 7z"/><path d="m2.5 12 9.5 5 9.5-5"/><path d="m2.5 17 9.5 5 9.5-5"/>',
    code: '<polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/>',
    clock: '<circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/>',
    hash: '<path d="M4 9h16"/><path d="M4 15h16"/><path d="M10 3 8 21"/><path d="M16 3l-2 18"/>',
    stream: '<path d="M4 6h10"/><path d="M4 10h16"/><path d="M10 14h10"/><path d="M4 18h12"/>',
    // Logo-Entwürfe (24er Raster, Strichstärke 2)
    'm-a': '<path d="M4 11h16v7.5a2.5 2.5 0 0 1-2.5 2.5h-11A2.5 2.5 0 0 1 4 18.5z"/><path d="M4 11 2 7.5"/><path d="M20 11l2-3.5"/><path d="M9.5 15.5h5"/><rect x="7.5" y="6" width="3.2" height="3.2" rx=".6" fill="currentColor" stroke="none"/><rect x="12.6" y="2.4" width="3.2" height="3.2" rx=".6" fill="currentColor" stroke="none"/>',
    'm-b': '<rect x="2.5" y="4" width="19" height="13" rx="2"/><path d="M8 21h8"/><path d="M12 17v4"/><rect x="5.6" y="9" width="3.2" height="3.2" rx=".6" fill="currentColor" stroke="none"/><rect x="10.4" y="9" width="3.2" height="3.2" rx=".6" fill="currentColor" stroke="none"/><rect x="15.2" y="9" width="3.2" height="3.2" rx=".6" fill="currentColor" stroke="none"/>',
    'm-c': '<circle cx="10.5" cy="10.5" r="7.5"/><path d="m21 21-5.2-5.2"/><rect x="5.8" y="9" width="2.8" height="2.8" rx=".5" fill="currentColor" stroke="none"/><rect x="9.1" y="9" width="2.8" height="2.8" rx=".5" fill="currentColor" stroke="none"/><rect x="12.4" y="9" width="2.8" height="2.8" rx=".5" fill="currentColor" stroke="none"/>',
    'm-d': '<circle cx="12" cy="12" r="9.5"/><path d="M2.5 12h2"/><path d="M8.5 12h1"/><path d="M14.5 12h1"/><path d="M19.5 12h2"/><rect x="4.6" y="10.4" width="3.2" height="3.2" rx=".6" fill="currentColor" stroke="none"/><rect x="10.4" y="10.4" width="3.2" height="3.2" rx=".6" fill="currentColor" stroke="none"/><rect x="16.2" y="10.4" width="3.2" height="3.2" rx=".6" fill="currentColor" stroke="none"/>',
    'm-e': '<polyline points="7 6 2 12 7 18"/><polyline points="17 6 22 12 17 18"/><rect x="8.4" y="10.4" width="3.2" height="3.2" rx=".6" fill="currentColor" stroke="none"/><rect x="12.4" y="10.4" width="3.2" height="3.2" rx=".6" fill="currentColor" stroke="none"/>',
    // zum Vergleich: opensecmon (Schild mit Radar) und opennetmon (Zeichen C, Bildschirm mit Puls)
    'm-sec': '<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="M8.5 13a3.5 3.5 0 0 1 7 0"/><circle cx="12" cy="13" r="1" fill="currentColor"/>',
    'm-net': '<rect x="2.5" y="4" width="19" height="13" rx="2"/><path d="M8 21h8"/><path d="M12 17v4"/><path d="M6 11h2.5l1.5-3 3 6 1.5-3H18"/>'
  };
  let s = '<svg xmlns="http://www.w3.org/2000/svg" style="display:none">';
  for (const k in P) s += '<symbol id="i-' + k + '" viewBox="0 0 24 24">' + P[k] + '</symbol>';
  s += '</svg>';
  document.addEventListener('DOMContentLoaded', function () {
    document.body.insertAdjacentHTML('afterbegin', s);
  });
  window.I = function (name, cls) {
    return '<svg class="ico' + (cls ? ' ' + cls : '') + '" aria-hidden="true" focusable="false"><use href="#i-' + name + '"/></svg>';
  };
  window.ICON_PATHS = P;
})();
