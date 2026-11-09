/* Entwurf openpacketmon – gemeinsamer Rahmen: Entwurfsleiste, Kopfleiste mit Aufzeichnung, Statusleiste,
   Thema, Menüs, Tastenkürzel und der Takt der laufenden Aufzeichnung. Nur für die Abnahme. */
(function () {
  const $ = function (s, r) { return (r || document).querySelector(s); };
  const $$ = function (s, r) { return Array.from((r || document).querySelectorAll(s)); };
  const esc = function (s) { return String(s == null ? '' : s).replace(/[&<>"']/g, function (c) { return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]; }); };
  const store = D.store;
  const FRAME = /[?&]frame=1/.test(location.search);
  if (FRAME) document.addEventListener('DOMContentLoaded', function () { document.body.classList.add('frame'); });
  const VERSION = '0.4.0', NEWER = '0.5.0', NEWER_DATE = '07.11.2026';

  /* ---------- Thema ---------- */
  function isDark() {
    const t = document.documentElement.getAttribute('data-theme');
    if (t) return t === 'dark';
    return window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;
  }
  function applyTheme(t) {
    const html = document.documentElement;
    if (t === 'light' || t === 'dark') html.setAttribute('data-theme', t); else html.removeAttribute('data-theme');
    store.set('theme', t || 'system');
    $$('[data-theme-btn]').forEach(function (b) { b.setAttribute('aria-pressed', String(b.dataset.themeBtn === (t || 'system'))); });
    const tb = $('#themeToggle');
    if (tb) tb.innerHTML = isDark() ? I('sun') : I('moon');
    document.dispatchEvent(new CustomEvent('opm-theme'));
  }
  applyTheme(store.get('theme') || 'system');

  /* ---------- Einstellungen des Entwurfs ---------- */
  function logoName() { return store.get('logo') || 'm-c'; }   // C gewählt (Nutzer, 2026-11-09)
  function mcp() { return store.get('mcp') || 'on'; }          // off | on | content
  function favs() { return store.get('favs') !== '0'; }
  // Neue Version: 'launcher' (mit Launcher, Linux) | 'plain' (ohne Launcher oder Windows) | 'none' (aktuell)
  function upd() { const u = store.get('upd'); return u === 'plain' || u === 'none' ? u : 'launcher'; }

  /* ---------- Hinweis auf eine neue Version (abgestimmt mit dem Entwurf des Launchers, 2026-11-09) ---------- */
  // Der Hauptknopf: mit Launcher ruft openpacketmon ihn für sich auf, sonst öffnet er die Release-Seite.
  function updPrimary(sm) {
    return upd() === 'launcher'
      ? '<button class="btn primary' + (sm ? ' sm' : '') + '" type="button" data-draft-toast="Entwurf: startet den Launcher mit der Seite von openpacketmon (läuft er schon, zeigt sein Fenster sie). Installiert wird, sobald openpacketmon geschlossen ist.">' + I('arrow-up-circle', 'sm') + 'Aktualisieren</button>'
      : '<button class="btn primary' + (sm ? ' sm' : '') + '" type="button" data-draft-toast="Entwurf: öffnet die Release-Seite von ' + NEWER + ' auf GitHub">' + I('external', 'sm') + 'Release-Seite öffnen</button>';
  }
  function updText() {
    return upd() === 'launcher'
      ? 'Der Launcher lädt, prüft und installiert es. openpacketmon muss dafür geschlossen sein; der Launcher wartet darauf.'
      : 'Das Paket liegt auf der Release-Seite auf GitHub, mit Prüfsummen in <span class="mono">SHA256SUMS</span>.';
  }
  function updPop() {
    if (upd() === 'none') return '';
    const run = D.mode() !== 'none';
    return '<div class="updpop" id="updPop" role="dialog" aria-labelledby="updTitle" hidden>' +
      '<header>' + I('arrow-up-circle') + '<div><h2 id="updTitle">openpacketmon ' + NEWER + ' ist erschienen</h2><p>installiert: ' + VERSION + ' · erschienen am ' + NEWER_DATE + '</p></div>' +
      '<button class="iconbtn sm" type="button" data-upd-close aria-label="Schließen">' + I('x', 'sm') + '</button></header>' +
      '<p>' + updText() + '</p>' +
      (upd() === 'launcher' && run ? '<p class="small muted">Beim Schließen endet die Aufzeichnung. Vorher sichern: Pakete → „Als PCAP speichern“.</p>' : '') +
      '<footer><a class="btn sm" href="einstellungen.html?notes=1#aktualisierungen">' + I('file', 'sm') + 'Was ist neu?</a>' + updPrimary(true) + '</footer></div>';
  }
  function toggleUpdPop(open) {
    const p = $('#updPop'), b = $('#updBtn'); if (!p || !b) return;
    const show = open === undefined ? p.hidden : open;
    p.hidden = !show; b.setAttribute('aria-expanded', String(show));
    if (show) { const f = p.querySelector('footer .primary'); if (f) f.focus(); }
  }

  /* ---------- Rahmen ---------- */
  const NAV = [
    ['uebersicht', 'uebersicht.html', 'overview', 'Übersicht'],
    ['verbindungen', 'verbindungen.html', 'connections', 'Verbindungen'],
    ['pakete', 'pakete.html', 'packets', 'Pakete'],
    ['statistik', 'statistik.html', 'statistics', 'Statistik'],
    ['hosts', 'hosts.html', 'hosts', 'Hosts', 1],
    ['dienste', 'dienste.html', 'services', 'Dienste', 1],
    ['programme', 'programme.html', 'programs', 'Programme', 1]
  ];
  const PAGES = [['index.html', 'Entwurf'], ['aufzeichnung.html', 'Aufzeichnung'], ['uebersicht.html', 'Übersicht'], ['verbindungen.html', 'Verbindungen'], ['verbindung.html', 'Verbindung'],
    ['pakete.html', 'Pakete'], ['paket.html', 'Paket'], ['stream.html', 'Stream'], ['statistik.html', 'Statistik'], ['hosts.html', 'Hosts'], ['dienste.html', 'Dienste'], ['programme.html', 'Programme'],
    ['benachrichtigungen.html', 'Benachrichtigungen'], ['einstellungen.html', 'Einstellungen'], ['logo.html', 'Logo'], ['breiten.html', 'Breiten']];

  function capChip() {
    const m = D.mode();
    if (m === 'none') return '<a class="btn primary sm" id="cap" href="aufzeichnung.html">' + I('play', 'sm') + 'Aufzeichnung starten</a>';
    const run = m === 'running';
    return '<div class="cap ' + m + '" id="cap" role="group" aria-label="Aufzeichnung">' +
      '<a href="aufzeichnung.html" title="Zur Seite Aufzeichnung"><span class="dot" aria-hidden="true"></span><span class="what">' + (run ? 'Aufzeichnung läuft' : 'Pausiert') + '</span>' +
      '<span class="src">' + D.ADAPTER.name + '</span><span class="time" id="capTime">' + D.fdur(D.now()) + '</span></a>' +
      (run ? '<button class="cbtn" type="button" data-cap="pause" aria-label="Pausieren" title="Pausieren">' + I('pause', 'sm') + '</button>'
        : '<button class="cbtn" type="button" data-cap="resume" aria-label="Fortsetzen" title="Fortsetzen">' + I('play', 'sm') + '</button>') +
      '<button class="cbtn stop" type="button" data-cap="stop" aria-label="Beenden" title="Beenden">' + I('stop', 'sm') + '</button></div>';
  }

  function statusbar() {
    const m = D.mode(); const n = D.count();
    const mc = mcp();
    const mcpHtml = mc === 'off' ? '' : '<span class="sep hide-sm">|</span><a class="mcp hide-sm' + (mc === 'content' ? ' content' : '') + '" href="einstellungen.html#mcp" title="Zu den Einstellungen"><i aria-hidden="true"></i>' + (mc === 'content' ? 'KI-Assistenten lesen Paketinhalte' : 'KI-Assistenten haben Zugriff') + '</a>';
    const left = m === 'none' ? '<span>Es läuft keine Aufzeichnung.</span>' :
      '<span>Pakete <span class="num" id="sbPk">' + D.fnum(n) + '</span></span><span class="sep hide-sm">|</span>' +
      '<span class="hide-sm">Paketspeicher <span class="num" id="sbStore">' + D.fnum(n) + '</span> von <span class="num">500.000</span><span class="hide-lg">, vollständig</span></span><span class="sep hide-sm">|</span>' +
      '<span class="hide-md">Verworfen <span class="num">0</span></span>';
    // Links darf gekürzt werden, rechts nie: der Hinweis auf DB-IP (CC BY 4.0) bleibt bei jeder Breite sichtbar.
    return '<footer class="statusbar" role="contentinfo"><span class="sl">' + left + mcpHtml + '</span><span class="sp"></span><span class="sr">' +
      (upd() === 'none' ? '' : '<button class="upd" type="button" id="updBtn" aria-expanded="false" aria-controls="updPop" title="Neue Version: Hinweise und Aktualisieren">' + I('download', 'sm') + ' Version ' + NEWER + ' verfügbar</button><span class="sep">|</span>') +
      '<span class="hide-md">openpacketmon ' + VERSION + '</span>' +
      '<a class="hide-lg" href="#" data-draft-toast="Entwurf: öffnet github.com/tomCSLK/openpacketmon">Quellcode</a>' +
      '<a class="hide-lg" href="#" data-draft-toast="Entwurf: öffnet github.com/GyulyVGC/sniffnet">basiert auf Sniffnet</a>' +
      '<a href="einstellungen.html#lizenzen">Lizenzen</a>' +
      '<a href="#" data-draft-toast="Entwurf: öffnet db-ip.com (Hinweis nach CC BY 4.0, bleibt immer sichtbar)">IP Geolocation by DB-IP</a></span></footer>';
  }

  function shell(active) {
    const file = location.pathname.split('/').pop() || 'index.html';
    const mode = D.mode();
    const draft =
      '<div class="draftbar" id="draftbar" role="region" aria-label="Entwurfsleiste">' +
      '<strong>ENTWURF</strong><span>Beispieldaten erfunden</span><span aria-hidden="true">·</span>' +
      '<span class="pages">' + PAGES.map(function (p) { return p[0] === file ? '<span><b>' + p[1] + '</b></span>' : '<a href="' + p[0] + '">' + p[1] + '</a>'; }).join(' ') + '</span>' +
      '<label class="sr-only" for="pgSel">Seite</label><select id="pgSel">' + PAGES.map(function (p) { return '<option value="' + p[0] + '"' + (p[0] === file ? ' selected' : '') + '>' + p[1] + '</option>'; }).join('') + '</select>' +
      '<span class="sp"></span><span class="ctl"><span class="opt">Aufzeichnung:</span>' +
      [['running', 'läuft'], ['paused', 'pausiert'], ['none', 'keine']].map(function (x) { return '<button type="button" data-mode="' + x[0] + '" aria-pressed="' + (mode === x[0]) + '">' + x[1] + '</button>'; }).join('') +
      '<span class="opt">Update:</span>' +
      [['launcher', 'mit Launcher', 'Neue Version, der Launcher ist installiert (Linux)'], ['plain', 'ohne Launcher', 'Neue Version, kein Launcher oder Windows'], ['none', 'keins', 'Installierte Version ist aktuell']].map(function (x) { return '<button type="button" data-upd="' + x[0] + '" title="' + x[2] + '" aria-pressed="' + (upd() === x[0]) + '">' + x[1] + '</button>'; }).join('') +
      '<span class="opt">Thema:</span><button type="button" data-theme-btn="system">System</button><button type="button" data-theme-btn="light">Hell</button><button type="button" data-theme-btn="dark">Dunkel</button></span>' +
      '</div>';
    const moreActive = NAV.some(function (x) { return x[4] && x[0] === active; });
    const top =
      '<header class="topbar">' +
      '<a class="brand" href="uebersicht.html" aria-label="openpacketmon, zur Übersicht">' + I(logoName(), 'mark') + '<span class="name">openpacketmon</span></a>' +
      '<nav class="nav" aria-label="Hauptnavigation">' +
      NAV.map(function (x) { return '<a href="' + x[1] + '"' + (x[4] ? ' class="opt"' : '') + (x[0] === active ? ' aria-current="page"' : '') + ' title="' + x[3] + '">' + I(x[2]) + '<span class="lbl">' + x[3] + '</span></a>'; }).join('') +
      '<details class="more menu' + (moreActive ? ' current' : '') + '"><summary aria-label="Weitere Seiten">' + I('more') + '<span class="lbl">Mehr</span>' + I('chev-down', 'sm') + '</summary><div class="menu-pop" role="menu">' +
      NAV.filter(function (x) { return x[4]; }).map(function (x) { return '<a role="menuitem" href="' + x[1] + '"' + (x[0] === active ? ' aria-current="page"' : '') + '>' + I(x[2]) + x[3] + '</a>'; }).join('') + '</div></details>' +
      '</nav><span class="sp"></span>' + capChip() +
      '<a class="iconbtn" href="benachrichtigungen.html" aria-label="Benachrichtigungen' + (mode !== 'none' ? ', ' + D.NOTES.length + ' neue' : '') + '" title="Benachrichtigungen"' + (active === 'benachrichtigungen' ? ' aria-current="page"' : '') + '>' + I('bell') + (mode !== 'none' ? '<span class="badge">' + D.NOTES.length + '</span>' : '') + '</a>' +
      '<button class="iconbtn" id="themeToggle" type="button" aria-label="Hell/Dunkel umschalten" title="Hell/Dunkel (t)"></button>' +
      '<button class="iconbtn" type="button" data-help aria-label="Tastenkürzel" title="Tastenkürzel (?)">' + I('help-circle') + '</button>' +
      '<a class="iconbtn" href="einstellungen.html" aria-label="Einstellungen" title="Einstellungen"' + (active === 'einstellungen' ? ' aria-current="page"' : '') + '>' + I('settings') + '</a>' +
      '</header>';
    const overlays =
      '<div class="scrim" id="scrim"></div>' +
      '<div class="toast" id="toast" role="status" aria-live="polite"><span id="toastMsg"></span></div>' +
      '<div class="modal" id="helpModal" role="dialog" aria-modal="true" aria-labelledby="helpTitle"><header><h2 id="helpTitle">Tastenkürzel</h2><button class="iconbtn" type="button" data-close aria-label="Schließen">' + I('x') + '</button></header>' +
      '<div class="mbody"><div class="keys">' +
        '<span class="sec">Überall</span>' +
        '<span><kbd>/</kbd></span><span>Filterfeld der Seite (sonst zu den Paketen)</span>' +
        '<span><kbd>t</kbd></span><span>Hell / Dunkel umschalten</span>' +
        '<span><kbd>?</kbd></span><span>Diese Hilfe</span>' +
        '<span><kbd>Alt</kbd> <kbd>←</kbd></span><span>Zur vorherigen Seite</span>' +
        '<span><kbd>Esc</kbd></span><span>Schließen, Vorschläge ausblenden</span>' +
        '<span class="sec">Pakete</span>' +
        '<span><kbd>Strg</kbd> <kbd>G</kbd></span><span>Gehe zu Paket</span>' +
        '<span><kbd>↑</kbd> <kbd>↓</kbd></span><span>Paket wählen</span>' +
        '<span><kbd>Enter</kbd></span><span>Paketdetails öffnen</span>' +
        '<span><kbd>Ende</kbd></span><span>Ans Ende, der Liste folgen</span>' +
        '<span class="sec">Nur im Programm (im Browser belegt)</span>' +
        '<span><kbd>Strg</kbd> <kbd>1</kbd> … <kbd>7</kbd></span><span>Übersicht, Verbindungen, Pakete, Statistik, Hosts, Dienste, Programme</span>' +
        '<span><kbd>Strg</kbd> <kbd>,</kbd></span><span>Einstellungen</span></div></div>' +
      '<footer><button class="btn" type="button" data-close>Schließen</button></footer></div>' +
      '<div class="modal" id="stopModal" role="dialog" aria-modal="true" aria-labelledby="stopTitle"><header><h2 id="stopTitle">' + I('alert') + 'Aufzeichnung beenden?</h2><button class="iconbtn" type="button" data-close aria-label="Schließen">' + I('x') + '</button></header>' +
      '<div class="mbody"><p>Beim Beenden werden die gesammelten Daten verworfen: ' + D.fnum(D.count()) + ' Pakete und ' + D.fnum(D.CONNS.filter(function (c) { return c.packets; }).length) + ' Verbindungen.</p><p class="small muted">Vorher sichern: Pakete → „Als PCAP speichern“.</p></div>' +
      '<footer><button class="btn" type="button" data-close>Abbrechen</button><button class="btn danger" type="button" id="stopYes">' + I('stop', 'sm') + 'Beenden</button></footer></div>';

    document.body.insertAdjacentHTML('afterbegin', draft + top);
    document.body.insertAdjacentHTML('beforeend', statusbar() + updPop() + overlays);
    applyTheme(store.get('theme') || 'system');
    $('#pgSel').addEventListener('change', function (e) { location.href = e.target.value; });
    $$('[data-theme-btn]').forEach(function (b) { b.addEventListener('click', function () { applyTheme(b.dataset.themeBtn); }); });
    $$('[data-mode]').forEach(function (b) { b.addEventListener('click', function () { D.setMode(b.dataset.mode); location.reload(); }); });
    $$('[data-upd]').forEach(function (b) { b.addEventListener('click', function () { store.set('upd', b.dataset.upd); location.reload(); }); });
    $('#themeToggle').addEventListener('click', function () { applyTheme(isDark() ? 'light' : 'dark'); });
    if (window.matchMedia) window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', function () { applyTheme(store.get('theme') || 'system'); });
    $('#stopYes').addEventListener('click', function () { D.setMode('none'); location.href = 'aufzeichnung.html'; });

    document.addEventListener('click', function (e) {
      const cp = e.target.closest('[data-cap]');
      if (cp) {
        e.preventDefault();
        if (cp.dataset.cap === 'stop') { openModal('stopModal'); return; }
        D.setMode(cp.dataset.cap === 'pause' ? 'paused' : 'running'); location.reload(); return;
      }
      const h = e.target.closest('[data-help]'); if (h) { e.preventDefault(); closeMenus(); openModal('helpModal'); }
      const c2 = e.target.closest('[data-close]'); if (c2) { e.preventDefault(); closeModals(); }
      if (e.target.closest('#updBtn')) { e.preventDefault(); closeMenus(); toggleUpdPop(); return; }
      if (e.target.closest('[data-upd-close]')) { e.preventDefault(); toggleUpdPop(false); $('#updBtn').focus(); return; }
      const t = e.target.closest('[data-draft-toast]'); if (t) { e.preventDefault(); closeMenus(); toggleUpdPop(false); toast(t.dataset.draftToast); }
      if (!e.target.closest('.menu')) closeMenus();
      if (!e.target.closest('#updPop') || e.target.closest('#updPop a')) toggleUpdPop(false);
    });
    $('#scrim').addEventListener('click', closeModals);
    document.addEventListener('keydown', function (e) {
      if (e.key === 'Escape') { closeModals(); closeMenus(); if ($('#updPop') && !$('#updPop').hidden) { toggleUpdPop(false); $('#updBtn').focus(); } }
      if (e.altKey && e.key === 'ArrowLeft') { e.preventDefault(); back(); return; }
      if (isTyping(e)) return;
      if (e.key === '/') {
        e.preventDefault();
        const f = $('[data-slash]');
        if (f) { f.focus(); f.select(); } else location.href = 'pakete.html?focus=1';
        return;
      }
      if (e.key === '?') { e.preventDefault(); openModal('helpModal'); return; }
      if (e.key === 't') { applyTheme(isDark() ? 'light' : 'dark'); }
    });
    startTicker();
  }
  function back(fallback) {
    if (history.length > 1 && document.referrer && document.referrer.indexOf(location.host) >= 0) history.back();
    else location.href = fallback || 'uebersicht.html';
  }
  function isTyping(e) {
    const t = e.target;
    return (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.tagName === 'SELECT' || t.isContentEditable)) || !!$('.modal.open') || e.ctrlKey || e.metaKey || e.altKey;
  }
  function closeMenus() { $$('details.menu[open]').forEach(function (d) { d.removeAttribute('open'); }); }
  let lastFocus = null;
  function openModal(id) {
    lastFocus = document.activeElement;
    $('#' + id).classList.add('open'); $('#scrim').classList.add('open');
    const f = $('#' + id).querySelector('footer button:last-child') || $('#' + id).querySelector('button');
    if (f) f.focus();
  }
  function closeModals() {
    const any = $('.modal.open');
    $$('.modal.open').forEach(function (m) { m.classList.remove('open'); });
    if ($('#scrim')) $('#scrim').classList.remove('open');
    if (any && lastFocus) lastFocus.focus();
  }
  let toastTimer = null;
  function toast(msg) {
    $('#toastMsg').textContent = msg; $('#toast').classList.add('open');
    clearTimeout(toastTimer); toastTimer = setTimeout(function () { $('#toast').classList.remove('open'); }, 4000);
  }

  /* ---------- Takt der laufenden Aufzeichnung: alle 0,3 s kommen Pakete dazu (wie die Live-Liste der Anwendung) ---------- */
  function startTicker() {
    if (D.mode() !== 'running') return;
    setInterval(function () {
      if (document.hidden) return;
      const from = D.count() + 1;
      const to = D.grow(14 + Math.floor(Math.random() * 12));
      const pk = $('#sbPk'); if (pk) pk.textContent = D.fnum(to);
      const sb = $('#sbStore'); if (sb) sb.textContent = D.fnum(to);
      const ct = $('#capTime'); if (ct) ct.textContent = D.fdur(D.now());
      document.dispatchEvent(new CustomEvent('opm-tick', { detail: { from: from, to: to } }));
    }, 300);
  }

  /* ---------- Bausteine ---------- */
  function tbar(inB, outB, max) {
    const w = max ? (inB + outB) / max * 100 : 100;
    const t = inB + outB || 1;
    return '<span class="tbar" style="width:' + Math.max(w, 1.5).toFixed(1) + '%" title="Eingehend ' + D.fbytes(inB) + ', Ausgehend ' + D.fbytes(outB) + '"><i class="in" style="width:' + (inB / t * 100).toFixed(1) + '%"></i><i class="out" style="width:' + (outB / t * 100).toFixed(1) + '%"></i></span>';
  }
  function star(on, label) {
    return '<button class="star" type="button" aria-pressed="' + !!on + '" aria-label="' + (on ? 'Aus den Favoriten entfernen' : 'Zu den Favoriten') + (label ? ': ' + esc(label) : '') + '" title="' + (on ? 'Aus den Favoriten entfernen' : 'Zu den Favoriten') + '"><svg class="ico sm" viewBox="0 0 24 24" aria-hidden="true"><use href="#i-star"/></svg></button>';
  }
  function flag(cc) { return cc ? '<span class="tag cc" title="Land">' + cc + '</span>' : ''; }

  window.App = { $: $, $$: $$, esc: esc, store: store, shell: shell, toast: toast, openModal: openModal, closeModals: closeModals, back: back,
    isDark: isDark, logoName: logoName, mcp: mcp, favs: favs, upd: upd, updPrimary: updPrimary, updText: updText, tbar: tbar, star: star, flag: flag,
    FRAME: FRAME, VERSION: VERSION, NEWER: NEWER, NEWER_DATE: NEWER_DATE, NAV: NAV };
})();
