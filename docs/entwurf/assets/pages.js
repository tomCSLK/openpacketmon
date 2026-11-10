/* Entwurf openpacketmon – Inhalt der Seiten. Nur für die Abnahme, kein Anwendungscode. */
(function () {
  const $ = App.$, $$ = App.$$, esc = App.esc;
  const q = new URLSearchParams(location.search);
  const DURNAME = function () { return D.fdur(D.now()) + ' min'; };

  /* ---------- Bausteine ---------- */
  function pagehead(h1, sub, actions, opt) {
    opt = opt || {};
    return '<div class="pagehead"><div>' + (opt.crumb ? '<div class="crumb">' + opt.crumb + '</div>' : '') +
      '<h1>' + (opt.back ? '<a class="back" href="' + opt.back + '" data-back="' + opt.back + '" aria-label="Zur vorherigen Seite" title="Zur vorherigen Seite (Alt+Links)">' + I('back') + '</a>' : '') + h1 + '</h1>' +
      (sub ? '<p>' + sub + '</p>' : '') + '</div>' + (actions ? '<div class="btngroup">' + actions + '</div>' : '') + '</div>';
  }
  function noCapture() {
    return '<section class="card"><div class="empty">' + I('capture', 'xl') + '<h2>Es läuft keine Aufzeichnung.</h2><p class="small">Die Seiten füllen sich, sobald eine Aufzeichnung läuft oder eine Datei gelesen wird.</p>' +
      '<a class="btn primary" href="aufzeichnung.html">' + I('play', 'sm') + 'Aufzeichnung starten</a></div></section>';
  }
  function none() { return D.mode() === 'none'; }
  function url(file, params) {
    const u = new URLSearchParams();
    Object.keys(params || {}).forEach(function (k) { if (params[k] != null && params[k] !== '') u.set(k, params[k]); });
    const s = u.toString(); return file + (s ? '?' + s : '');
  }
  function connLabel(c) {
    const a = c.dir === 'in' ? c.src : c.src, b = c.dst;
    const port = function (x, p) { return x.indexOf(':') >= 0 ? '[' + x + ']:' + p : x + ':' + p; };
    if (c.proto === 'ARP' || c.proto === 'ICMP') return c.proto + ' ' + a + ' → ' + b;
    return c.proto + ' ' + port(a, c.sport) + ' → ' + port(b, c.dport);
  }
  function addr(x, p) { if (p == null || p === 0 && x.indexOf('.') < 0 && x.indexOf(':') < 0) return x; return (x.indexOf(':') >= 0 ? '[' + x + ']' : x) + (p ? ':' + p : ''); }
  function hostName(h) { return h.name; }
  function hostSub(h) {
    if (h.tags) return h.tags.join(', ') + (h.name !== h.ip ? ' · ' + h.ip : '');
    return (h.name !== h.ip ? h.ip + ' · ' : '') + 'AS' + h.asn + ' ' + h.as;
  }
  function svcLabel(c) { return c.svc ? c.svc : '–'; }
  function unitSeg(id, cur) {
    return '<div class="seg sm" id="' + id + '" role="group" aria-label="Einheit">' + [['bytes', 'Bytes'], ['bits', 'Bits'], ['packets', 'Pakete']].map(function (u) {
      return '<button type="button" data-u="' + u[0] + '" aria-pressed="' + (u[0] === cur) + '">' + u[1] + '</button>';
    }).join('') + '</div>';
  }
  function bindSeg(id, fn) {
    $$('#' + id + ' button').forEach(function (b) {
      b.addEventListener('click', function () { $$('#' + id + ' button').forEach(function (o) { o.setAttribute('aria-pressed', String(o === b)); }); fn(b.dataset.u || b.dataset.v); });
    });
  }
  function niceMax(m) { if (m <= 0) return 1; const e = Math.pow(10, Math.floor(Math.log10(m))); const f = m / e; return (f <= 1 ? 1 : f <= 2 ? 2 : f <= 2.5 ? 2.5 : f <= 5 ? 5 : 10) * e; }
  function rateFmt(unit, v) { return unit === 'bits' ? D.fbits(v) + '/s' : unit === 'packets' ? D.fnum(v, v < 10 ? 1 : 0) + '/s' : D.fbytes(v) + '/s'; }

  /* Linien-Diagramm: Eingehend (Blau, Fläche) und Ausgehend (Orange, gestrichelt) */
  function drawChart(box, pts, opt) {
    const W = Math.max(320, box.clientWidth), H = opt.h || 220, pl = 64, pr = 12, pt = 10, pb = 26;
    let max = 0; pts.forEach(function (p) { max = Math.max(max, p.a, p.b); });
    const top = niceMax(max * 1.08);
    const n = pts.length;
    const x = function (i) { return pl + (W - pl - pr) * (n > 1 ? i / (n - 1) : 0); };
    const y = function (v) { return pt + (H - pt - pb) * (1 - v / top); };
    let g = '';
    for (let k = 0; k <= 4; k += 1) { const v = top * k / 4; g += '<line x1="' + pl + '" x2="' + (W - pr) + '" y1="' + y(v).toFixed(1) + '" y2="' + y(v).toFixed(1) + '"/>'; }
    let ax = '';
    for (let k = 0; k <= 4; k += 1) { const v = top * k / 4; ax += '<text x="' + (pl - 8) + '" y="' + (y(v) + 4).toFixed(1) + '" text-anchor="end">' + esc(opt.yfmt(v)) + '</text>'; }
    const ticks = opt.ticks || 6;
    for (let k = 0; k <= ticks; k += 1) { const i = Math.round((n - 1) * k / ticks); ax += '<text x="' + x(i).toFixed(1) + '" y="' + (H - 6) + '" text-anchor="' + (k === 0 ? 'start' : k === ticks ? 'end' : 'middle') + '">' + esc(opt.xfmt(pts[i], i)) + '</text>'; }
    let d1 = '', d2 = '';
    pts.forEach(function (p, i) { d1 += (i ? 'L' : 'M') + x(i).toFixed(1) + ' ' + y(p.a).toFixed(1); d2 += (i ? 'L' : 'M') + x(i).toFixed(1) + ' ' + y(p.b).toFixed(1); });
    const area = d1 + 'L' + x(n - 1).toFixed(1) + ' ' + y(0) + 'L' + x(0).toFixed(1) + ' ' + y(0) + 'Z';
    box.innerHTML = '<div class="chart"><svg viewBox="0 0 ' + W + ' ' + H + '" width="' + W + '" height="' + H + '" role="img" aria-label="' + esc(opt.label) + '">' +
      '<g class="grid">' + g + '</g><path class="ar1" d="' + area + '"/><path class="ln ln1" d="' + d1 + '"/><path class="ln ln2" d="' + d2 + '"/>' +
      '<g class="axis">' + ax + '</g><line class="cross" y1="' + pt + '" y2="' + (H - pb) + '" style="display:none"/><circle class="dotp p1" r="4" style="display:none"/><circle class="dotp p2" r="4" style="display:none"/></svg></div><div class="ctip"></div>';
    const svg = $('svg', box), cross = $('.cross', box), p1 = $('.p1', box), p2 = $('.p2', box), tip = $('.ctip', box);
    svg.addEventListener('mousemove', function (e) {
      const r = svg.getBoundingClientRect(); const mx = (e.clientX - r.left) * W / r.width;
      const i = Math.max(0, Math.min(n - 1, Math.round((mx - pl) / ((W - pl - pr) / Math.max(1, n - 1)))));
      const p = pts[i];
      [cross, p1, p2].forEach(function (el) { el.style.display = ''; });
      cross.setAttribute('x1', x(i)); cross.setAttribute('x2', x(i));
      p1.setAttribute('cx', x(i)); p1.setAttribute('cy', y(p.a)); p2.setAttribute('cx', x(i)); p2.setAttribute('cy', y(p.b));
      tip.innerHTML = esc(opt.tipx(p, i)) + '<br><span style="color:var(--s1)">●</span> Eingehend <b>' + esc(opt.yfmt(p.a)) + '</b><br><span style="color:var(--s2)">●</span> Ausgehend <b>' + esc(opt.yfmt(p.b)) + '</b>';
      tip.style.display = 'block';
      const tx = x(i) * r.width / W + 14; tip.style.left = Math.min(tx, r.width - tip.offsetWidth - 4) + 'px'; tip.style.top = '18px';
    });
    svg.addEventListener('mouseleave', function () { [cross, p1, p2, tip].forEach(function (el) { el.style.display = 'none'; }); });
  }

  /* ---------- Filterfeld mit Prüfung, Vorschlägen und Hilfe ---------- */
  const OPS = [['==', 'gleich'], ['!=', 'ungleich'], ['contains', 'enthält Text'], ['in', 'einer von, z. B. in {80 443}'], ['<', 'kleiner'], ['>', 'größer'], ['<=', 'kleiner oder gleich'], ['>=', 'größer oder gleich']];
  const VALHINT = { ip: 'IP-Adresse oder Netz, z. B. 192.0.2.20 oder 198.51.100.0/24', n: 'Zahl, z. B. 443', s: 'Text, z. B. example' };
  function filterUI(id, kind, value, placeholder) {
    return '<div class="ffield" id="' + id + '-f">' + I('filter') +
      '<label class="sr-only" for="' + id + '">' + (kind === 'c' ? 'Verbindungsfilter' : 'Anzeigefilter') + '</label>' +
      '<input id="' + id + '" type="text" data-slash spellcheck="false" autocomplete="off" role="combobox" aria-expanded="false" aria-controls="' + id + '-s" placeholder="' + esc(placeholder) + '" value="' + esc(value || '') + '">' +
      '<button class="iconbtn sm clear" type="button" aria-label="Filter leeren" title="Filter leeren"' + (value ? '' : ' hidden') + '>' + I('x', 'sm') + '</button>' +
      '<div class="sugg" id="' + id + '-s" role="listbox"></div></div>' +
      '<button class="iconbtn" type="button" id="' + id + '-h" aria-expanded="false" aria-label="Hilfe zum Filter" title="' + (kind === 'c' ? 'Hilfe zum Verbindungsfilter' : 'Hilfe zum Anzeigefilter') + '">' + I('help-circle') + '</button>' +
      '<button class="btn primary" type="button" id="' + id + '-a">Anwenden</button>';
  }
  function filterExtras(id, kind) {
    const ex = kind === 'c' ? ['ip.addr == 192.0.2.40', 'port == 443', 'domain contains example', 'program == firefox', 'direction == incoming', 'country == US', 'favorite', 'tcp and bytes > 1000000'] :
      ['dns', 'tls.sni contains example', 'ip.addr == 192.0.2.0/24', 'tcp.analysis.flags', 'http.request.method == GET', 'tcp.port in {80 443}', 'sip or rtp', 'not arp and not mdns'];
    const txt = kind === 'c' ? 'Dieselbe Sprache wie der Anzeigefilter der Pakete, mit Feldern der Verbindungen. Ein Protokoll allein (tcp, udp, icmp, arp, ipv6) wählt die Verbindungen dieses Protokolls, favorite allein die Verbindungen von Favoriten. Vergleiche mit == != &lt; &lt;= &gt; &gt;= contains und in {80 443}, verknüpft mit and, or, not und Klammern. remote ist die Gegenstelle, domain der Name des Hosts, as.name und country kommen aus der Geolokalisierung.' :
      'Ein Protokoll allein (dns, tcp) wählt die Pakete, die es enthalten; ein Feld allein (tls.sni, tcp.flags.syn) die Pakete, die es haben. Vergleiche mit == != &lt; &lt;= &gt; &gt;= contains und in {80 443}, verknüpft mit and, or, not und Klammern. Text ohne Groß-/Kleinschreibung, Adressen auch als Netz. domain sucht in DNS-Namen, TLS-SNI und HTTP-Host; tcp.analysis.flags findet alle Befunde der TCP-Analyse; voip.call_id wählt alle Pakete eines Anrufs.';
    return '<div class="ferr" id="' + id + '-e" role="alert">' + I('x-circle', 'sm') + '<span></span></div>' +
      '<div class="helppop" id="' + id + '-p">' + txt + '<div class="ex">' + ex.map(function (e) { return '<button type="button" data-ex="' + esc(e) + '">' + esc(e) + '</button>'; }).join('') + '</div>' +
      (kind === 'p' ? '<div class="rowkeys" style="margin-top:var(--sp-3)"><span class="muted">Farben:</span>' + [['bad', 'Fehler/Reset/TCP-Probleme'], ['tcp', 'TCP-Auf-/Abbau'], ['dns', 'DNS'], ['local', 'ARP/DHCP/SSDP/NTP'], ['tls', 'TLS'], ['http', 'HTTP'], ['icmp', 'ICMP'], ['udp', 'QUIC/UDP'], ['voip', 'SIP/RTP']].map(function (r) { return '<span class="rowkey ' + r[0] + '">' + r[1] + '</span>'; }).join('') + '</div>' : '') + '</div>';
  }
  function bindFilter(id, kind, onApply) {
    const inp = $('#' + id), box = $('#' + id + '-f'), sg = $('#' + id + '-s'), err = $('#' + id + '-e'), help = $('#' + id + '-p'), hb = $('#' + id + '-h');
    const fields = kind === 'c' ? D.CFIELDS : D.PFIELDS;
    const parse = kind === 'c' ? D.connFilter : D.packetFilter;
    let sel = -1, items = [];
    function check() {
      const v = inp.value.trim();
      $('.clear', box).hidden = !inp.value;
      box.classList.remove('ok', 'bad');
      if (!v) { err.classList.remove('open'); return true; }
      const r = parse(v);
      if (r.err) { box.classList.add('bad'); $('span', err).textContent = r.err; return false; }
      box.classList.add('ok'); err.classList.remove('open'); return true;
    }
    function lastWord() { const v = inp.value.slice(0, inp.selectionStart == null ? inp.value.length : inp.selectionStart); const m = /([^\s(){}]*)$/.exec(v); return { word: m[1], before: v.slice(0, v.length - m[1].length) }; }
    function prevToken(before) { const m = /([^\s(){}]+)\s*$/.exec(before); return m ? m[1].toLowerCase() : ''; }
    function suggest() {
      const lw = lastWord(); const w = lw.word.toLowerCase(); const prev = prevToken(lw.before);
      items = []; let head = '';
      if (!w && prev in fields && fields[prev] !== 'p' && fields[prev] !== 'b') {
        head = 'Vergleich'; items = OPS.map(function (o) { return { ins: o[0] + ' ', code: o[0], hint: o[1] }; });
      } else if (!w && ['==', '!=', 'contains', '<', '>', '<=', '>='].indexOf(prev) >= 0) {
        const f = prevToken(lw.before.replace(/\S+\s*$/, ''));
        if (fields[f]) {
          head = 'Wert';
          let vals = [];
          if (f === 'domain' || f === 'tls.sni' || f === 'dns.qry.name' || f === 'http.host') vals = D.HOSTS.filter(function (h) { return !h.tags; }).map(function (h) { return h.name; });
          else if (f === 'program') vals = Object.keys(D.PROGS);
          else if (f === 'service') vals = ['https', 'http', 'domain', 'imaps', 'ssh', 'microsoft-ds', 'ntp', 'mdns', 'ssdp'];
          else if (f === 'country') vals = ['DE', 'NL', 'US', 'IE'];
          else if (f === 'direction') vals = ['outgoing', 'incoming'];
          else if (fields[f] === 'ip') vals = ['192.0.2.20', '192.0.2.0/24', '198.51.100.23', '203.0.113.44'];
          items = vals.slice(0, 8).map(function (v) { return { ins: (/\s/.test(v) ? '"' + v + '"' : v) + ' ', code: v, hint: '' }; });
          if (!items.length) items = [{ ins: '', code: '…', hint: VALHINT[fields[f]] || '' }];
        }
      } else if (w) {
        head = 'Felder und Protokolle';
        const words = Object.keys(fields).concat(['and', 'or', 'not', 'contains']);
        items = words.filter(function (k) { return k.indexOf(w) === 0 && k !== w; }).slice(0, 9).map(function (k) { return { ins: k + ' ', code: k, hint: D.HINTS[k] || (fields[k] === 'n' ? 'Zahl' : fields[k] === 'ip' ? 'Adresse' : '') }; });
      } else if (!inp.value.trim()) {
        head = 'Häufig';
        items = (kind === 'c' ? ['tcp', 'udp', 'ip.addr', 'port', 'domain', 'program', 'service', 'country', 'favorite'] : ['dns', 'tls', 'http', 'quic', 'tcp', 'udp', 'ip.addr', 'tcp.port', 'domain', 'tcp.analysis.flags']).map(function (k) { return { ins: k + ' ', code: k, hint: D.HINTS[k] || '' }; });
      } else if (prev && !(prev in fields) && ['and', 'or', 'not', '(', '{'].indexOf(prev) < 0) {
        head = 'Verknüpfen'; items = [{ ins: 'and ', code: 'and', hint: 'Beide Bedingungen' }, { ins: 'or ', code: 'or', hint: 'Eine der Bedingungen' }];
      }
      sel = -1;
      if (!items.length) { sg.classList.remove('open'); inp.setAttribute('aria-expanded', 'false'); return; }
      sg.innerHTML = '<div class="shead">' + head + '</div>' + items.map(function (it, i) { return '<button type="button" role="option" data-i="' + i + '" tabindex="-1"><code>' + esc(it.code) + '</code><span>' + esc(it.hint) + '</span></button>'; }).join('');
      sg.classList.add('open'); inp.setAttribute('aria-expanded', 'true');
    }
    function pick(i) {
      const it = items[i]; if (!it || !it.ins) return;
      const lw = lastWord(); const rest = inp.value.slice(inp.selectionStart == null ? inp.value.length : inp.selectionStart);
      inp.value = lw.before + it.ins + rest.replace(/^\S*/, '');
      const pos = (lw.before + it.ins).length; inp.setSelectionRange(pos, pos); inp.focus();
      check(); suggest();
    }
    function mark() { $$('button', sg).forEach(function (b, i) { b.setAttribute('aria-selected', String(i === sel)); if (i === sel) b.scrollIntoView({ block: 'nearest' }); }); }
    function apply() {
      if (!check()) { err.classList.add('open'); return; }
      sg.classList.remove('open'); onApply(inp.value.trim());
    }
    inp.addEventListener('input', function () { check(); suggest(); });
    inp.addEventListener('focus', suggest);
    inp.addEventListener('blur', function () { setTimeout(function () { sg.classList.remove('open'); inp.setAttribute('aria-expanded', 'false'); }, 150); });
    inp.addEventListener('keydown', function (e) {
      const open = sg.classList.contains('open');
      if (e.key === 'ArrowDown' && open) { e.preventDefault(); sel = Math.min(items.length - 1, sel + 1); mark(); }
      else if (e.key === 'ArrowUp' && open) { e.preventDefault(); sel = Math.max(-1, sel - 1); mark(); }
      else if ((e.key === 'Enter' || e.key === 'Tab') && open && sel >= 0) { e.preventDefault(); pick(sel); }
      else if (e.key === 'Enter') { e.preventDefault(); e.stopPropagation(); apply(); }
      else if (e.key === 'Escape') { if (open) { e.stopPropagation(); sg.classList.remove('open'); } else inp.blur(); }
    });
    sg.addEventListener('mousedown', function (e) { const b = e.target.closest('button'); if (b) { e.preventDefault(); pick(+b.dataset.i); } });
    $('.clear', box).addEventListener('click', function () { inp.value = ''; check(); apply(); inp.focus(); });
    $('#' + id + '-a').addEventListener('click', apply);
    hb.addEventListener('click', function () { const o = help.classList.toggle('open'); hb.setAttribute('aria-expanded', String(o)); });
    $$('[data-ex]', help).forEach(function (b) { b.addEventListener('click', function () { inp.value = b.dataset.ex; check(); apply(); }); });
    check();
    return { set: function (v) { inp.value = v; check(); } };
  }
  function setParam(k, v) {
    const u = new URLSearchParams(location.search);
    if (v == null || v === '') u.delete(k); else u.set(k, v);
    history.replaceState(null, '', location.pathname + (u.toString() ? '?' + u : ''));
  }

  /* Auswahl der Pakete: Anzeigefilter f, Verbindung c, Verbindungsfilter cf */
  function selection() {
    const f = q.get('f') || '', c = q.get('c') || '', cf = q.get('cf') || '';
    const conn = c && D.CBY[c] ? D.CBY[c] : null;
    let cset = null;
    if (cf) { const r = D.connFilter(cf); if (!r.err) { cset = {}; D.CONNS.forEach(function (x) { if (x.packets && r.test(x)) cset[x.id] = 1; }); } }
    return { f: f, c: c, cf: cf, conn: conn, cset: cset, test: function (p, pf) { return (!conn || p.conn === conn.id) && (!cset || cset[p.conn]) && (!pf || pf.test(p)); } };
  }
  function connNote(s, here) {
    if (s.conn) {
      const id = s.conn.id;
      return '<div class="notice info">' + I('connections') + '<div>Nur die Pakete der Verbindung <a class="mono" href="verbindung.html?id=' + id + '">' + esc(connLabel(s.conn)) + '</a> (beide Richtungen) · ' +
        (here !== 'pakete' ? '<a href="pakete.html?c=' + id + '">Pakete</a> · ' : '') + (here !== 'statistik' ? '<a href="statistik.html?c=' + id + '">Statistik</a> · ' : '') +
        (s.conn.proto === 'TCP' || s.conn.proto === 'UDP' ? '<a href="stream.html?c=' + id + '">Stream</a> · ' : '') + '<a href="' + here + '.html">Alle Pakete</a></div></div>';
    }
    if (s.cset) {
      const n = Object.keys(s.cset).length;
      return '<div class="notice info">' + I('connections') + '<div>Nur die Pakete der ' + D.fnum(n) + ' Verbindungen im Verbindungsfilter <code>' + esc(s.cf) + '</code> · <a href="verbindungen.html?f=' + encodeURIComponent(s.cf) + '">Verbindungen</a> · <a href="' + here + '.html">Alle Pakete</a></div></div>';
    }
    return '';
  }

  /* ---------- Aufzeichnung ---------- */
  function pageCapture() {
    const m = D.mode();
    if (m !== 'none') {
      const run = m === 'running';
      $('#main').innerHTML = pagehead('Aufzeichnung', 'Aufzeichnung auf ' + D.ADAPTER.name + ' seit ' + D.clock(0) + ' Uhr') +
        '<section class="card"><header><h2>' + (run ? '<span class="st ok">' + I('check-circle', 'sm') + 'Aufzeichnung läuft</span>' : '<span class="st warn">' + I('pause', 'sm') + 'Pausiert</span>') + '</h2><span class="hint" id="capDur">Dauer ' + DURNAME() + '</span></header>' +
        '<div class="cbody"><dl class="facts">' +
        '<dt>Quelle</dt><dd><b class="mono">' + D.ADAPTER.name + '</b><span class="muted">' + D.ADAPTER.desc + '</span></dd>' +
        '<dt>Link-Typ</dt><dd>Ethernet</dd>' +
        '<dt>Filter</dt><dd class="muted">kein BPF-Filter</dd>' +
        '<dt>Paketspeicher</dt><dd>vollständige Pakete, höchstens 500.000 Pakete</dd>' +
        '<dt>PCAP-Datei</dt><dd class="muted">keine</dd>' +
        '<dt>Verworfene Pakete</dt><dd><span class="mono">0</span> vom Adapter, <span class="mono">0</span> von openpacketmon (Puffer voll)</dd></dl></div>' +
        '<div class="cfoot">' + (run ? '<button class="btn" type="button" data-cap="pause">' + I('pause', 'sm') + 'Pausieren</button>' : '<button class="btn" type="button" data-cap="resume">' + I('play', 'sm') + 'Fortsetzen</button>') +
        '<button class="btn danger" type="button" data-cap="stop">' + I('stop', 'sm') + 'Beenden</button><span class="muted small">Beim Beenden werden die gesammelten Daten verworfen.</span><span class="sp"></span>' +
        '<a class="btn primary" href="uebersicht.html">Zur Übersicht' + I('arrow-right', 'sm') + '</a></div></section>';
      document.addEventListener('opm-tick', function () { const d = $('#capDur'); if (d) d.textContent = 'Dauer ' + DURNAME(); });
      return;
    }
    const sparkPts = {};
    D.ADAPTERS.forEach(function (a, i) { const r = D.rng(31 + i); sparkPts[a.name] = []; for (let k = 0; k < 30; k += 1) sparkPts[a.name].push(a.act ? a.act * (0.45 + 0.55 * r()) : 0); });
    function spark(a) {
      if (a.noip) return '<span class="muted small">Kein IP-Verkehr, wird nicht ausgewertet</span>';
      const p = sparkPts[a.name]; const flat = !a.act;
      let d = ''; p.forEach(function (v, k) { d += (k ? 'L' : 'M') + (k * 140 / 29).toFixed(1) + ' ' + (24 - v * 22).toFixed(1); });
      return '<svg class="spark" viewBox="0 0 140 26" preserveAspectRatio="none" aria-label="' + (flat ? 'kein Verkehr' : 'Verkehr') + '"><path class="' + (flat ? 'flat' : '') + '" d="' + d + '"/></svg>';
    }
    const rows = D.ADAPTERS.map(function (a, i) {
      return '<tr class="' + (a.noip ? 'off' : 'click') + (i === 0 ? ' chosen' : '') + '" data-a="' + a.name + '"><td><input type="radio" name="ad" id="ad' + i + '" value="' + a.name + '"' + (i === 0 ? ' checked' : '') + (a.noip ? ' disabled' : '') + '></td>' +
        '<td><label for="ad' + i + '"><b class="mono">' + a.name + '</b></label><span class="sub">' + a.desc + '</span></td>' +
        '<td class="addr">' + (a.addr.length ? a.addr.join('<br>') : '<span class="muted">–</span>') + '</td><td data-spark="' + a.name + '">' + spark(a) + '</td></tr>';
    }).join('');
    $('#main').innerHTML = pagehead('Aufzeichnung', 'Wähle aus, welcher Verkehr untersucht wird.') +
      '<div class="notice err" id="capErr" hidden>' + I('x-circle') + '<div>Wähle einen Netzwerkadapter aus der Liste.</div></div>' +
      '<section class="card"><header><h2>Quelle</h2><div class="seg" id="srcSeg" role="group" aria-label="Quelle">' +
      [['dev', 'adapter', 'Netzwerkadapter'], ['file', 'file', 'Aufzeichnungsdatei'], ['ipfix', 'radio', 'IPFIX-Kollektor']].map(function (s, i) { return '<button type="button" data-v="' + s[0] + '" aria-pressed="' + (i === 0) + '">' + I(s[1], 'sm') + s[2] + '</button>'; }).join('') + '</div></header>' +
      '<div class="srcpanel" id="src-dev"><p class="help" style="padding:var(--sp-3) var(--sp-4) 0;margin:0">Die Kurven zeigen die Pakete der letzten 30 Sekunden; so erkennst du den Adapter mit Verkehr.</p>' +
      '<div class="tblwrap"><table class="tbl adapters"><thead><tr><th><span class="sr-only">Auswahl</span></th><th>Adapter</th><th>Adressen</th><th>Pakete (30 s)</th></tr></thead><tbody>' + rows + '</tbody></table></div></div>' +
      '<div class="srcpanel cbody" id="src-file" hidden><div class="field"><label for="fpath">Pfad der Datei (PCAP oder PCAPNG)</label><div style="display:flex;gap:var(--sp-2)"><input id="fpath" type="text" style="flex:1" placeholder="/home/…/aufzeichnung.pcapng"><button class="btn" type="button" data-draft-toast="Entwurf: öffnet den Dateidialog">' + I('folder', 'sm') + 'Durchsuchen …</button></div><span class="help">Die Datei wird auf diesem Rechner gelesen; der Pfad muss für deinen Benutzer lesbar sein.</span></div></div>' +
      '<div class="srcpanel cbody" id="src-ipfix" hidden><div class="field-row"><div class="field"><label for="ipa">Adresse</label><input id="ipa" type="text" placeholder="alle Adressen"></div><div class="field"><label for="ipp">UDP-Port</label><input id="ipp" type="number" placeholder="4739"></div></div><p class="help">openpacketmon empfängt Flussdaten (IPFIX) von Routern oder Switches. Leer lassen für alle Adressen und Port 4739.</p></div></section>' +
      '<div class="grid2"><section class="card"><header><h2>Filter</h2></header><div class="cbody"><div class="field" style="margin:0"><label for="bpf">BPF-Filter (optional)</label><input id="bpf" type="text" class="mono" placeholder="z. B. tcp port 443"><span class="help">Nur passende Pakete werden ausgewertet, z. B. „tcp port 443“ oder „host 192.0.2.1“ (Syntax wie bei tcpdump).</span></div></div></section>' +
      '<section class="card"><header><h2>Paketspeicher</h2><div class="seg sm" id="storeSeg" role="group" aria-label="Paketspeicher"><button type="button" data-v="full" aria-pressed="true">Vollständige Pakete</button><button type="button" data-v="head" aria-pressed="false">Nur Kopfdaten</button><button type="button" data-v="off" aria-pressed="false">Aus</button></div></header>' +
      '<div class="cbody"><p class="small" id="storeLine" style="font-weight:500">vollständige Pakete, höchstens 500.000 Pakete</p><p class="help" style="margin:0">Gespeicherte Pakete zeigen die Seiten Pakete, Paketdetails und Stream folgen und lassen sich als PCAP speichern. Vollständige Pakete brauchen mehr Arbeitsspeicher (Grenze in den Einstellungen, bis 2 GiB). „Nur Kopfdaten“ behält die ersten 128 Bytes jedes Pakets; Anrufe lassen sich dann nicht anhören.</p></div></section></div>' +
      '<section class="card"><header><h2>PCAP-Datei</h2></header><div class="cbody"><label class="check"><input type="checkbox" id="exp"> Pakete zusätzlich in eine PCAP-Datei schreiben</label>' +
      '<div class="field-row" style="margin-top:var(--sp-2)"><div class="field"><label for="exd">Ordner</label><input id="exd" type="text" value="~/Dokumente" disabled></div><div class="field"><label for="exn">Dateiname</label><input id="exn" type="text" value="openpacketmon.pcap" disabled></div></div></div></section>' +
      '<div class="btngroup" style="justify-content:flex-end"><button class="btn primary" type="button" id="startBtn">' + I('play', 'sm') + 'Aufzeichnung starten</button></div>';
    bindSeg('srcSeg', function (v) { $$('.srcpanel').forEach(function (p) { p.hidden = p.id !== 'src-' + v; }); });
    bindSeg('storeSeg', function (v) { $('#storeLine').textContent = { full: 'vollständige Pakete, höchstens 500.000 Pakete', head: 'nur Kopfdaten (128 Bytes je Paket), höchstens 500.000 Pakete', off: 'keine Pakete speichern (nur Verbindungen und Statistik der Verbindungen)' }[v]; });
    $$('.adapters tr.click').forEach(function (tr) { tr.addEventListener('click', function () { $('input', tr).checked = true; $$('.adapters tr').forEach(function (o) { o.classList.toggle('chosen', o === tr); }); $('#capErr').hidden = true; }); });
    $('#exp').addEventListener('change', function () { $('#exd').disabled = $('#exn').disabled = !this.checked; });
    $('#startBtn').addEventListener('click', function () {
      if (!$('input[name="ad"]:checked') && !$('#src-dev').hidden) { $('#capErr').hidden = false; return; }
      D.setMode('running'); location.href = 'uebersicht.html';
    });
    setInterval(function () {
      D.ADAPTERS.forEach(function (a) { if (a.noip) return; const p = sparkPts[a.name]; p.shift(); p.push(a.act ? a.act * (0.45 + 0.55 * Math.random()) : 0); const td = $('[data-spark="' + a.name + '"]'); if (td) td.innerHTML = spark(a); });
    }, 1000);
  }

  /* ---------- Übersicht ---------- */
  function pageOverview() {
    if (none()) { $('#main').innerHTML = pagehead('Übersicht', '') + noCapture(); return; }
    let unit = App.store.get('unit') || 'bytes';
    const hosts = function () { return D.hostsAgg(); };
    $('#main').innerHTML = pagehead('Übersicht', 'Aufzeichnung auf <span class="mono">' + D.ADAPTER.name + '</span> seit ' + D.clock(0) + ' Uhr · <span id="ovDur">' + DURNAME() + '</span>', unitSeg('unitSeg', unit)) +
      '<div class="tiles" id="tiles"></div>' +
      '<section class="card attn"><header><h2>Auffällig</h2><span class="hint">Aus der TCP-Analyse, den Anrufen und den Favoriten, nur ein Hinweis</span></header><div id="attn"></div></section>' +
      '<section class="card"><header><h2>Verkehr pro Sekunde</h2><div class="legend"><span><i></i>Eingehend</span><span><i class="l2"></i>Ausgehend</span><span class="rate" id="rate"></span></div></header><div class="chartbox" id="ovChart"></div></section>' +
      '<div class="grid3" id="tops"></div>';
    bindSeg('unitSeg', function (u) { unit = u; App.store.set('unit', u); render(); });
    function tiles() {
      const T = D.total(); const conns = D.CONNS.filter(function (c) { return c.packets; }).length; const hs = hosts().length;
      const v = function (b, p) { const s = D.funit(unit, b, p).split(' '); return s[0] + (s[1] ? ' <small>' + s[1] + '</small>' : ''); };
      const inP = Math.round(T.packets * T.inB / Math.max(1, T.bytes)), outP = T.packets - inP;
      $('#tiles').innerHTML =
        '<div class="tile"><div class="k">' + I('activity', 'sm') + 'Daten gesamt</div><div class="v">' + v(T.bytes, T.packets) + '</div><div class="s">' + D.fnum(T.packets) + ' Pakete</div></div>' +
        '<div class="tile"><div class="k">' + I('arrow-down', 'sm i1') + 'Eingehend</div><div class="v">' + v(T.inB, inP) + '</div><div class="s">' + Math.round(T.inB / Math.max(1, T.bytes) * 100) + ' % der Daten</div></div>' +
        '<div class="tile"><div class="k">' + I('arrow-up', 'sm i2') + 'Ausgehend</div><div class="v">' + v(T.outB, outP) + '</div><div class="s">' + Math.round(T.outB / Math.max(1, T.bytes) * 100) + ' % der Daten</div></div>' +
        '<a class="tile" href="verbindungen.html"><div class="k">' + I('connections', 'sm') + 'Verbindungen</div><div class="v">' + D.fnum(conns) + '</div><div class="s">TCP, UDP, ICMP, ARP</div></a>' +
        '<a class="tile" href="hosts.html"><div class="k">' + I('hosts', 'sm') + 'Hosts</div><div class="v">' + hs + '</div><div class="s">davon 4 Favoriten</div></a>' +
        '<div class="tile"><div class="k">' + I('alert', 'sm') + 'Verworfene Pakete</div><div class="v">0</div><div class="s">Adapter und openpacketmon</div></div>';
    }
    function attn() {
      const A = D.ANALYSIS; const all = A['Retransmission'] + A['Dup ACK'] + A['Out-Of-Order'] + A['RST'];
      $('#attn').innerHTML =
        '<div class="arow warn">' + I('alert') + '<div><b>' + D.fnum(all) + ' Pakete mit Befund der TCP-Analyse</b><div class="msg">' + A['Retransmission'] + ' Wiederholungen, ' + A['Dup ACK'] + ' doppelte ACKs, ' + A['Out-Of-Order'] + ' außer der Reihe, ' + A['RST'] + ' Abbrüche – meist zu video.example.net und www.example.com</div></div><a class="btn sm" href="statistik.html?f=tcp.analysis.flags">Statistik</a></div>' +
        D.calls().filter(function (k) { return k.rate === 'bad'; }).map(function (k) {
          const sd = k.i.rate === 'bad' ? k.i : k.o;
          return '<div class="arow warn">' + I('phone') + '<div><b>Anruf mit schlechter Verbindung</b><div class="msg">' + esc(callLabel(k.call)) + ', ' + D.clock(k.t0).slice(0, 5) + ' Uhr: ' + D.fnum(sd.loss, 1) + ' % Verlust ' + (sd === k.i ? 'beim Empfang' : 'beim Senden') + (sd.jit ? ', Jitter ' + D.fnum(sd.jit[0], 1) + ' ms' : '') + '</div></div><a class="btn sm" href="anruf.html?id=' + k.id + '">Anruf</a></div>';
        }).join('') +
        '<div class="arow">' + I('star') + '<div><b>Favoriten tauschen Daten aus</b><div class="msg">nas.local, mail.example.com, 203.0.113.44 und das Programm thunderbird</div></div><a class="btn sm" href="benachrichtigungen.html">Benachrichtigungen</a></div>' +
        '<div class="arow">' + I('info') + '<div><b>Unverschlüsseltes Web zu updates.example.net</b><div class="msg">HTTP auf Port 80 vom Programm http (apt); Inhalte sind lesbar</div></div><a class="btn sm" href="pakete.html?f=http">Pakete</a></div>';
    }
    function chart() {
      const now = Math.floor(D.now()); const from = Math.max(0, now - 299);
      const pts = []; let sa = 0, sb = 0;
      for (let s = from; s <= now; s += 1) {
        const e = D.SEC[s] || [0, 0, 0]; const tot = e[0] + e[1] || 1;
        const a = unit === 'packets' ? Math.round(e[2] * e[0] / tot) : e[0], b = unit === 'packets' ? e[2] - Math.round(e[2] * e[0] / tot) : e[1];
        pts.push({ s: s, a: a, b: b });
      }
      const last = pts.slice(-4, -1); last.forEach(function (p) { sa += p.a; sb += p.b; }); sa /= Math.max(1, last.length); sb /= Math.max(1, last.length);
      $('#rate').innerHTML = 'jetzt <span class="i1">↓</span> <b>' + rateFmt(unit, sa) + '</b> <span class="i2">↑</span> <b>' + rateFmt(unit, sb) + '</b>';
      drawChart($('#ovChart'), pts, { label: 'Verkehr pro Sekunde, letzte 5 Minuten', yfmt: function (v) { return rateFmt(unit, v); }, xfmt: function (p) { return D.clock(p.s).slice(0, 5) === D.clock(now).slice(0, 5) && p.s === now ? 'jetzt' : D.clock(p.s); }, tipx: function (p) { return D.clock(p.s) + ' Uhr'; }, ticks: 5 });
    }
    function tops() {
      const val = function (e) { return unit === 'packets' ? e.packets : e.bytes; };
      const fmt = function (e) { return D.funit(unit, e.bytes, e.packets); };
      function list(title, icon, rows, more, nameFn, linkFn, starFn) {
        rows = rows.sort(function (a, b) { return val(b) - val(a); }).slice(0, 7);
        const max = rows.length ? val(rows[0]) : 1;
        return '<section class="card top"><header><h2>' + I(icon) + title + '</h2><a class="small" href="' + more + '">Alle anzeigen</a></header>' +
          rows.map(function (e) {
            return '<div class="trow">' + (starFn ? starFn(e) : '<span></span>') + '<div class="nm"><a href="' + linkFn(e) + '" title="' + esc(nameFn(e)[0]) + '">' + esc(nameFn(e)[0]) + '</a><span class="sub">' + esc(nameFn(e)[1]) + '</span></div><div class="val">' + fmt(e) + '</div>' +
              App.tbar(unit === 'packets' ? e.packets * e.inB / Math.max(1, e.bytes) : e.inB, unit === 'packets' ? e.packets * e.outB / Math.max(1, e.bytes) : e.outB, max) + '</div>';
          }).join('') + '</section>';
      }
      $('#tops').innerHTML =
        list('Top-Hosts', 'hosts', hosts(), 'hosts.html', function (e) { return [e.host.name, e.host.cc || (e.host.tags || [''])[0]]; }, function (e) { return 'verbindungen.html?f=' + encodeURIComponent('domain == ' + e.host.name); }, function (e) { return App.star(e.host.fav, e.host.name); }) +
        list('Top-Dienste', 'services', D.servicesAgg(), 'dienste.html', function (e) { return [e.name, e.port + '/' + e.tp]; }, function (e) { return 'verbindungen.html?f=' + encodeURIComponent('service == ' + e.name); }) +
        list('Top-Programme', 'programs', D.programsAgg(), 'programme.html', function (e) { return [e.key, '']; }, function (e) { return 'verbindungen.html?f=' + encodeURIComponent('program == "' + e.key + '"'); }, function (e) { return App.star(D.FAV_PROG.indexOf(e.key) >= 0, e.key); });
    }
    function render() { tiles(); chart(); tops(); }
    attn(); render();
    let last = 0;
    document.addEventListener('opm-tick', function () {
      const t = Date.now(); if (t - last < 1000) return; last = t;
      tiles(); chart(); $('#ovDur').textContent = DURNAME();
      if (t % 3 < 1) tops();
    });
    let rt = null; window.addEventListener('resize', function () { clearTimeout(rt); rt = setTimeout(chart, 120); });
    document.addEventListener('click', function (e) { const s = e.target.closest('.star'); if (s) { const on = s.getAttribute('aria-pressed') !== 'true'; s.setAttribute('aria-pressed', String(on)); App.toast(on ? 'Zu den Favoriten hinzugefügt (Entwurf)' : 'Aus den Favoriten entfernt (Entwurf)'); } });
  }

  /* ---------- Verbindungen ---------- */
  function pageConnections() {
    if (none()) { $('#main').innerHTML = pagehead('Verbindungen', '') + noCapture(); return; }
    let f = q.get('f') || '', page = +(q.get('page') || 1), sort = App.store.get('csort') || 'desc', unit = App.store.get('unit') || 'bytes', onlyFav = false;
    const PS = +(App.store.get('ps') || 30);
    $('#main').innerHTML = pagehead('Verbindungen', 'Alle Verbindungen der Aufzeichnung mit Filter und Seiten.',
      '<a class="btn" id="toPk" href="#">' + I('packets', 'sm') + 'Pakete</a><a class="btn" id="toSt" href="#">' + I('statistics', 'sm') + 'Statistik</a>') +
      '<section class="card"><div class="toolbar">' + filterUI('cf', 'c', f, 'Verbindungsfilter, z. B. ip.addr == 192.0.2.40 oder port == 443 oder domain contains example') + '</div>' + filterExtras('cf', 'c') +
      '<div class="toolbar"><label class="sr-only" for="csort">Sortierung</label><select id="csort"><option value="desc">Größte zuerst</option><option value="asc">Kleinste zuerst</option><option value="recent">Neueste zuerst</option></select>' +
      unitSeg('cunit', unit) + '<label class="switch"><input type="checkbox" id="cfav"> Nur Favoriten</label><span class="sp"></span><span class="sum" id="csum"></span></div>' +
      '<div class="tblwrap"><table class="tbl" id="ctbl"><thead><tr><th class="w0"><span class="sr-only">Favorit</span></th><th>Quelle</th><th>Ziel</th><th>Protokoll</th><th>Dienst</th><th class="hide-md">Programm</th><th>Gegenstelle</th><th class="num">Pakete</th><th class="num">Daten</th></tr></thead><tbody></tbody></table></div>' +
      '<div class="pager" id="cpager"></div></section>';
    $('#csort').value = sort;
    bindFilter('cf', 'c', function (v) { f = v; page = 1; setParam('f', v); setParam('page', null); render(); });
    $('#csort').addEventListener('change', function () { sort = this.value; App.store.set('csort', sort); render(); });
    bindSeg('cunit', function (u) { unit = u; App.store.set('unit', u); render(); });
    $('#cfav').addEventListener('change', function () { onlyFav = this.checked; page = 1; render(); });
    function render() {
      const r = D.connFilter(f); const flt = r.err ? null : r;
      let list = D.CONNS.filter(function (c) { return c.packets && (!flt || flt.test(c)) && (!onlyFav || D.isFav(c)); });
      const key = function (c) { return unit === 'packets' ? c.packets : c.bytes; };
      list.sort(sort === 'recent' ? function (a, b) { return b.last - a.last; } : sort === 'asc' ? function (a, b) { return key(a) - key(b); } : function (a, b) { return key(b) - key(a); });
      const total = list.length, pages = Math.max(1, Math.ceil(total / PS)); page = Math.min(Math.max(1, page), pages);
      const max = list.reduce(function (m, c) { return Math.max(m, key(c)); }, 1);
      const sumB = list.reduce(function (s, c) { return s + c.bytes; }, 0);
      const rows = list.slice((page - 1) * PS, page * PS).map(function (c) {
        const h = c.host;
        return '<tr class="click" data-id="' + c.id + '"><td class="w0">' + App.star(D.isFav(c), h.name) + '</td>' +
          '<td class="mono">' + esc(addr(c.src, c.sport)) + '</td><td class="mono">' + esc(addr(c.dst, c.dport)) + '</td><td>' + c.proto + '</td><td>' + esc(svcLabel(c)) + '</td>' +
          '<td class="hide-md">' + esc(c.prog || 'unbekannt') + '</td>' +
          '<td class="ell"><a href="verbindung.html?id=' + c.id + '">' + esc(h.name) + '</a> ' + App.flag(h.cc) + '<span class="sub">' + esc(h.asn ? 'AS' + h.asn + ' ' + h.as : (h.tags || []).join(', ')) + '</span></td>' +
          '<td class="num">' + D.fnum(c.packets) + '</td><td class="num" style="min-width:150px">' + D.funit(unit, c.bytes, c.packets) + App.tbar(unit === 'packets' ? c.packets * c.inBytes / c.bytes : c.inBytes, unit === 'packets' ? c.packets * c.outBytes / c.bytes : c.outBytes, max) + '</td></tr>';
      }).join('');
      $('#ctbl tbody').innerHTML = rows || '<tr><td colspan="9"><div class="empty"><h2>Keine Verbindung passt zum Filter.</h2></div></td></tr>';
      $('#csum').textContent = total ? 'Verbindungen ' + ((page - 1) * PS + 1) + '–' + Math.min(total, page * PS) + ' von ' + D.fnum(total) + ' (' + D.fbytes(sumB) + ')' : '0 Verbindungen';
      let pg = '<span>Seite</span><div class="pages">';
      const show = []; for (let i = 1; i <= pages; i += 1) if (i <= 2 || i > pages - 2 || Math.abs(i - page) <= 1) show.push(i);
      show.forEach(function (i, k) { if (k && i - show[k - 1] > 1) pg += '<span>…</span>'; pg += i === page ? '<span aria-current="page">' + i + '</span>' : '<a href="#" data-pg="' + i + '">' + i + '</a>'; });
      pg += '</div><span class="sp"></span><span>' + PS + ' pro Seite</span>';
      $('#cpager').innerHTML = pg;
      $('#toPk').href = url('pakete.html', { cf: f }); $('#toSt').href = url('statistik.html', { cf: f });
    }
    $('#cpager').addEventListener('click', function (e) { const a = e.target.closest('[data-pg]'); if (a) { e.preventDefault(); page = +a.dataset.pg; setParam('page', page); render(); } });
    $('#ctbl').addEventListener('click', function (e) {
      const s = e.target.closest('.star'); if (s) { const on = s.getAttribute('aria-pressed') !== 'true'; s.setAttribute('aria-pressed', String(on)); App.toast((on ? 'Zu den Favoriten hinzugefügt' : 'Aus den Favoriten entfernt') + ' (Entwurf)'); return; }
      if (e.target.closest('a')) return;
      const tr = e.target.closest('tr[data-id]'); if (tr) location.href = 'verbindung.html?id=' + tr.dataset.id;
    });
    render();
  }

  /* ---------- Verbindung ---------- */
  function pageConnection() {
    if (none()) { $('#main').innerHTML = pagehead('Verbindung', '', '', { back: 'verbindungen.html' }) + noCapture(); return; }
    const c = D.CBY[q.get('id')] || D.ST.http;
    const h = c.host; const out = c.dir === 'out';
    const remoteIp = out ? c.dst : c.src;
    const mac = function (local) { return local ? '00:00:5e:00:53:14' : (h.mac || '00:00:5e:00:53:01'); };
    const stream = c.proto === 'TCP' || c.proto === 'UDP';
    const ptxt = c.prog ? '<b>' + esc(c.prog) + '</b><span class="muted mono small">' + esc(D.PROGS[c.prog] || '') + '</span>' + App.star(D.FAV_PROG.indexOf(c.prog) >= 0, c.prog) : '<span class="muted">unbekannt</span>';
    $('#main').innerHTML = pagehead('<span class="tag">' + c.proto + '</span><span class="mono">' + esc(addr(c.src, c.sport)) + ' → ' + esc(addr(c.dst, c.dport)) + '</span>', '',
      '<a class="btn" href="pakete.html?c=' + c.id + '">' + I('packets', 'sm') + 'Pakete</a><a class="btn" href="statistik.html?c=' + c.id + '">' + I('statistics', 'sm') + 'Statistik</a>' + (stream ? '<a class="btn" href="stream.html?c=' + c.id + '">' + I('stream', 'sm') + 'Stream folgen</a>' : ''),
      { back: 'verbindungen.html', crumb: '<a href="verbindungen.html">Verbindungen</a> <span>›</span> <span>Verbindung</span>' }) +
      '<div class="split2"><section class="card"><header><h2>Verbindung</h2></header><div class="cbody tight"><dl class="facts">' +
      '<dt>Quelle</dt><dd class="mono">' + esc(addr(c.src, c.sport)) + '</dd><dt>MAC der Quelle</dt><dd class="mono">' + mac(out) + '</dd>' +
      '<dt>Ziel</dt><dd class="mono">' + esc(addr(c.dst, c.dport)) + '</dd><dt>MAC des Ziels</dt><dd class="mono">' + (c.dst.indexOf('224.') === 0 ? '01:00:5e:00:00:fb' : c.dst.indexOf('239.') === 0 ? '01:00:5e:7f:ff:fa' : mac(!out)) + '</dd>' +
      '<dt>Protokoll</dt><dd>' + c.proto + (c.host.v6 ? ' über IPv6' : c.proto === 'ARP' ? '' : ' über IPv4') + '</dd><dt>Richtung</dt><dd>' + (out ? I('arrow-up', 'sm') + ' ausgehend' : I('arrow-down', 'sm') + ' eingehend') + '</dd>' +
      '<dt>Dienst</dt><dd>' + (c.svc ? '<b>' + c.svc + '</b><span class="muted">' + c.dport + '/' + c.proto.toLowerCase() + ' · ' + ({ https: 'Web', http: 'Web', imaps: 'E-Mail', domain: 'Namensauflösung', ssh: 'Fernzugriff', 'microsoft-ds': 'Dateifreigaben' }[c.svc] || 'Sonstige') + '</span>' : '<span class="muted">–</span>') + '</dd>' +
      '<dt>Programm</dt><dd>' + ptxt + '</dd>' +
      '<dt>Pakete</dt><dd><span class="mono">' + D.fnum(c.packets) + '</span><span class="links">· <a href="pakete.html?c=' + c.id + '">anzeigen</a> · <a href="statistik.html?c=' + c.id + '">Statistik</a>' + (stream ? ' · <a href="stream.html?c=' + c.id + '">Stream</a>' : '') + '</span></dd>' +
      '<dt>Daten</dt><dd><span class="mono">' + D.fbytes(c.bytes) + '</span><span class="muted small">↓ ' + D.fbytes(c.inBytes) + ' · ↑ ' + D.fbytes(c.outBytes) + '</span><span style="flex:1;min-width:80px;max-width:200px">' + App.tbar(c.inBytes, c.outBytes) + '</span></dd>' +
      '<dt>Erstes Paket</dt><dd class="mono">' + D.clock(c.first || 0, true).slice(0, 12) + '</dd><dt>Letztes Paket</dt><dd class="mono">' + D.clock(c.last || 0, true).slice(0, 12) + '</dd>' +
      '<dt>Latenz</dt><dd id="lat"><button class="btn sm" type="button" id="latBtn">' + I('clock', 'sm') + 'messen</button></dd></dl></div></section>' +
      '<section class="card"><header><h2>Gegenstelle</h2>' + App.star(D.FAV_ADDR.indexOf(remoteIp) >= 0 || h.fav, remoteIp) + '</header><div class="cbody tight"><dl class="facts">' +
      '<dt>Adresse</dt><dd class="mono">' + esc(remoteIp) + '</dd><dt>Reverse DNS</dt><dd>' + esc(h.name !== h.ip ? h.name : '–') + '</dd>' +
      '<dt>Autonomes System</dt><dd>' + (h.asn ? '<span class="mono">AS' + h.asn + '</span> ' + esc(h.as) : '<span class="muted">' + esc((h.tags || ['–']).join(', ')) + '</span>') + '</dd>' +
      '<dt>Land</dt><dd>' + (h.cc ? App.flag(h.cc) + ' ' + ({ DE: 'Deutschland', NL: 'Niederlande', US: 'Vereinigte Staaten', IE: 'Irland' }[h.cc] || '') : '<span class="muted">–</span>') + '</dd></dl></div>' +
      '<div class="cfoot" style="justify-content:flex-start"><a class="btn sm" href="verbindungen.html?f=' + encodeURIComponent('domain == ' + h.name) + '">Verbindungen des Hosts</a><a class="btn sm" href="verbindungen.html?f=' + encodeURIComponent('ip.addr == ' + remoteIp) + '">Verbindungen der Adresse</a></div></section></div>';
    $('#latBtn').addEventListener('click', function () {
      $('#lat').innerHTML = '<span class="muted">wird gemessen …</span>';
      setTimeout(function () { $('#lat').innerHTML = '<span class="mono">' + (h.tags ? '0,6' : (12 + D.hash(c.packets, 3) * 30).toFixed(1).replace('.', ',')) + ' ms</span><button class="btn sm ghost" type="button" onclick="this.parentNode.innerHTML=\'\';document.getElementById(\'lat\').append(\'…\');location.reload()">neu messen</button>'; }, 900);
    });
    document.addEventListener('click', function (e) { const s = e.target.closest('.star'); if (s) { const on = s.getAttribute('aria-pressed') !== 'true'; s.setAttribute('aria-pressed', String(on)); App.toast((on ? 'Zu den Favoriten hinzugefügt' : 'Aus den Favoriten entfernt') + ' (Entwurf)'); } });
  }

  /* ---------- Pakete ---------- */
  function pagePackets() {
    if (none()) { $('#main').innerHTML = pagehead('Pakete', '') + noCapture(); return; }
    $('#main').classList.add('fill');
    const S = selection();
    let pf = S.f ? D.packetFilter(S.f) : null; if (pf && pf.err) pf = null;
    let list = null; // null = alle Pakete 1..N
    let selN = +(q.get('n') || 0), follow = !selN && D.mode() === 'running', newFrom = Infinity;
    $('#main').innerHTML = '<div class="page">' + pagehead('Pakete', 'Die gespeicherten Pakete der Aufzeichnung mit Schichten und Bytes.',
      '<a class="btn" id="toSt" href="#">' + I('statistics', 'sm') + 'Statistik</a><button class="btn" type="button" id="pcap" data-draft-toast="Entwurf: speichert die Pakete als PCAP-Datei (Dateidialog)">' + I('download', 'sm') + '<span id="pcapLbl">Als PCAP speichern</span></button>') +
      connNote(S, 'pakete') +
      '<section class="card pktcard"><div class="toolbar">' + filterUI('pf', 'p', S.f, 'Anzeigefilter, z. B. dns oder tls.sni contains example oder ip.addr == 192.0.2.20') +
      '<span class="goto"><label for="goto" class="small muted hide-sm">Gehe zu Paket</label><input id="goto" type="text" inputmode="numeric" placeholder="Nr." title="Paketnummer eingeben und Enter drücken (Strg+G)"></span></div>' +
      filterExtras('pf', 'p') +
      '<div class="selbar" id="selbar" hidden></div>' +
      '<div class="pktscroll" id="pscroll" tabindex="0" aria-label="Paketliste"><table class="pkt"><colgroup><col style="width:72px"><col style="width:96px"><col style="width:20%"><col style="width:20%"><col style="width:84px"><col style="width:64px"><col></colgroup>' +
      '<thead><tr><th class="r">Nr.</th><th class="r">Zeit (s)</th><th>Quelle</th><th>Ziel</th><th>Protokoll</th><th class="r">Länge</th><th>Info</th></tr></thead><tbody id="pbody"></tbody></table></div>' +
      '<div class="followbar"><span><span class="num" id="pkKept"></span> Pakete gespeichert · <span class="num">0</span> ältere verworfen</span><span id="pkHits"></span><span class="sp"></span>' +
      '<span class="hide-md" id="pkFollow"></span></div></section></div>';
    const sc = $('#pscroll'), body = $('#pbody');
    let ROW = parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--pkt-h')) || 26;
    function len() { return list ? list.length : D.count(); }
    function at(i) { return list ? list[i] : i + 1; }
    function indexOf(n) {
      if (!list) return n >= 1 && n <= D.count() ? n - 1 : -1;
      let lo = 0, hi = list.length - 1;
      while (lo <= hi) { const m = (lo + hi) >> 1; if (list[m] === n) return m; if (list[m] < n) lo = m + 1; else hi = m - 1; }
      return -1;
    }
    function build() {
      if (!pf && !S.conn && !S.cset) { list = null; return; }
      list = []; const N = D.count();
      for (let n = 1; n <= N; n += 1) if (S.test(D.pkt(n), pf)) list.push(n);
    }
    function row(n) {
      const p = D.pkt(n);
      return '<tr class="' + p.cls + (n === selN ? ' sel' : '') + (n >= newFrom ? ' new' : '') + '" data-n="' + n + '"><td class="r">' + n + '</td><td class="r">' + D.ftime(p.t) + '</td><td>' + esc(p.src) + '</td><td>' + esc(p.dst) + '</td><td>' + esc(p.proto) + '</td><td class="r">' + p.len + '</td><td>' + esc(p.info) + '</td></tr>';
    }
    function render() {
      const L = len(); const top = sc.scrollTop, h = sc.clientHeight || 400;
      const vis = Math.ceil(h / ROW) + 24;
      const first = Math.max(0, Math.min(Math.floor(top / ROW) - 12, L - vis)), last = Math.min(L, first + vis);
      let s = '<tr aria-hidden="true"><td colspan="7" style="height:' + (first * ROW) + 'px;padding:0;border:0"></td></tr>';
      for (let i = first; i < last; i += 1) s += row(at(i));
      s += '<tr aria-hidden="true"><td colspan="7" style="height:' + ((L - last) * ROW) + 'px;padding:0;border:0"></td></tr>';
      body.innerHTML = s;
      const r = body.querySelector('tr[data-n]'); if (r && r.offsetHeight && Math.abs(r.offsetHeight - ROW) > 0.5) { ROW = r.offsetHeight; render(); return; }
      $('#pkKept').textContent = D.fnum(D.count());
      $('#pkHits').textContent = list ? '· ' + D.fnum(L) + ' passen zur Auswahl' : '';
      $('#pkFollow').innerHTML = D.mode() === 'running' ? (follow ? '<span class="st ok">' + I('activity', 'sm') + 'Live, folgt den neuen Paketen</span>' : 'Am Ende der Liste folgt sie den neuen Paketen; hochscrollen hält sie an.') : '<span class="st warn">' + I('pause', 'sm') + 'Aufzeichnung pausiert</span>';
      $('#toSt').href = url('statistik.html', { f: S.f, c: S.c, cf: S.cf });
      $('#pcapLbl').textContent = list ? 'Auswahl als PCAP speichern' : 'Als PCAP speichern';
    }
    function toBottom() { sc.scrollTop = sc.scrollHeight; }
    function scrollToN(n) {
      const i = indexOf(n); if (i < 0) return false;
      const y = i * ROW; if (y < sc.scrollTop || y > sc.scrollTop + sc.clientHeight - ROW * 3) sc.scrollTop = Math.max(0, y - sc.clientHeight / 3);
      return true;
    }
    function select(n, scroll) {
      selN = n; follow = false; setParam('n', n || null);
      const p = D.pkt(n); const c = D.CBY[p.conn];
      const sb = $('#selbar'); sb.hidden = false;
      sb.innerHTML = '<b>Paket ' + n + '</b><span class="mono small">' + esc(p.proto) + ' · ' + esc(p.info.length > 70 ? p.info.slice(0, 70) + '…' : p.info) + '</span><span class="sp"></span>' +
        '<a class="btn sm primary" href="' + url('paket.html', { n: n, f: S.f, c: S.c, cf: S.cf }) + '">Details öffnen <kbd>Enter</kbd></a>' +
        '<a class="btn sm" href="verbindung.html?id=' + c.id + '">Verbindung</a>' + (c.proto === 'TCP' || c.proto === 'UDP' ? '<a class="btn sm" href="stream.html?c=' + c.id + '">Stream</a>' : '') +
        (function () { const v = p.sip || p.rtp || p.rtcp, k = v && D.CALL[v.call]; return k && !k.reg ? '<a class="btn sm" href="anruf.html?id=' + v.call + '">' + I('phone', 'sm') + 'Anruf</a>' : ''; })();
      if (scroll) scrollToN(n);
      render();
    }
    build(); render();
    if (selN) { requestAnimationFrame(function () { scrollToN(selN); select(selN); }); }
    else requestAnimationFrame(function () { toBottom(); render(); }); // wie Wireshark: die neuesten Pakete unten, auch pausiert
    sc.addEventListener('scroll', function () {
      const atEnd = sc.scrollTop + sc.clientHeight >= sc.scrollHeight - ROW * 1.5;
      if (D.mode() === 'running' && atEnd !== follow && !selN) follow = atEnd;
      else if (atEnd && D.mode() === 'running' && selN && sc.dataset.user) follow = true;
      render();
    }, { passive: true });
    sc.addEventListener('wheel', function () { sc.dataset.user = '1'; }, { passive: true });
    body.addEventListener('click', function (e) { const tr = e.target.closest('tr[data-n]'); if (tr) select(+tr.dataset.n); });
    body.addEventListener('dblclick', function (e) { const tr = e.target.closest('tr[data-n]'); if (tr) location.href = url('paket.html', { n: tr.dataset.n, f: S.f, c: S.c, cf: S.cf }); });
    document.addEventListener('opm-tick', function (e) {
      newFrom = e.detail.from;
      if (list) for (let n = e.detail.from; n <= e.detail.to; n += 1) if (S.test(D.pkt(n), pf)) list.push(n);
      if (follow) { render(); toBottom(); }
      render();
    });
    bindFilter('pf', 'p', function (v) {
      setParam('f', v); S.f = v; pf = v ? D.packetFilter(v) : null; if (pf && pf.err) pf = null;
      build(); sc.scrollTop = 0; follow = false; render();
      if (selN && indexOf(selN) >= 0) scrollToN(selN);
    });
    const go = $('#goto');
    go.addEventListener('keydown', function (e) {
      if (e.key !== 'Enter') return;
      e.preventDefault(); e.stopPropagation(); // sonst öffnet das Enter danach gleich die Details
      const n = parseInt(go.value, 10);
      if (!(n >= 1 && n <= D.count())) { App.toast('Paket ' + (go.value || '?') + ' gibt es nicht (1 bis ' + D.fnum(D.count()) + ').'); return; }
      if (indexOf(n) < 0) { App.toast('Paket ' + n + ' passt nicht zur Auswahl.'); return; }
      select(n, true); sc.focus();
    });
    document.addEventListener('keydown', function (e) {
      if ((e.ctrlKey || e.metaKey) && (e.key === 'g' || e.key === 'G')) { e.preventDefault(); go.focus(); go.select(); return; }
      if (document.activeElement && (document.activeElement.tagName === 'INPUT' || document.activeElement.tagName === 'TEXTAREA') || $('.modal.open')) return;
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const i = selN ? indexOf(selN) : (e.key === 'ArrowDown' ? -1 : len());
        const j = Math.max(0, Math.min(len() - 1, i + (e.key === 'ArrowDown' ? 1 : -1)));
        select(at(j), true);
      } else if (e.key === 'Enter' && selN) { location.href = url('paket.html', { n: selN, f: S.f, c: S.c, cf: S.cf }); }
      else if (e.key === 'End') { e.preventDefault(); selN = 0; setParam('n', null); $('#selbar').hidden = true; follow = D.mode() === 'running'; toBottom(); render(); }
      else if (e.key === 'Home') { e.preventDefault(); follow = false; sc.scrollTop = 0; render(); }
    });
    if (q.get('focus')) $('#pf').focus();
    let rt = null; window.addEventListener('resize', function () { clearTimeout(rt); rt = setTimeout(render, 80); });
  }

  /* ---------- Paket ---------- */
  function pagePacket() {
    if (none()) { $('#main').innerHTML = pagehead('Paket', '', '', { back: 'pakete.html' }) + noCapture(); return; }
    const S = selection();
    let pf = S.f ? D.packetFilter(S.f) : null; if (pf && pf.err) pf = null;
    const n = Math.min(Math.max(1, +(q.get('n') || 1205)), D.count());
    const d = D.detail(n); const p = d.p; const c = d.c;
    function step(dir) { for (let k = n + dir; k >= 1 && k <= D.count(); k += dir) if (S.test(D.pkt(k), pf)) return k; return 0; }
    const prev = step(-1), next = step(1);
    const keep = { f: S.f, c: S.c, cf: S.cf };
    const open = d.layers.length >= 4 ? [d.layers.length - 2, d.layers.length - 1] : [d.layers.length - 1];
    const layers = d.layers.map(function (l, i) {
      return '<li data-l="' + i + '"><details' + (open.indexOf(i) >= 0 ? ' open' : '') + '><summary data-r="' + l.range.join(',') + '">' + I('chev-down', 'sm chev') + esc(l.title) + '</summary>' +
        l.fields.map(function (fd) { return '<div class="f' + (fd.warn ? ' warnf' : '') + '"' + (fd.range ? ' data-r="' + fd.range.join(',') + '"' : '') + '><span class="k">' + esc(fd.k) + '</span><span class="v">' + esc(fd.v) + '</span></div>'; }).join('') +
        (l.rels || []).map(function (r) { return '<div class="rel">' + esc(r[0]) + ' ' + (r[1] ? '<a href="' + url('paket.html', Object.assign({ n: r[1] }, keep)) + '">' + r[1] + '</a>' : '') + (r[2] ? ' <span class="muted">(nach ' + esc(r[2]) + ')</span>' : '') + '</div>'; }).join('') +
        '</details></li>';
    }).join('');
    function hex() {
      const b = d.bytes; let s = '';
      for (let o = 0; o < b.length; o += 16) {
        let hx = '', as = '';
        for (let k = 0; k < 16; k += 1) {
          const i = o + k;
          if (i < b.length) { hx += '<b data-i="' + i + '">' + b[i].toString(16).padStart(2, '0') + '</b>' + (k === 7 ? '  ' : ' '); const ch = b[i]; as += '<b data-i="' + i + '"' + (ch >= 32 && ch < 127 ? '' : ' class="dotc"') + '>' + (ch >= 32 && ch < 127 ? esc(String.fromCharCode(ch)) : '.') + '</b>'; }
          else hx += '   ' + (k === 7 ? ' ' : '');
        }
        s += '<span class="o">' + o.toString(16).padStart(4, '0') + '</span>  ' + hx + ' <span class="a">' + as + '</span>\n';
      }
      return s;
    }
    const stream = c.proto === 'TCP' || c.proto === 'UDP';
    $('#main').innerHTML = pagehead('Paket ' + n + ' <span class="tag">' + esc(p.proto) + '</span>', '<span class="mono">' + esc(p.info) + '</span>',
      '<div class="seg" role="group" aria-label="Blättern">' + (prev ? '<a href="' + url('paket.html', Object.assign({ n: prev }, keep)) + '">' + I('chev-left', 'sm') + 'Vorheriges</a>' : '<button type="button" disabled>' + I('chev-left', 'sm') + 'Vorheriges</button>') +
      (next ? '<a href="' + url('paket.html', Object.assign({ n: next }, keep)) + '">Nächstes' + I('chev-right', 'sm') + '</a>' : '<button type="button" disabled>Nächstes' + I('chev-right', 'sm') + '</button>') + '</div>',
      { back: url('pakete.html', Object.assign({ n: n }, keep)), crumb: '<a href="' + url('pakete.html', Object.assign({ n: n }, keep)) + '">Pakete</a> <span>›</span> <span>Paket ' + n + '</span>' }) +
      '<div class="notice info">' + I('connections') + '<div>Verbindung: <a class="mono" href="verbindung.html?id=' + c.id + '">' + esc(connLabel(c)) + '</a> · <a href="pakete.html?c=' + c.id + '">Pakete</a> · <a href="statistik.html?c=' + c.id + '">Statistik</a>' + (stream ? ' · <a href="stream.html?c=' + c.id + '">Stream</a>' : '') + ' · <a href="verbindung.html?id=' + c.id + '">Details</a>' +
        (d.call ? '<br>' + I('phone', 'sm') + ' Gehört zum Anruf <a href="anruf.html?id=' + d.call.id + '">' + esc(callLabel(d.call)) + '</a>' : '') + '</div></div>' +
      '<div class="split2"><section class="card"><header><h2>Schichten</h2><span class="hint">Klick auf ein Feld markiert seine Bytes</span></header><ul class="layers" id="layers">' + layers + '</ul></section>' +
      '<section class="card"><header><h2>Bytes</h2><span class="hint">' + d.bytes.length + ' Bytes</span><button class="btn sm ghost" type="button" data-draft-toast="Entwurf: kopiert die Bytes als Hex">' + I('copy', 'sm') + 'Kopieren</button></header><pre class="hex" id="hex">' + hex() + '</pre></section></div>';
    function mark(r, el) {
      $$('#hex b.on').forEach(function (b) { b.classList.remove('on'); });
      $$('#layers .f.on').forEach(function (f) { f.classList.remove('on'); });
      $$('#layers li.on').forEach(function (l) { l.classList.remove('on'); });
      if (!r) return;
      const a = r.split(',').map(Number);
      $$('#hex b').forEach(function (b) { const i = +b.dataset.i; if (i >= a[0] && i < a[1]) b.classList.add('on'); });
      if (el) { if (el.classList.contains('f')) el.classList.add('on'); else el.closest('li').classList.add('on'); }
    }
    $('#layers').addEventListener('click', function (e) {
      const f = e.target.closest('.f[data-r]'); if (f) { mark(f.dataset.r, f); return; }
      const s = e.target.closest('summary'); if (s) mark(s.dataset.r, s);
    });
    $('#hex').addEventListener('click', function (e) {
      const b = e.target.closest('b[data-i]'); if (!b) return; const i = +b.dataset.i;
      let best = null, bw = 1e9;
      $$('#layers .f[data-r]').forEach(function (f) { const a = f.dataset.r.split(',').map(Number); if (i >= a[0] && i < a[1] && a[1] - a[0] < bw) { best = f; bw = a[1] - a[0]; } });
      if (best) { best.closest('details').open = true; mark(best.dataset.r, best); best.scrollIntoView({ block: 'nearest' }); }
    });
    document.addEventListener('keydown', function (e) {
      if (document.activeElement && document.activeElement.tagName === 'INPUT') return;
      if (e.key === 'ArrowLeft' && !e.altKey && prev) location.href = url('paket.html', Object.assign({ n: prev }, keep));
      if (e.key === 'ArrowRight' && next) location.href = url('paket.html', Object.assign({ n: next }, keep));
    });
  }

  /* ---------- Stream folgen ---------- */
  function pageStream() {
    if (none()) { $('#main').innerHTML = pagehead('Stream folgen', '', '', { back: 'pakete.html' }) + noCapture(); return; }
    const c = D.CBY[q.get('c')] || D.ST.http;
    let side = 'both', mode = 'text';
    // Nutzdaten der Pakete dieser Verbindung (höchstens 400 Pakete, wie die Anwendung höchstens das erste MiB zeigt)
    const segs = []; let count = 0;
    for (let n = 1; n <= D.count() && segs.length < 400; n += 1) {
      const p = D.pkt(n); if (p.conn !== c.id) continue;
      const d = D.detail(n); const L = d.layers;
      if (L.length < 5) continue;
      const app = L[L.length - 1]; const bytes = d.bytes.slice(app.range[0], app.range[1]);
      if (!bytes.length) continue;
      count += 1;
      const client = c.dir === 'in' ? p.src === c.src : p.dir === 'out';
      const last = segs[segs.length - 1];
      if (last && last.client === client) { last.bytes = last.bytes.concat(bytes); last.packets.push(n); } else segs.push({ client: client, bytes: bytes, packets: [n] });
    }
    const totalB = segs.reduce(function (s, x) { return s + x.bytes.length; }, 0);
    $('#main').innerHTML = pagehead('Stream folgen', '<a class="mono" href="verbindung.html?id=' + c.id + '">' + esc(connLabel(c)) + '</a>' + (c.svc ? ' · ' + c.svc : ''),
      '<div class="seg" id="sideSeg" role="group" aria-label="Richtung"><button type="button" data-v="both" aria-pressed="true">Beide Richtungen</button><button type="button" data-v="c" aria-pressed="false">Client → Server</button><button type="button" data-v="s" aria-pressed="false">Server → Client</button></div>' +
      '<div class="seg" id="modeSeg" role="group" aria-label="Darstellung"><button type="button" data-v="text" aria-pressed="true">Text</button><button type="button" data-v="hex" aria-pressed="false">Hex</button></div>' +
      '<a class="btn" href="pakete.html?c=' + c.id + '">' + I('packets', 'sm') + 'Pakete dieser Verbindung</a>',
      { back: 'pakete.html?c=' + c.id, crumb: '<a href="pakete.html?c=' + c.id + '">Pakete</a> <span>›</span> <span>Stream</span>' }) +
      (c.svc === 'https' || c.svc === 'imaps' ? '<div class="notice info">' + I('info') + '<div>Die Verbindung ist mit TLS verschlüsselt; lesbar sind nur Verbindungsaufbau und Servername, der Rest erscheint als Punkte.</div></div>' : '') +
      '<section class="card"><header><div class="legend"><span><i class="cli"></i>Client <span class="mono">' + esc(addr(c.src, c.sport)) + '</span></span><span><i class="srv"></i>Server <span class="mono">' + esc(addr(c.dst, c.dport)) + '</span></span></div><span class="hint" id="scount"></span></header><pre class="stream" id="stream"></pre></section>';
    function text(b) { let s = ''; for (let i = 0; i < b.length; i += 1) { const ch = b[i]; s += ch === 10 ? '\n' : ch === 13 ? '' : ch === 9 ? '\t' : ch >= 32 && ch < 127 ? String.fromCharCode(ch) : '.'; } return s; }
    function hexd(b, base) {
      let s = '';
      for (let o = 0; o < b.length; o += 16) {
        const row = b.slice(o, o + 16);
        s += (base + o).toString(16).padStart(8, '0') + '  ' + row.map(function (x, k) { return x.toString(16).padStart(2, '0') + (k === 7 ? ' ' : ''); }).join(' ').padEnd(49) + '  ' + text(row).replace(/\n|\t/g, '.') + '\n';
      }
      return s.replace(/\n$/, '');
    }
    function render() {
      let html = '', b = 0, pk = 0, oc = 0, os = 0;
      segs.forEach(function (sg) {
        if (side === 'c' && !sg.client || side === 's' && sg.client) return;
        b += sg.bytes.length; pk += sg.packets.length;
        const body = mode === 'hex' ? hexd(sg.bytes, sg.client ? oc : os) : text(sg.bytes).replace(/\n$/, '');
        if (sg.client) oc += sg.bytes.length; else os += sg.bytes.length;
        html += '<span class="' + (sg.client ? 'c' : 's') + '" title="Pakete ' + sg.packets.join(', ') + '">' + esc(body) + '</span>';
      });
      $('#stream').innerHTML = html || '<span class="gap">Diese Verbindung hat in den gespeicherten Paketen keine Nutzdaten.</span>';
      $('#scount').textContent = D.fbytes(b) + ' in ' + D.fnum(pk) + ' Paketen' + (side === 'both' ? '' : ' (von ' + D.fbytes(totalB) + ' in ' + count + ')');
    }
    bindSeg('sideSeg', function (v) { side = v; render(); });
    bindSeg('modeSeg', function (v) { mode = v; render(); });
    render();
  }

  /* ---------- Statistik ---------- */
  // Unterreiter der Statistik; Anrufe stehen bewusst nicht in der Kopfleiste (selten gebraucht, Variante D, Nutzer 2026-11-10)
  function statTabs(cur) {
    const n = D.calls().filter(function (k) { return !k.call.rtpOnly; }).length;
    return '<nav class="ptabs" aria-label="Bereiche der Statistik"><a href="statistik.html"' + (cur === 'p' ? ' aria-current="page"' : '') + '>' + I('statistics', 'sm') + 'Protokolle</a>' +
      '<a href="anrufe.html"' + (cur === 'a' ? ' aria-current="page"' : '') + '>' + I('phone', 'sm') + 'Anrufe' + (n ? '<span class="cnt">' + n + '</span>' : '') + '</a></nav>';
  }

  function pageStatistics() {
    if (none()) { $('#main').innerHTML = pagehead('Statistik', '') + noCapture(); return; }
    const S = selection();
    let f = S.f;
    $('#main').innerHTML = pagehead('Statistik', 'Protokolle, TCP-Probleme und Antwortzeiten der gespeicherten Pakete, für alle oder die des Anzeigefilters.',
      '<a class="btn" id="toPk" href="#">' + I('packets', 'sm') + 'Pakete anzeigen</a>') + statTabs('p') + connNote(S, 'statistik') +
      '<section class="card"><div class="toolbar">' + filterUI('sf', 'p', f, 'Anzeigefilter, z. B. ip.addr == 192.0.2.20 oder tcp.port == 443 (leer: alle Pakete)') + '</div>' + filterExtras('sf', 'p') + '</section>' +
      '<div class="tiles t5" id="stiles"></div>' +
      '<section class="card"><header><h2>Verkehr der Auswahl</h2><div class="legend"><span><i></i>Eingehend</span><span><i class="l2"></i>Ausgehend</span></div></header><div class="chartbox" id="sChart"></div></section>' +
      '<section class="card"><header><h2>Protokollhierarchie</h2><span class="hint">Jede Zeile zählt die Pakete, die das Protokoll enthalten, mit ihrer ganzen Länge. Ein Klick zeigt die Pakete.</span></header><div class="tblwrap"><table class="tbl hier"><thead><tr><th>Protokoll</th><th class="num">Pakete</th><th style="width:28%">Anteil</th><th class="num">Bytes</th><th class="num hide-sm">Ø Datenrate</th></tr></thead><tbody id="hier"></tbody></table></div></section>' +
      '<div class="grid2"><section class="card"><header><h2>TCP-Analyse</h2></header><p class="help" style="padding:var(--sp-3) var(--sp-4) 0;margin:0">Wiederholungen, Lücken und doppelte ACKs deuten auf Paketverlust hin, ein Nullfenster auf einen überlasteten Empfänger. Ein Klick zeigt die Pakete.</p><div class="tblwrap"><table class="tbl findings"><thead><tr><th>Befund</th><th class="num">Anzahl</th></tr></thead><tbody id="find"></tbody></table></div></section>' +
      '<section class="card"><header><h2>Antwortzeiten</h2></header><p class="help" style="padding:var(--sp-3) var(--sp-4) 0;margin:0">TCP-Verbindungsaufbau: SYN bis zum ACK des Clients. ACK: Zeit bis zur Bestätigung eines Segments (gemessen an diesem Rechner). DNS: Anfrage bis Antwort.</p><div class="tblwrap"><table class="tbl"><thead><tr><th>Messung</th><th class="num">Anzahl</th><th class="num">Mittel</th><th class="num">Höchstens</th></tr></thead><tbody id="times"></tbody></table></div></section></div>';
    const APP = { 'TLSv1.3': 'TLS', 'TLSv1.2': 'TLS', HTTP: 'HTTP', IPP: 'HTTP', SSHv2: 'SSH', SMB2: 'SMB2', DNS: 'DNS', MDNS: 'MDNS', QUIC: 'QUIC', SSDP: 'SSDP', NTP: 'NTP', DHCP: 'DHCP', SIP: 'SIP', 'SIP/SDP': 'SIP', RTP: 'RTP', RTCP: 'RTCP' };
    const FN = { Ethernet: 'eth', IPv4: 'ip', IPv6: 'ipv6', ARP: 'arp', TCP: 'tcp', UDP: 'udp', ICMP: 'icmp', TLS: 'tls', HTTP: 'http', SSH: 'ssh', SMB2: 'smb2', DNS: 'dns', MDNS: 'mdns', QUIC: 'quic', SSDP: 'ssdp', NTP: 'ntp', DHCP: 'dhcp', SIP: 'sip', SDP: 'sdp', RTP: 'rtp', RTCP: 'rtcp' };
    const LONG = { Ethernet: 'Ethernet', IPv4: 'Internet Protocol Version 4', IPv6: 'Internet Protocol Version 6', ARP: 'Address Resolution Protocol', TCP: 'Transmission Control Protocol', UDP: 'User Datagram Protocol', ICMP: 'Internet Control Message Protocol', TLS: 'Transport Layer Security', HTTP: 'Hypertext Transfer Protocol', SSH: 'SSH Protocol', SMB2: 'SMB2', DNS: 'Domain Name System', MDNS: 'Multicast DNS', QUIC: 'QUIC IETF', SSDP: 'Simple Service Discovery Protocol', NTP: 'Network Time Protocol', DHCP: 'Dynamic Host Configuration Protocol', SIP: 'Session Initiation Protocol', SDP: 'Session Description Protocol', RTP: 'Real-Time Transport Protocol', RTCP: 'Real-time Transport Control Protocol' };
    function render() {
      let pf = f ? D.packetFilter(f) : null; if (pf && pf.err) pf = null;
      const N = D.count(); let pk = 0, by = 0, t0 = null, t1 = 0, syn = 0, ackOut = 0, dnsR = 0;
      const tree = {}; const A = { 'Retransmission': 0, 'Dup ACK': 0, 'Out-Of-Order': 0, 'RST': 0 }; let ka = 0;
      const sel = [];
      for (let n = 1; n <= N; n += 1) {
        const p = D.pkt(n); if (!S.test(p, pf)) continue;
        const c = D.CBY[p.conn];
        pk += 1; by += p.len; if (t0 == null) t0 = p.t; t1 = p.t;
        const path = ['Ethernet', c.proto === 'ARP' ? 'ARP' : c.host.v6 ? 'IPv6' : 'IPv4'];
        if (c.proto !== 'ARP') path.push(c.proto);
        if (APP[p.proto]) path.push(APP[p.proto]);
        if (p.proto === 'SIP/SDP') path.push('SDP');
        let node = tree;
        path.forEach(function (k) { node[k] = node[k] || { _p: 0, _b: 0 }; node[k]._p += 1; node[k]._b += p.len; node = node[k]; });
        if (p.an) A[p.an] += 1;
        if (p.flags === 'SYN') syn += 1;
        if (c.proto === 'TCP' && p.dir === 'out' && !p.an) ackOut += 1;
        if (p.kind === 'dns-r') dnsR += 1;
        if (c.svc === 'ssh' && p.len === 102) ka += 1;
        sel.push(p);
      }
      const span = t0 == null ? 0 : t1 - t0;
      $('#stiles').innerHTML = [['Pakete', D.fnum(pk), ''], ['Bytes', D.fbytes(by).replace(' ', ' <small>') + '</small>', D.fnum(by) + ' Bytes'], ['Zeitraum', span < 60 ? D.fnum(span, span < 1 ? 3 : 1) + ' <small>s</small>' : D.fdur(span) + ' <small>min</small>', D.ftime(t0 || 0) + ' bis ' + D.ftime(t1) + ' s'],
        ['Ø Pakete', D.fnum(pk / Math.max(span, 1), 1) + ' <small>/s</small>', ''], ['Ø Datenrate', D.fbits(by / Math.max(span, 1)).replace(' ', ' <small>') + '/s</small>', '']].map(function (t) {
        return '<div class="tile"><div class="k">' + t[0] + '</div><div class="v">' + t[1] + '</div><div class="s">' + (t[2] || '&nbsp;') + '</div></div>';
      }).join('');
      // Ganze Aufzeichnung als Zeitachse; ist die Auswahl kürzer als 2 Minuten, nur ihr Zeitraum (sonst bliebe eine Nadel).
      const long = span >= 120;
      const from = long ? 0 : (t0 || 0);
      const dur = long ? Math.max(1, Math.ceil(D.now())) : Math.max(span, 0.001);
      const bucket = long ? Math.max(1, Math.ceil(dur / 240)) : dur / 120;
      const nb = long ? Math.ceil(dur / bucket) : 120; const bins = [];
      sel.forEach(function (p) { const bi = Math.min(nb - 1, Math.floor((p.t - from) / bucket)); bins[bi] = bins[bi] || [0, 0]; bins[bi][p.dir === 'in' ? 0 : 1] += p.len; });
      const pts = [];
      for (let i = 0; i < nb; i += 1) { const e = bins[i] || [0, 0]; pts.push({ s: from + i * bucket, a: e[0] / bucket, b: e[1] / bucket }); }
      const dec = dur < 1 ? 3 : dur < 10 ? 2 : 1;
      if (sel.length) drawChart($('#sChart'), pts, { label: 'Verkehr der Auswahl', h: 200, yfmt: function (v) { return D.fbytes(v) + '/s'; },
        xfmt: function (p) { return long ? D.clock(p.s).slice(0, 5) : '+' + D.fnum(p.s - from, dec) + ' s'; },
        tipx: function (p) { return long ? D.clock(p.s) + ' Uhr' + (bucket > 1 ? ' (' + bucket + ' s)' : '') : D.ftime(p.s) + ' s (+' + D.fnum(p.s - from, dec + 1) + ' s)'; }, ticks: 6 });
      else $('#sChart').innerHTML = '<div class="empty"><h2>Keine Pakete.</h2></div>';
      let rows = '';
      (function walk(node, depth, trail) {
        Object.keys(node).filter(function (k) { return k[0] !== '_'; }).sort(function (a, b) { return node[b]._p - node[a]._p; }).forEach(function (k) {
          const e = node[k]; const pct = pk ? e._p / pk * 100 : 0;
          const ff = (f ? '(' + f + ') and ' : '') + FN[k];
          rows += '<tr><td class="name"><span class="ind" style="width:' + depth * 18 + 'px"></span><a href="' + url('pakete.html', { f: ff, c: S.c, cf: S.cf }) + '" title="' + esc(LONG[k]) + '">' + esc(k) + '</a>' + (depth >= 3 ? ' <span class="muted small hide-sm">' + esc(LONG[k]) + '</span>' : '') + '</td><td class="num">' + D.fnum(e._p) + '</td>' +
            '<td><div class="pbar"><i style="width:' + Math.max(pct, .5).toFixed(1) + '%"></i><span>' + D.fnum(pct, 1) + ' %</span></div></td><td class="num">' + D.fbytes(e._b) + '</td><td class="num hide-sm">' + D.fbits(e._b / Math.max(span, 1)) + '/s</td></tr>';
          walk(e, depth + 1, trail.concat(k));
        });
      })(tree, 0, []);
      $('#hier').innerHTML = rows || '<tr><td colspan="5"><div class="empty"><h2>Keine Pakete.</h2></div></td></tr>';
      const fi = [['Wiederholte Segmente (Retransmission)', A['Retransmission'], 'tcp.analysis.retransmission'], ['Lücken (vorheriges Segment fehlt)', Math.round(A['Dup ACK'] / 3), 'tcp.analysis.lost_segment'], ['Segmente außer der Reihe', A['Out-Of-Order'], 'tcp.analysis.out_of_order'],
        ['Doppelte ACKs', A['Dup ACK'], 'tcp.analysis.duplicate_ack'], ['Nullfenster (Empfänger voll)', 0, 'tcp.analysis.zero_window'], ['Verbindungsabbrüche (RST)', A['RST'], 'tcp.flags.rst'], ['Keep-Alive', ka, 'tcp.analysis.keep_alive'], ['Fenster-Aktualisierungen', 0, 'tcp.analysis.window_update']];
      $('#find').innerHTML = fi.map(function (x) {
        return '<tr' + (x[1] ? ' class="has"' : ' class="off"') + '><td class="name">' + (x[1] ? '<a href="' + url('pakete.html', { f: (f ? '(' + f + ') and ' : '') + x[2], c: S.c, cf: S.cf }) + '">' + x[0] + '</a>' : x[0]) + '</td><td class="num"><span class="count' + (x[1] ? (x[2].indexOf('keep') < 0 ? ' warn' : '') : ' zero') + '">' + D.fnum(x[1]) + '</span></td></tr>';
      }).join('');
      const ms = function (v) { return D.fnum(v, 1) + ' ms'; };
      $('#times').innerHTML = [['TCP-Verbindungsaufbau', syn, 19.1, 61.4], ['TCP-Bestätigung (ACK)', ackOut, 21.7, 212.6], ['DNS-Antwort', dnsR, 6.2, 48.3]].map(function (x) {
        return '<tr' + (x[1] ? '' : ' class="off"') + '><td>' + x[0] + '</td><td class="num">' + D.fnum(x[1]) + '</td><td class="num">' + (x[1] ? ms(x[2]) : '–') + '</td><td class="num">' + (x[1] ? ms(x[3]) : '–') + '</td></tr>';
      }).join('');
      $('#toPk').href = url('pakete.html', { f: f, c: S.c, cf: S.cf });
    }
    bindFilter('sf', 'p', function (v) { f = v; setParam('f', v); render(); });
    render();
    let rt = null; window.addEventListener('resize', function () { clearTimeout(rt); rt = setTimeout(render, 150); });
  }

  /* ---------- Anrufe (VoIP, Vorhaben 5.7) ---------- */
  const CST = {
    running: ['Läuft', 'run', 'activity'], ended: ['Beendet', '', 'check-circle'], rejected: ['Abgelehnt', 'warn', 'x-circle'], missed: ['Verpasst', 'warn', 'alert'],
    cancelled: ['Abgebrochen', '', 'x-circle'], ringing: ['Klingelt', 'run', 'phone'], setup: ['Aufbau', '', 'phone'], rtp: ['Nur RTP', '', 'help-circle']
  };
  const SIPDE = { 486: 'besetzt', 487: 'abgebrochen', 404: 'nicht gefunden', 480: 'nicht erreichbar', 603: 'abgewiesen' };
  const RATE = { good: ['ok', 'check-circle', 'gut'], fair: ['warn', 'alert', 'mäßig'], bad: ['err', 'x-circle', 'schlecht'] };
  const ME = { name: 'Anna Beispiel', uri: 'sip:anna@sip.example.net' };
  function uriTxt(u) { return u.replace(/^sip:/, ''); }
  function callLabel(k) {
    if (k.rtpOnly) return 'RTP-Strom ohne SIP (' + D.H.meet.name + ')';
    const p = k.peer.name || uriTxt(k.peer.uri);
    return k.out ? ME.name + ' → ' + p : p + ' → ' + ME.name;
  }
  function mmss(sec) { sec = Math.max(0, Math.round(sec)); return Math.floor(sec / 60) + ':' + String(sec % 60).padStart(2, '0'); }
  function cstate(st) { const x = CST[st.state]; return '<span class="cstate ' + x[1] + '">' + I(x[2], 'sm') + x[0] + '</span>'; }
  function cstateSub(st) {
    if (st.state === 'rejected') return st.code + ' ' + ({ 486: 'Busy Here', 487: 'Request Terminated' }[st.code] || '') + (SIPDE[st.code] ? ' (' + SIPDE[st.code] + ')' : '');
    if (st.state === 'missed') return 'Anrufer hat aufgelegt';
    if (st.state === 'cancelled') return 'vor der Annahme aufgelegt';
    if (st.state === 'ended') return st.byUs ? 'selbst aufgelegt' : 'Gegenseite hat aufgelegt';
    if (st.state === 'rtp') return 'kein SIP gesehen';
    if (st.state === 'running') return 'seit ' + D.clock(st.tAnswer).slice(0, 5) + ' Uhr';
    return '';
  }
  function rateTag(r, note) { return !r ? '<span class="muted">–</span>' : '<span class="st ' + RATE[r][0] + '"' + (note ? ' title="' + esc(note) + '"' : '') + '>' + I(RATE[r][1], 'sm') + RATE[r][2] + '</span>'; }
  function party(p, here) {
    if (!p.uri) return '<span class="who">' + esc(D.H.meet.name) + '</span><span class="sub">198.51.100.90:10000</span>';
    const anon = /anonymous/.test(p.uri);
    const sub = here ? 'dieser Rechner' : anon ? 'Nummer unterdrückt' : p.name ? uriTxt(p.uri) : '';
    return '<span class="who" title="' + esc(uriTxt(p.uri)) + '">' + (p.name ? '<b>' + esc(p.name) + '</b>' : esc(uriTxt(p.uri))) + '</span>' + (sub ? '<span class="sub">' + esc(sub) + '</span>' : '');
  }
  function callFilter(st) { const k = st.call; return k.cid ? 'voip.call_id == "' + k.cid + '"' : 'rtp.ssrc in {' + D.hx8(st.o.ssrc) + ' ' + D.hx8(st.i.ssrc) + '}'; }
  function lawModal() {
    if ($('#lawModal')) return;
    document.body.insertAdjacentHTML('beforeend', '<div class="modal law" id="lawModal" role="dialog" aria-modal="true" aria-labelledby="lawTitle"><header><h2 id="lawTitle">' + I('alert') + 'Gespräch anhören?</h2><button class="iconbtn" type="button" data-close aria-label="Schließen">' + I('x') + '</button></header>' +
      '<div class="mbody"><p>Telefonate sind vertraulich. Wer das nichtöffentlich gesprochene Wort eines anderen ohne Einwilligung abhört, aufnimmt oder eine Aufnahme weitergibt, kann sich strafbar machen (§ 201 StGB).</p>' +
      '<p>Höre ein Gespräch nur an oder speichere es, wenn du selbst daran teilgenommen hast oder alle Beteiligten einverstanden sind, zum Beispiel bei der Fehlersuche an der eigenen Telefonanlage.</p>' +
      '<p class="small muted">Dieser Hinweis kommt einmal. Wieder anzeigen lässt er sich unter Einstellungen → Anrufe.</p></div>' +
      '<footer><button class="btn" type="button" data-close>Abbrechen</button><button class="btn primary" type="button" id="lawYes">Verstanden, anhören</button></footer></div>');
  }
  // Erst nach dem Hinweis auf § 201 StGB anhören oder speichern (einmal je Benutzer)
  function withLaw(what, go) {
    if (App.store.get('voipOk')) { go(); return; }
    lawModal();
    $('#lawYes').textContent = what === 'wav' ? 'Verstanden, speichern' : 'Verstanden, anhören';
    $('#lawYes').onclick = function () { App.store.set('voipOk', new Date().toLocaleDateString('de-DE')); App.closeModals(); go(); };
    App.openModal('lawModal');
  }
  function wavToast(st) { App.toast('Entwurf: speichert den Anruf als WAV (Dateidialog): 8 kHz, 16 bit, Stereo, links gesendet, rechts empfangen; ' + (st.i.lost + st.o.lost ? 'Lücken als Stille' : 'ohne Lücken')); }
  function voipHelp() {
    return '<ul class="small" style="margin:0;padding-left:20px;display:grid;gap:6px">' +
      '<li><b>Softphone auf diesem Rechner:</b> Die Aufzeichnung auf dem Netzwerkadapter genügt.</li>' +
      '<li><b>Telefon am Router oder an der Telefonanlage:</b> Den Verkehr sieht dieser Rechner nicht. Viele Router schneiden selbst mit (bei einer FRITZ!Box unter <span class="mono">fritz.box/html/capture.html</span>); die Datei unter Aufzeichnung → Aufzeichnungsdatei öffnen. Oder Port-Spiegelung am Switch.</li>' +
      '<li><b>Verschlüsselt:</b> SIP über TLS (Port 5061) und SRTP bleiben unlesbar. Dann gibt es keine Anrufliste, höchstens Ströme ohne Anruf, die sich nicht abspielen lassen.</li>' +
      '<li><b>Anhören</b> geht mit G.711 (A-law und µ-law) und braucht vollständige Pakete im Paketspeicher. G.722 und Opus folgen später.</li></ul>';
  }

  function pageCalls() {
    if (none()) { $('#main').innerHTML = pagehead('Statistik', '') + noCapture(); return; }
    const all = q.get('leer') === '1' ? [] : D.calls();
    const head = pagehead('Statistik', 'Telefonate über SIP und RTP (VoIP) in den gespeicherten Paketen, mit Ablauf, Sprachqualität und, bei G.711, zum Anhören.',
      '<a class="btn" href="' + url('pakete.html', { f: 'sip or rtp or rtcp' }) + '">' + I('packets', 'sm') + 'Pakete anzeigen</a>') + statTabs('a');
    if (!all.length) {
      $('#main').innerHTML = head + '<section class="card"><div class="empty">' + I('phone', 'xl') + '<h2>Keine Anrufe gefunden.</h2><p class="small" style="max-width:640px">openpacketmon erkennt Anrufe an SIP (UDP oder TCP, meist Port 5060) und an den Sprachströmen (RTP), die darin ausgehandelt werden.</p></div>' +
        '<div class="cbody" style="border-top:1px solid var(--border)"><h3 style="margin:0 0 var(--sp-2)">So kommen Anrufe in die Aufzeichnung</h3>' + voipHelp() + '</div></section>';
      return;
    }
    $('#main').innerHTML = head + '<div class="tiles t5" id="ctiles"></div>' +
      '<section class="card"><header><h2>Anrufe</h2><span class="hint">Neueste oben. Ein Klick zeigt Ablauf, Qualität und Anhören.</span></header><div class="tblwrap"><table class="tbl calls"><thead><tr><th>Zustand</th><th class="hide-sm">Beginn</th><th class="num">Dauer</th><th class="hide-md">Richtung</th><th>Von</th><th>An</th><th class="hide-md">Codec</th><th>Qualität</th><th class="num hide-sm">Pakete</th><th class="hide-sm"><span class="sr-only">Anhören</span></th></tr></thead><tbody id="cbody"></tbody></table></div>' +
      '<div class="cfoot" style="justify-content:flex-start" id="creg"></div></section>' +
      '<section class="card"><header><h2>So kommen Anrufe in die Aufzeichnung</h2></header><div class="cbody">' + voipHelp() + '</div></section>';
    function render() {
      const list = D.calls().slice().reverse();
      const sipc = list.filter(function (k) { return !k.call.rtpOnly; });
      const ans = sipc.filter(function (k) { return k.ok; }), noans = sipc.filter(function (k) { return !k.ok && k.state !== 'ringing' && k.state !== 'setup'; });
      const bad = list.filter(function (k) { return k.rate === 'bad'; }), rtpOnly = list.filter(function (k) { return k.call.rtpOnly; });
      const run = sipc.filter(function (k) { return k.state === 'running'; }).length;
      const talk = ans.reduce(function (a, k) { return a + k.talk; }, 0);
      $('#ctiles').innerHTML = [
        ['phone', 'Anrufe', sipc.length, run ? 'davon ' + run + ' läuft' : 'keiner läuft', ''],
        ['check-circle', 'Angenommen', ans.length, 'Gesprächszeit ' + mmss(talk), ''],
        ['x-circle', 'Nicht zustande gekommen', noans.length, noans.map(function (k) { return CST[k.state][0].toLowerCase(); }).join(', ') || '–', ''],
        ['alert', 'Schlechte Verbindung', bad.length, 'ab 3 % Verlust oder 50 ms Jitter', bad.length ? 'warn' : ''],
        ['lock', 'Ströme ohne Anruf', rtpOnly.length, 'RTP ohne SIP, nicht abspielbar', '']
      ].map(function (t) { return '<div class="tile ' + t[4] + '"><div class="k">' + I(t[0], 'sm') + t[1] + '</div><div class="v">' + t[2] + '</div><div class="s">' + t[3] + '</div></div>'; }).join('');
      $('#cbody').innerHTML = list.map(function (k) {
        const c = k.call;
        const us = { name: ME.name, uri: ME.uri };
        const from = c.rtpOnly ? { name: '', uri: 'sip:' + D.LOCAL } : c.out ? us : c.peer, to = c.rtpOnly ? c.peer : c.out ? c.peer : us;
        const durTxt = k.ok || c.rtpOnly ? mmss(k.talk) : '–';
        const durSub = k.ok ? (k.state === 'running' ? 'läuft' : '') : c.rtpOnly ? '' : (k.state === 'missed' ? 'geklingelt ' : 'nach ') + D.fnum(k.wait, 1) + ' s';
        return '<tr class="click" data-id="' + k.id + '"><td>' + cstate(k) + '<span class="sub">' + esc(cstateSub(k)) + '</span></td><td class="mono hide-sm">' + D.clock(k.t0).slice(0, 8) + '</td>' +
          '<td class="num" data-dur="' + k.id + '">' + durTxt + (durSub ? '<span class="sub">' + durSub + '</span>' : '') + '</td>' +
          '<td class="hide-md"><span class="dirc">' + (c.out ? I('arrow-up', 'sm') + 'ausgehend' : I('arrow-down', 'sm') + 'eingehend') + '</span></td>' +
          '<td>' + (c.rtpOnly ? '<span class="who mono">' + D.LOCAL + '</span><span class="sub">dieser Rechner · firefox</span>' : party(from, from === us)) + '</td><td>' + party(to, to === us) + '</td>' +
          '<td class="hide-md nowrap"' + (c.rtpOnly ? ' title="Nutzlast-Typ 111 (dynamisch), ohne SDP nicht zuzuordnen"' : '') + '>' + (k.codec || '<span class="muted">–</span>') + '</td><td>' + rateTag(k.rate, c.rtpOnly ? 'nur aus dem Verlust; Jitter ohne Taktrate unbekannt' : '') + '</td><td class="num hide-sm">' + D.fnum(k.packets) + '</td>' +
          '<td class="w0 hide-sm">' + (k.playable ? '<a class="btn sm ghost" href="anruf.html?id=' + k.id + '#anhoeren" title="Anhören" aria-label="Anhören">' + I('play', 'sm') + '</a>' : '') + '</td></tr>';
      }).join('');
      const reg = D.pkt(303);
      $('#creg').innerHTML = '<span class="small muted">' + I('check-circle', 'sm') + ' Angemeldet: <span class="mono">anna@sip.example.net</span> bei sip.example.net um ' + D.clock(reg.t).slice(0, 8) + ' Uhr, gültig 1 Stunde (<a href="paket.html?n=303">Paket 303</a>). Das Kennwort steht nicht in den Paketen, nur eine Prüfsumme (Digest).</span>';
    }
    render();
    $('#cbody').addEventListener('click', function (e) { if (e.target.closest('a')) return; const tr = e.target.closest('tr[data-id]'); if (tr) location.href = 'anruf.html?id=' + tr.dataset.id; });
    let last = 0;
    document.addEventListener('opm-tick', function () { const t = Date.now(); if (t - last < 1000) return; last = t; render(); });
  }

  function pageCall() {
    if (none()) { $('#main').innerHTML = pagehead('Anruf', '', '', { back: 'anrufe.html' }) + noCapture(); return; }
    const id = D.CALL[q.get('id')] && !D.CALL[q.get('id')].reg ? q.get('id') : 'A';
    let st = D.callStats(id); const k = st.call, R = k.rtp;
    const crumb = '<a href="statistik.html">Statistik</a> <span>›</span> <a href="anrufe.html">Anrufe</a> <span>›</span> <span>Anruf</span>';
    const fromTxt = k.rtpOnly ? D.LOCAL : k.out ? ME.name : (k.peer.name || uriTxt(k.peer.uri)), toTxt = k.rtpOnly ? D.H.meet.name : k.out ? (k.peer.name || uriTxt(k.peer.uri)) : ME.name;
    const pf = url('pakete.html', { f: callFilter(st) });
    $('#main').innerHTML = pagehead('<span class="tag">' + (k.rtpOnly ? 'RTP' : 'SIP') + '</span>' + esc(fromTxt) + ' → ' + esc(toTxt), cstate(st) + ' <span class="muted">' + esc(cstateSub(st)) + '</span>',
      '<a class="btn" href="' + pf + '">' + I('packets', 'sm') + (k.rtpOnly ? 'Pakete des Stroms' : 'Pakete des Anrufs') + '</a>' + (st.playable ? '<button class="btn" type="button" id="wavBtn">' + I('download', 'sm') + 'Als WAV speichern</button>' : ''),
      { back: 'anrufe.html', crumb: crumb }) +
      (st.rate === 'bad' ? '<div class="notice warn">' + I('alert') + '<div><b>Schlechte Verbindung beim ' + (st.i.rate === 'bad' ? 'Empfang' : 'Senden') + ':</b> ' + D.fnum((st.i.rate === 'bad' ? st.i : st.o).loss, 1) + ' % der Sprachpakete fehlen, Jitter im Mittel ' + D.fnum((st.i.rate === 'bad' ? st.i : st.o).jit[0], 1) + ' ms. Die Ursache liegt meist auf dem Weg zum Anbieter, etwa im WLAN oder an einer ausgelasteten Leitung; Sprache klingt dann abgehackt.</div></div>' : '') +
      '<div class="split2"><section class="card"><header><h2>' + (k.rtpOnly ? 'Strom' : 'Anruf') + '</h2></header><div class="cbody tight"><dl class="facts" id="cfacts"></dl></div></section>' +
      '<section class="card" id="anhoeren"><header><h2>Anhören</h2>' + (st.playable ? '<span class="hint">' + st.codec + ', 8 kHz, beide Richtungen</span>' : '') + '</header><div class="cbody" id="cplay"></div></section></div>' +
      '<section class="card"><header><h2>Ablauf</h2><span class="hint">' + (k.rtpOnly ? 'Ohne SIP gibt es keinen Ablauf, nur den Strom.' : 'SIP-Nachrichten und Sprachströme; ein Klick auf eine Zeile zeigt das Paket.') + '</span></header><div class="ladder" id="ladder"></div></section>' +
      '<section class="card"><header><h2>Sprachströme (RTP)</h2></header>' + (R ? '<p class="help" style="padding:var(--sp-3) var(--sp-4) 0;margin:0">Verlust aus den laufenden Nummern, Jitter nach RFC 3550 aus Ankunftszeit und Zeitstempel, größter Abstand zwischen zwei Paketen. Gut: unter 1 % Verlust und unter 30 ms Jitter; schlecht: ab 3 % oder ab 50 ms.</p>' +
      '<div class="tblwrap"><table class="tbl"><thead><tr><th>Richtung</th><th>Quelle → Ziel</th><th class="hide-md">SSRC</th><th class="num">Pakete</th><th class="num">Verloren</th><th class="num">Jitter Ø / max</th><th class="num hide-sm">Größter Abstand</th><th class="num hide-md">Folgefehler</th><th>Bewertung</th></tr></thead><tbody id="cstreams"></tbody></table></div><div class="cfoot" style="justify-content:flex-start" id="crtcp"></div>'
        : '<div class="empty"><h2>Kein Sprachstrom.</h2><p class="small">Der Anruf kam nicht zustande; angeboten war G.711 (A-law oder µ-law), gesprochen wurde nicht.</p></div>') + '</section>';

    function facts() {
      st = D.callStats(id);
      const row = function (dt, dd) { return '<dt>' + dt + '</dt><dd>' + dd + '</dd>'; };
      const nlink = function (n) { return '<a class="mono" href="paket.html?n=' + n + '">' + n + '</a>'; };
      let h = '';
      if (k.rtpOnly) {
        h += row('Zustand', cstate(st) + '<span class="muted">kein SIP gesehen</span>') +
          row('Endpunkte', '<span class="mono">' + D.LOCAL + ':' + k.media.lport + '</span> ↔ <span class="mono">198.51.100.90:' + k.media.rport + '</span><span class="muted">' + D.H.meet.name + '</span>') +
          row('Programm', '<b>firefox</b><span class="muted mono small">' + esc(D.PROGS.firefox) + '</span>') +
          row('Erkannt', 'an der Form der Pakete (Einstellungen → Anrufe, „RTP auch ohne SIP erkennen“)') +
          row('Codec', 'unbekannt: Typ 111 ist dynamisch und wird nur im SDP benannt') +
          row('Inhalt', 'vermutlich verschlüsselt (SRTP), etwa eine Videokonferenz im Browser; die Steuerung lief verschlüsselt über HTTPS') +
          row('Beginn', '<span class="mono">' + D.clock(st.t0, true).slice(0, 12) + '</span>') + row('Dauer', '<span class="mono">' + mmss(st.talk) + '</span>');
      } else {
        const us = k.out ? 'ausgehend' : 'eingehend';
        h += row('Zustand', cstate(st) + '<span class="muted">' + esc(cstateSub(st)) + '</span>') +
          row('Von', esc(k.out ? '"' + ME.name + '" <' + ME.uri + '>' : (k.peer.name ? '"' + k.peer.name + '" ' : '') + '<' + k.peer.uri + '>') + (/anonymous/.test(k.peer.uri) && !k.out ? '<span class="muted">Nummer unterdrückt</span>' : '')) +
          row('An', esc('<' + (k.out ? k.peer.uri : ME.uri) + '>')) +
          row('Richtung', (k.out ? I('arrow-up', 'sm') : I('arrow-down', 'sm')) + us + '<span class="muted">' + (k.out ? 'von diesem Rechner' : 'an diesen Rechner') + '</span>') +
          row('Beginn', '<span class="mono">' + D.clock(st.t0, true).slice(0, 12) + '</span><span class="links">· INVITE in Paket ' + nlink(st.inv.n) + '</span>') +
          row(st.ok ? 'Bis zur Annahme' : st.state === 'missed' ? 'Geklingelt' : 'Bis zur Antwort', '<span class="mono">' + D.fnum(st.wait, 1) + ' s</span>' + (st.ring ? '<span class="muted">klingelt ab Paket ' + nlink(st.ring.n) + '</span>' : '')) +
          (st.ok ? row('Gesprächsdauer', '<span class="mono" id="ctalk">' + mmss(st.talk) + '</span>' + (st.state === 'running' ? '<span class="cstate run">' + I('activity', 'sm') + 'läuft</span>' : '')) : '') +
          (st.endN ? row('Ende', '<span class="mono">' + D.clock(st.t1, true).slice(0, 12) + '</span><span class="muted">' + esc(cstateSub(st)) + ', Paket ' + nlink(st.endN) + '</span>') : '') +
          (R ? row('Codec', st.codec + ' (' + (R.pt === 8 ? 'PCMA' : 'PCMU') + '), 8 kHz<span class="links">· ausgehandelt in Paket ' + nlink(R.setup) + '</span>') : row('Angeboten', 'G.711 A-law, G.711 µ-law<span class="links">· SDP in Paket ' + nlink(st.inv.n) + '</span>')) +
          row('Programm', '<b>linphone</b><span class="muted mono small">' + esc(D.PROGS.linphone) + '</span>') +
          row('Anbieter', 'sip.example.net <span class="muted mono small">203.0.113.60</span>' + (R ? '<span class="muted">· Sprache über media.example.net</span>' : '')) +
          row('Call-ID', '<span class="mono">' + esc(k.cid) + '</span>') +
          (k.out ? row('Anmeldung', 'Digest im Kopf <span class="mono small">Proxy-Authorization</span><span class="muted">' + (id === 'C' ? 'nach 407 in Paket ' + nlink(12001) : 'ohne neue Rückfrage') + '; für KI-Assistenten (MCP) immer geschwärzt</span>') : '');
      }
      h += row('Pakete', '<span class="mono">' + D.fnum(st.packets) + '</span><span class="muted">' + (k.rtpOnly ? 'RTP' : st.rtp ? st.sip + ' SIP, ' + D.fnum(st.rtp) + ' RTP, ' + st.rtcp + ' RTCP' : 'SIP') + ' · ' + D.fbytes(st.bytes) + '</span><span class="links">· <a href="' + pf + '">anzeigen</a></span>');
      $('#cfacts').innerHTML = h;
    }

    function ladder() {
      const lanes = k.rtpOnly ? [['Dieser Rechner', D.LOCAL + ' · firefox'], ['kein SIP', '–'], [D.H.meet.name, '198.51.100.90']] :
        [['Dieser Rechner', D.LOCAL + ' · linphone'], ['sip.example.net', '203.0.113.60 · SIP'], ['media.example.net', '203.0.113.61 · RTP']];
      const pos = function (a, b) { const lo = Math.min(a, b), hi = Math.max(a, b); return 'left:' + ((lo * 2 + 1) / 6 * 100).toFixed(3) + '%;width:' + ((hi - lo) * 2 / 6 * 100).toFixed(3) + '%'; };
      const rel = function (n) { return '+' + D.fnum(D.time(n) - st.t0, 3) + ' s'; };
      const band = function () {
        const live = st.state === 'running';
        return '<div class="lrow band-row"><span class="lt">' + rel(st.rtpFirst) + '</span><div class="lanes"><span class="band' + (R.enc ? ' enc' : '') + '" style="' + pos(0, 2) + '">' + (R.enc ? I('lock', 'sm') : '') +
          '<b>RTP</b> ' + (R.enc ? 'Typ 111' : st.codec) + ' · ↑ ' + D.fnum(st.o.n) + ' · ↓ ' + D.fnum(st.i.n) + ' Pakete · ' + mmss(st.talk) + (st.rtcp ? ' · ' + st.rtcp + ' RTCP' : '') + (live ? ' · läuft' : '') + '</span></div>' +
          '<a class="ln" href="' + url('pakete.html', { f: 'rtp.ssrc in {' + D.hx8(st.o.ssrc) + ' ' + D.hx8(st.i.ssrc) + '}' }) + '" title="Pakete der Sprachströme">' + st.rtpFirst + ' …</a></div>';
      };
      let rows = '<div class="lrow lhead"><span>Zeit</span><div class="lanes">' + lanes.map(function (l) { return '<b>' + esc(l[0]) + '<small>' + esc(l[1]) + '</small></b>'; }).join('') + '</div><span>Paket</span></div>';
      let banded = false;
      st.msgs.forEach(function (m) {
        if (R && !banded && st.rtpFirst && m.n > st.rtpFirst) { rows += band(); banded = true; }
        const out = m.dir === 'out';
        const cls = (out ? 'r' : 'l') + (!m.req && m.what >= 400 && m.what !== 401 && m.what !== 407 ? ' bad' : '') + (!m.req && m.what === 200 && m.cm === 'INVITE' ? ' ok' : '');
        rows += '<div class="lrow" data-n="' + m.n + '"><span class="lt" title="' + D.clock(D.time(m.n), true).slice(0, 12) + ' Uhr">' + rel(m.n) + '</span><div class="lanes"><span class="arr ' + cls + '" style="' + pos(0, 1) + '"><span>' + esc(m.label) + '</span></span></div><a class="ln" href="paket.html?n=' + m.n + '">' + m.n + '</a></div>';
      });
      if (R && !banded && st.rtpFirst) rows += band();
      $('#ladder').innerHTML = rows;
    }

    function streams() {
      if (!R) return;
      const row = function (dir, sd) {
        const src = dir === 'o' ? D.LOCAL + ':' + k.media.lport : (R.enc ? '198.51.100.90' : '203.0.113.61') + ':' + k.media.rport;
        const dst = dir === 'o' ? (R.enc ? '198.51.100.90' : '203.0.113.61') + ':' + k.media.rport : D.LOCAL + ':' + k.media.lport;
        return '<tr><td><span class="dirc">' + (dir === 'o' ? I('arrow-up', 'sm') + 'gesendet' : I('arrow-down', 'sm') + 'empfangen') + '</span></td><td class="mono">' + src + ' → ' + dst + '</td>' +
          '<td class="mono hide-md"><a href="' + url('pakete.html', { f: 'rtp.ssrc == ' + D.hx8(sd.ssrc) }) + '">' + D.hx8(sd.ssrc) + '</a></td><td class="num">' + D.fnum(sd.n) + '</td>' +
          '<td class="num">' + (sd.lost ? '<span class="count warn">' + D.fnum(sd.lost) + '</span> <span class="muted">(' + D.fnum(sd.loss, 1) + ' %)</span>' : '0') + '</td>' +
          '<td class="num">' + (sd.jit ? D.fnum(sd.jit[0], 1) + ' / ' + D.fnum(sd.jit[1], 1) + ' ms' : '<span class="muted" title="Ohne SDP ist die Taktrate unbekannt">unbekannt</span>') + '</td>' +
          '<td class="num hide-sm">' + D.fnum(sd.gap, 1) + ' ms</td><td class="num hide-md">' + sd.ooo + '</td><td>' + rateTag(sd.rate, sd.jit ? '' : 'nur aus dem Verlust') + '</td></tr>';
      };
      $('#cstreams').innerHTML = row('o', st.o) + row('i', st.i);
      $('#crtcp').innerHTML = '<span class="small muted">' + (st.rtcp ? 'RTCP: ' + st.rtcp + ' Berichte. Die Gegenseite meldet für den gesendeten Strom ' + D.fnum(st.o.loss, 1) + ' % Verlust und ' + D.fnum(st.o.jit ? st.o.jit[0] + 0.2 : 0, 1) + ' ms Jitter.' : 'Keine RTCP-Berichte gesehen; Verlust und Jitter stammen nur aus den eigenen Messungen.') + '</span>';
    }

    function player() {
      const box = $('#cplay');
      if (!st.playable) {
        box.innerHTML = '<div class="empty" style="padding:var(--sp-4) 0">' + I(k.rtpOnly ? 'lock' : 'phone', 'xl') + '<h2>' + (k.rtpOnly ? 'Nicht abspielbar.' : 'Kein Gespräch zustande gekommen.') + '</h2><p class="small">' +
          (k.rtpOnly ? 'Ohne SIP ist der Codec unbekannt, und der Inhalt ist vermutlich verschlüsselt (SRTP).' : st.state === 'missed' ? 'Der Anruf wurde nicht angenommen; es gibt keine Sprache.' : 'Die Gegenseite hat abgelehnt (' + esc(cstateSub(st)) + '); es gibt keine Sprache.') + '</p></div>';
        return;
      }
      const BARS = 150, dur = Math.max(1, st.talk);
      // Erfundene Hüllkurve: abwechselnd spricht eine Seite, die andere hört zu
      function env(side) {
        const out = []; let t = 0, speaker = 0, seg = 0;
        const turns = []; while (t < dur) { const l = 2 + D.hash(seg + id.charCodeAt(0) * 31, 3) * 6; turns.push([t, t + l, seg % 2]); t += l; seg += 1; }
        for (let b = 0; b < BARS; b += 1) {
          const tt = (b + 0.5) / BARS * dur;
          const tr = turns.filter(function (x) { return tt >= x[0] && tt < x[1]; })[0] || [0, 0, 0];
          speaker = tr[2];
          const talk = speaker === side && D.hash(b * 7 + side, 11) > 0.12;
          out.push(talk ? 0.3 + D.hash(b * 13 + side, 12) * 0.7 : 0.04 + D.hash(b * 5 + side, 13) * 0.06);
        }
        return out;
      }
      const eo = env(0), ei = env(1);
      const gaps = {}; if (st.i.lost) { for (let g = 0; g < Math.min(18, st.i.lost); g += 1) gaps[Math.floor(D.hash(g, 21) * BARS)] = 1; }
      const lane = function (cls, lbl, sub, e, gp) { return '<div class="lane"><div class="lbl">' + lbl + '<small>' + sub + '</small></div><div class="wave ' + cls + '" data-w="' + cls + '">' + e.map(function (v, b) { return '<i' + (gp && gp[b] ? ' class="gap"' : '') + ' style="height:' + Math.round(8 + v * 92) + '%"></i>'; }).join('') + '<span class="head" style="left:0"></span></div></div>'; };
      box.innerHTML = '<div class="player">' +
        lane('out', I('arrow-up', 'sm') + ' Gesendet', ME.name, eo) +
        lane('in', I('arrow-down', 'sm') + ' Empfangen', esc(k.peer.name || uriTxt(k.peer.uri)), ei, gaps) +
        '<div class="axis"><span>0:00</span><span>' + mmss(dur / 2) + '</span><span>' + mmss(dur) + '</span></div>' +
        '<div class="ctl"><button class="btn primary" type="button" id="playBtn">' + I('play', 'sm') + '<span>Abspielen</span></button><span class="time" id="ptime">0:00 / ' + mmss(dur) + '</span>' +
        '<div class="seg sm" role="group" aria-label="Richtung" id="chSeg"><button type="button" data-v="both" aria-pressed="true">Beide</button><button type="button" data-v="out" aria-pressed="false">Gesendet</button><button type="button" data-v="in" aria-pressed="false">Empfangen</button></div></div>' +
        '<p class="help" style="margin:0">' + (st.i.lost ? 'Rot: verlorene Pakete, sie werden als Stille gespielt. ' : '') + (st.state === 'running' ? 'Der Anruf läuft noch; abgespielt wird, was bis jetzt im Paketspeicher liegt. ' : '') + 'Braucht vollständige Pakete im Paketspeicher.</p></div>';
      let pos = 0, playing = false, lastT = 0, toasted = false;
      function draw() {
        const f = Math.min(1, pos / dur);
        $$('#cplay .head').forEach(function (h) { h.style.left = (f * 100).toFixed(2) + '%'; });
        const k2 = Math.floor(f * BARS);
        $$('#cplay .wave').forEach(function (w) { Array.prototype.forEach.call(w.querySelectorAll('i'), function (b, i) { b.classList.toggle('done', i < k2); }); });
        $('#ptime').textContent = mmss(pos) + ' / ' + mmss(dur);
      }
      function frame(ts) { if (!playing) return; if (lastT) pos += (ts - lastT) / 1000; lastT = ts; if (pos >= dur) { pos = dur; stop(); } draw(); if (playing) requestAnimationFrame(frame); }
      function stop() { playing = false; $('#playBtn').innerHTML = I('play', 'sm') + '<span>Abspielen</span>'; }
      function start() {
        if (pos >= dur) pos = 0;
        playing = true; lastT = 0; $('#playBtn').innerHTML = I('pause', 'sm') + '<span>Anhalten</span>'; requestAnimationFrame(frame);
        if (!toasted) { toasted = true; App.toast('Entwurf: spielt keinen Ton ab. Im Programm über die Soundausgabe des Systems.'); }
      }
      $('#playBtn').addEventListener('click', function () { if (playing) stop(); else withLaw('play', start); });
      $$('#cplay .wave').forEach(function (w) { w.addEventListener('click', function (e) { const r = w.getBoundingClientRect(); pos = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)) * dur; draw(); }); });
      bindSeg('chSeg', function (v) { $('#cplay .wave.out').classList.toggle('mute', v === 'in'); $('#cplay .wave.in').classList.toggle('mute', v === 'out'); });
      if (location.hash === '#anhoeren') setTimeout(function () { $('#anhoeren').scrollIntoView({ block: 'start' }); }, 0);
    }

    facts(); ladder(); streams(); player();
    if ($('#wavBtn')) $('#wavBtn').addEventListener('click', function () { withLaw('wav', function () { wavToast(st); }); });
    $('#ladder').addEventListener('click', function (e) { if (e.target.closest('a')) return; const r = e.target.closest('.lrow[data-n]'); if (r) location.href = 'paket.html?n=' + r.dataset.n; });
    if (st.state === 'running') {
      let last = 0;
      document.addEventListener('opm-tick', function () { const t = Date.now(); if (t - last < 1000) return; last = t; facts(); ladder(); streams(); });
    }
  }

  /* ---------- Hosts, Dienste, Programme ---------- */
  function pageEntries(kind) {
    const T = { hosts: ['Hosts', 'Gegenstellen mit Domain, autonomem System und Land.'], dienste: ['Dienste', 'Dienste nach Port und Protokoll.'], programme: ['Programme', 'Programme dieses Rechners, die Verkehr verursachen.'] }[kind];
    if (none()) { $('#main').innerHTML = pagehead(T[0], '') + noCapture(); return; }
    let unit = App.store.get('unit') || 'bytes', sort = 'desc', onlyFav = false, txt = '';
    const data = kind === 'hosts' ? D.hostsAgg().map(function (e) { return { e: e, name: e.host.name, sub: hostSub(e.host), cc: e.host.cc, fav: !!e.host.fav || D.FAV_ADDR.indexOf(e.host.ip) >= 0, link: 'domain == ' + e.host.name }; }) :
      kind === 'dienste' ? D.servicesAgg().map(function (e) { return { e: e, name: e.name, sub: e.port + '/' + e.tp + ' · ' + e.cat, link: 'service == ' + e.name + ' and proto == ' + e.tp }; }) :
        D.programsAgg().map(function (e) { return { e: e, name: e.key, sub: e.path || 'Verkehr ohne zugeordnetes Programm', fav: D.FAV_PROG.indexOf(e.key) >= 0, link: e.key === 'unbekannt' ? 'not program' : 'program == "' + e.key + '"' }; });
    const stars = kind !== 'dienste';
    $('#main').innerHTML = pagehead(T[0], T[1]) +
      '<section class="card"><div class="toolbar"><label class="sr-only" for="etxt">Name enthält</label><input type="search" id="etxt" data-slash placeholder="Name enthält …">' +
      '<label class="sr-only" for="esort">Sortierung</label><select id="esort"><option value="desc">Größte zuerst</option><option value="asc">Kleinste zuerst</option><option value="recent">Neueste zuerst</option></select>' +
      unitSeg('eunit', unit) + (stars ? '<label class="switch"><input type="checkbox" id="efav"> Nur Favoriten</label>' : '') + '<span class="sp"></span><span class="sum" id="esum"></span></div>' +
      '<div class="tblwrap"><table class="tbl" id="etbl"><thead><tr>' + (stars ? '<th class="w0"><span class="sr-only">Favorit</span></th>' : '') + '<th>Name</th><th style="width:22%">Verkehr</th><th class="num">Eingehend</th><th class="num">Ausgehend</th><th class="num">Gesamt</th><th class="num hide-sm">Verbindungen</th></tr></thead><tbody></tbody></table></div></section>' +
      (kind === 'programme' ? '<p class="muted small">Programme werden nur bei der Aufzeichnung eines Netzwerkadapters erkannt.</p>' : kind === 'dienste' ? '<p class="muted small">Namen der Dienste aus der IANA-Registry; Dienste lassen sich nicht als Favorit markieren.</p>' : '');
    function render() {
      const key = function (x) { return unit === 'packets' ? x.e.packets : x.e.bytes; };
      let L = data.filter(function (x) { return (!txt || (x.name + ' ' + x.sub).toLowerCase().indexOf(txt) >= 0) && (!onlyFav || x.fav); });
      L.sort(sort === 'recent' ? function (a, b) { return b.e.last - a.e.last; } : sort === 'asc' ? function (a, b) { return key(a) - key(b); } : function (a, b) { return key(b) - key(a); });
      const max = L.reduce(function (m, x) { return Math.max(m, key(x)); }, 1);
      const fu = function (b, x) { return D.funit(unit, b, Math.round(x.e.packets * b / Math.max(1, x.e.bytes))); };
      $('#etbl tbody').innerHTML = L.map(function (x) {
        return '<tr class="click" data-l="' + esc(x.link) + '">' + (stars ? '<td class="w0">' + App.star(x.fav, x.name) + '</td>' : '') +
          '<td class="name"><a href="verbindungen.html?f=' + encodeURIComponent(x.link) + '">' + esc(x.name) + '</a> ' + App.flag(x.cc) + '<span class="sub">' + esc(x.sub) + '</span></td>' +
          '<td>' + App.tbar(unit === 'packets' ? x.e.packets * x.e.inB / x.e.bytes : x.e.inB, unit === 'packets' ? x.e.packets * x.e.outB / x.e.bytes : x.e.outB, max) + '</td>' +
          '<td class="num">' + fu(x.e.inB, x) + '</td><td class="num">' + fu(x.e.outB, x) + '</td><td class="num"><b>' + D.funit(unit, x.e.bytes, x.e.packets) + '</b></td><td class="num hide-sm">' + D.fnum(x.e.conns) + '</td></tr>';
      }).join('') || '<tr><td colspan="7"><div class="empty"><h2>Keine Einträge.</h2></div></td></tr>';
      $('#esum').textContent = L.length === 1 ? '1 Eintrag' : L.length + ' Einträge';
    }
    $('#etxt').addEventListener('input', function () { txt = this.value.trim().toLowerCase(); render(); });
    $('#esort').addEventListener('change', function () { sort = this.value; render(); });
    bindSeg('eunit', function (u) { unit = u; App.store.set('unit', u); render(); });
    if (stars) $('#efav').addEventListener('change', function () { onlyFav = this.checked; render(); });
    $('#etbl').addEventListener('click', function (e) {
      const s = e.target.closest('.star'); if (s) { const on = s.getAttribute('aria-pressed') !== 'true'; s.setAttribute('aria-pressed', String(on)); App.toast((on ? 'Zu den Favoriten hinzugefügt' : 'Aus den Favoriten entfernt') + ' (Entwurf)'); return; }
      if (e.target.closest('a')) return;
      const tr = e.target.closest('tr[data-l]'); if (tr) location.href = 'verbindungen.html?f=' + encodeURIComponent(tr.dataset.l);
    });
    render();
  }

  /* ---------- Benachrichtigungen ---------- */
  function pageNotifications() {
    const list = none() ? [] : D.NOTES.slice();
    $('#main').innerHTML = pagehead('Benachrichtigungen', 'Ereignisse der laufenden Aufzeichnung.',
      '<a class="btn ghost" href="einstellungen.html#benachrichtigungen">' + I('settings', 'sm') + 'Benachrichtigungen einstellen</a><button class="btn" type="button" id="clr"' + (list.length ? '' : ' disabled') + '>' + I('trash', 'sm') + 'Alle löschen</button>') +
      '<section class="card"><header><h2 id="ncount">' + (list.length === 1 ? '1 Benachrichtigung' : list.length + ' Benachrichtigungen') + '</h2><span class="hint">die neuesten 200 werden angezeigt</span></header><div id="nlist"></div></section>';
    function render() {
      $('#nlist').innerHTML = list.length ? '<div class="tblwrap"><table class="tbl"><thead><tr><th class="w0">Zeit</th><th>Ereignis</th><th>Favorit</th><th class="num">Daten</th></tr></thead><tbody>' + list.map(function (x) {
        const link = x[1].indexOf('(') > 0 ? 'ip.addr == ' + x[1].split(' ')[0] : x[1].indexOf('.') > 0 ? 'domain == ' + x[1] : 'program == "' + x[1] + '"';
        return '<tr><td class="mono w0">' + x[0] + '</td><td><span style="color:#c98a00">' + I('star', 'sm') + '</span> Verkehr eines Favoriten</td><td><a href="verbindungen.html?f=' + encodeURIComponent(link) + '">' + esc(x[1]) + '</a></td><td class="num">' + D.fbytes(x[2]) + '</td></tr>';
      }).join('') + '</tbody></table></div>' : '<div class="empty">' + I('bell', 'xl') + '<h2>Noch keine Benachrichtigungen.</h2><p class="small">Ein Hinweis kommt, sobald ein Favorit Daten austauscht (Hosts, Programme oder einzelne Adressen mit Stern).</p></div>';
    }
    render();
    $('#clr').addEventListener('click', function () { list.length = 0; render(); this.disabled = true; $('#ncount').textContent = '0 Benachrichtigungen'; const b = $('.topbar .badge'); if (b) b.remove(); });
  }

  /* ---------- Einstellungen ---------- */
  function sw(id, label, on, big) { return '<label class="switch' + (big ? ' big' : '') + '"><input type="checkbox" id="' + id + '"' + (on ? ' checked' : '') + '> ' + label + '</label>'; }
  const save = '<div class="cfoot"><button class="btn primary" type="button" data-draft-toast="Einstellungen gespeichert.">Speichern</button></div>';
  const PANELS = [
    ['anzeige', 'display', 'Anzeige', function () {
      return '<section class="card"><header><h2>Anzeige</h2></header><div class="cbody">' +
        '<div class="field-row"><div class="field"><label for="lang">Sprache</label><select id="lang"><option>Deutsch</option><option>English</option></select></div>' +
        '<div class="field"><label for="ps">Verbindungen pro Seite</label><select id="ps"><option>30</option><option>50</option><option>100</option></select></div></div>' +
        '<div class="field"><span class="label">Design</span><div class="seg" role="group" aria-label="Design"><button type="button" data-theme-btn="system">System</button><button type="button" data-theme-btn="light">Hell</button><button type="button" data-theme-btn="dark">Dunkel</button></div><span class="help">Wirkt sofort; „System“ folgt der Einstellung des Desktops.</span></div></div>' + save + '</section>';
    }],
    ['benachrichtigungen', 'bell', 'Benachrichtigungen', function () {
      return '<section class="card"><header><h2>Benachrichtigungen</h2></header><div class="cbody">' +
        '<div class="field">' + sw('nfav', 'Benachrichtigen, wenn ein Favorit Daten austauscht', true, true) + '<span class="help">Favoriten sind Hosts und Programme (Stern in den Listen) sowie einzelne IP-Adressen. Ein Hinweis kommt, sobald ein Favorit Daten austauscht.</span></div>' +
        '<div class="field"><label for="fa">Favorisierte IP-Adressen</label><textarea id="fa" rows="3" class="mono">203.0.113.44</textarea><span class="help">Eine Adresse je Zeile (IPv4 oder IPv6). Auch über den Stern auf der Seite einer Verbindung.</span></div>' +
        '<div class="field-row"><div class="field"><label for="snd">Ton</label><div style="display:flex;gap:var(--sp-2)"><select id="snd" style="flex:1"><option>Gluck</option><option>Plopp</option><option>Wusch</option><option>Kein Ton</option></select><button class="btn" type="button" data-draft-toast="Entwurf: spielt den Ton ab">' + I('volume', 'sm') + 'Anhören</button></div></div>' +
        '<div class="field"><label for="vol">Lautstärke</label><input id="vol" type="range" min="0" max="100" value="60"></div></div></div>' + save + '</section>';
    }],
    ['paketspeicher', 'database', 'Paketspeicher', function () {
      return '<section class="card"><header><h2>Paketspeicher</h2></header><div class="cbody">' +
        '<div class="field">' + sw('pst', 'Pakete speichern (Paketliste, Paketdetails, PCAP-Datei)', true, true) + '</div>' +
        '<div class="field-row"><div class="field"><label for="pmax">Höchstens Pakete</label><input id="pmax" type="number" value="500000" step="10000" min="1000" max="2000000"></div>' +
        '<div class="field"><label for="pmem">Höchstens Arbeitsspeicher</label><select id="pmem"><option>256 MiB</option><option selected>512 MiB</option><option>1 GiB</option><option>2 GiB</option></select></div></div>' +
        '<label class="check"><input type="checkbox"> Nur Kopfdaten speichern (die ersten 128 Bytes jedes Pakets)</label>' +
        '<p class="help" style="margin-top:var(--sp-2)">Gilt ab der nächsten Aufzeichnung (auch auf der Seite Aufzeichnung wählbar). Mit „Nur Kopfdaten“ fehlen Inhalte wie der TLS-Servername (SNI) teilweise, Stream folgen ist unvollständig, und Anrufe lassen sich nicht anhören. Die Pakete liegen nur im Arbeitsspeicher; ist eine der Grenzen erreicht, fallen die ältesten heraus.</p></div>' + save + '</section>';
    }],
    ['anrufe', 'phone', 'Anrufe', function () {
      const ok = App.store.get('voipOk');
      return '<section class="card"><header><h2>Anrufe (VoIP)</h2></header><div class="cbody">' +
        '<div class="field">' + sw('rtpH', 'RTP auch ohne SIP erkennen', true, true) + '<span class="help">Erkennt Sprach- und Videoströme an ihrer Form (Version 2, gleichbleibende SSRC, fortlaufende Nummern über mindestens 10 Pakete), auch wenn die Steuerung verschlüsselt lief, etwa bei einer Videokonferenz im Browser. Selten erscheint dabei anderer UDP-Verkehr fälschlich als RTP.</span></div>' +
        '<div class="field">' + sw('sipAny', 'SIP auch auf anderen Ports als 5060 erkennen', true) + '<span class="help">An der ersten Zeile, z. B. „INVITE sip:…“ oder „SIP/2.0 200 OK“.</span></div>' +
        '<dl class="facts"><dt>Anhören</dt><dd>G.711 A-law und µ-law, beide Richtungen; G.722 und Opus folgen später. Braucht vollständige Pakete im Paketspeicher.</dd>' +
        '<dt>Hinweis zu § 201 StGB</dt><dd id="lawState">' + (ok ? 'bestätigt am ' + esc(ok) + ' <button class="btn sm" type="button" id="lawReset">wieder zeigen</button>' : 'kommt vor dem ersten Anhören oder Speichern') + '</dd></dl></div>' + save + '</section>';
    }],
    ['geodaten', 'globe', 'Geodaten', function () {
      return '<section class="card"><header><h2>Geodaten</h2></header><div class="cbody"><dl class="facts" style="margin-bottom:var(--sp-4)">' +
        '<dt>Länder</dt><dd><span class="st ok">' + I('check-circle', 'sm') + 'DB-IP Lite</span><span class="muted">Stand 01.11.2026 (vor 8 Tagen)</span></dd>' +
        '<dt>Netzbetreiber (AS)</dt><dd><span class="st ok">' + I('check-circle', 'sm') + 'DB-IP Lite</span><span class="muted">Stand 01.11.2026 (vor 8 Tagen)</span></dd></dl>' +
        '<button class="btn" type="button" data-draft-toast="Entwurf: lädt die Geodatenbanken von db-ip.com herunter">' + I('download', 'sm') + 'Geodatenbanken von DB-IP herunterladen</button>' +
        '<p class="help" style="margin:var(--sp-2) 0 var(--sp-4)">Lädt die kostenlosen Datenbanken „IP to Country Lite“ und „IP to ASN Lite“ (CC BY 4.0, zusammen etwa 10 MB) von db-ip.com nach <code>~/.local/share/openpacketmon/geo</code>. Nur dabei fragt openpacketmon db-ip.com ab; jeden Monat erscheint ein neuer Stand.</p>' +
        '<fieldset class="fset"><legend>Eigene Datenbanken (optional)</legend><div class="field"><label for="mc">Eigene Länderdatenbank (MMDB)</label><input id="mc" type="text" class="mono"></div><div class="field"><label for="ma">Eigene ASN-Datenbank (MMDB)</label><input id="ma" type="text" class="mono"><span class="help">Pfad zu einer eigenen Datei, z. B. MaxMind GeoLite2 oder IPinfo Lite. Leer lassen: die heruntergeladenen Daten von DB-IP, sonst die Dateien von geoipupdate in /var/lib/GeoIP.</span></div></fieldset></div>' + save + '</section>';
    }],
    ['mcp', 'bot', 'KI-Assistenten (MCP)', function () {
      const m = App.mcp();
      return '<section class="card"><header><h2>KI-Assistenten (MCP)</h2>' + (m === 'off' ? '' : '<span class="st ' + (m === 'content' ? 'warn' : 'ok') + '">' + I(m === 'content' ? 'alert' : 'check-circle', 'sm') + (m === 'content' ? 'Paketinhalte erlaubt' : 'Aktiv') + '</span>') + '</header><div class="cbody">' +
        '<div class="field">' + sw('mcpOn', 'KI-Assistenten den Zugriff erlauben', m !== 'off', true) + '<span class="help">Programme wie Claude Code können dann auf diesem Rechner über MCP die Aufzeichnung abfragen, solange dieses Fenster offen ist: Verbindungen, Pakete ohne Inhalt, Statistik und Anrufe (ohne Ton; Rufnummern bis auf die letzten zwei Ziffern gekürzt). Die Abfragen lesen nur. Was ein Assistent abfragt, geht an seinen KI-Dienst.</span></div>' +
        (m === 'off' ? '' : '<p class="small">Aktiv, Abfragen seit dem Einschalten: <b class="mono">14</b></p><div class="field"><span class="label">Einrichten in Claude Code:</span><div class="codebox"><code>claude mcp add openpacketmon -- openpacketmon mcp</code><button class="iconbtn sm" type="button" aria-label="Kopieren" data-draft-toast="Befehl kopiert (Entwurf)">' + I('copy', 'sm') + '</button></div></div>' +
          '<fieldset class="fset"><legend>Nur für diese Sitzung</legend><div class="field">' + sw('mcpContent', 'Paketinhalte erlauben (Stream folgen, Bytes eines Pakets)', m === 'content') + '<span class="help">Gilt nur, bis openpacketmon geschlossen wird. Assistenten lesen dann auch den Inhalt unverschlüsselter Verbindungen. Werte geheimer Köpfe wie Cookie und Authorization, URL-Queries und Passwörter nach PASS oder AUTH werden durch x ersetzt; Geheimnisse an anderen Stellen bleiben sichtbar.</span></div></fieldset>') +
        '</div></section><p class="muted small">Der Schalter wirkt sofort, ohne Speichern. Unten in der Statusleiste steht, solange Assistenten Zugriff haben (grün) oder Paketinhalte lesen dürfen (orange).</p>';
    }],
    ['aktualisierungen', 'refresh', 'Aktualisierungen', function () {
      const u = App.upd(), N = App.NEWER, V = App.VERSION;
      const latest = u === 'none'
        ? '<span class="mono">' + V + '</span><span class="st ok">' + I('check-circle', 'sm') + 'aktuell</span>'
        : '<span class="mono">' + N + '</span><span class="muted">erschienen am ' + App.NEWER_DATE + '</span><span class="pill">neu</span>';
      // Ohne Launcher (oder unter Windows): kurz, wie es von Hand geht.
      const howto = u !== 'plain' ? '' : '<ol class="steps small">' +
        '<li><b>Linux:</b> <span class="mono">openpacketmon_' + N + '-1_amd64.deb</span> und <span class="mono">SHA256SUMS</span> von der Release-Seite laden, prüfen und installieren:' +
          '<div class="codebox" style="margin-top:4px"><code>sha256sum -c --ignore-missing SHA256SUMS &amp;&amp; sudo apt install ./openpacketmon_' + N + '-1_amd64.deb</code><button class="iconbtn sm" type="button" aria-label="Kopieren" data-draft-toast="Befehl kopiert (Entwurf)">' + I('copy', 'sm') + '</button></div></li>' +
        '<li><b>Windows:</b> <span class="mono">openpacketmon-' + N + '-setup.exe</span> laden und starten (signiert).</li>' +
        '<li>Danach openpacketmon neu starten. Eine laufende Aufzeichnung endet dabei; vorher als PCAP sichern.</li></ol>';
      const notes = u === 'none'
        ? '<h3>Highlights</h3><ul><li>Native interface (egui): capture page, overview with live chart and top lists, connections, hosts, services, programs, notifications and settings; English and German, light and dark</li>' +
          '<li>Packet list as in Wireshark with packet details, hex dump, display filter, coloring rules, follow stream and PCAP export</li>' +
          '<li>AI assistants can read the capture over MCP (<code>openpacketmon mcp</code>), off by default</li></ul>' +
          '<p>The full list of differences from Sniffnet is in the <a href="#" data-draft-toast="Entwurf: öffnet CHANGELOG.md auf GitHub">changelog</a>.</p>'
        : '<p>New interface in the style of opennetmon and opensecmon.</p>' +
          '<h3>Changed</h3><ul><li>Header bar instead of the sidebar; the recording stays visible at the top right</li>' +
          '<li>Status bar with packets, packet store, AI assistant notice and new versions</li><li>Fira Sans and Fira Code instead of Inter, calmer colors in the packet list</li><li>Settings in one place with a sub-navigation</li></ul>' +
          '<h3>Added</h3><ul><li>“Unusual” card on the overview</li><li>Keyboard shortcuts help (<code>?</code>), <code>/</code> jumps to the filter field</li><li>Bar for the selected packet above the packet list</li><li>Release notes in Settings → Updates</li></ul>' +
          '<h3>Installation</h3><p><b>Linux:</b> <code>sudo apt install ./openpacketmon_' + N + '-1_amd64.deb</code>. <b>Windows:</b> run <code>openpacketmon-' + N + '-setup.exe</code>. <code>SHA256SUMS</code> holds the checksums.</p>';
      return '<section class="card"><header><h2>Aktualisierungen</h2></header><div class="cbody">' +
        '<div class="field">' + sw('upd', 'Beim Start nach einer neuen Version suchen', true, true) + '<span class="help">Fragt GitHub nach der neuesten Version von openpacketmon; es werden keine Daten über dein Netz gesendet. openpacketmon lädt und installiert nichts selbst.</span></div>' +
        '<dl class="facts" style="margin-bottom:var(--sp-4)"><dt>Installiert</dt><dd class="mono">' + V + '</dd><dt>Neueste Version</dt><dd>' + latest + '</dd>' +
        '<dt>Zuletzt gesucht</dt><dd>heute, 09:12 (beim Start)</dd></dl>' +
        (u === 'none' ? '<button class="btn" type="button" data-draft-toast="Entwurf: öffnet die Release-Seite auf GitHub">' + I('external', 'sm') + 'Release-Seite</button>'
          : '<div class="notice info" style="margin:0">' + I('download') + '<div style="flex:1"><b>openpacketmon ' + N + ' ist erschienen.</b> ' + App.updText() + howto +
            '<div class="btngroup" style="margin-top:var(--sp-3)">' + App.updPrimary() + (u === 'launcher' ? '<button class="btn" type="button" data-draft-toast="Entwurf: öffnet die Release-Seite auf GitHub">' + I('external', 'sm') + 'Release-Seite</button>' : '') + '</div></div></div>') +
        '</div>' + save + '</section>' +
        '<section class="card" id="relnotes"><header><h2>Was ist neu in ' + (u === 'none' ? V : N) + '</h2><span class="hint">Release-Notes von GitHub, englisch wie dort' + (u === 'none' ? '' : '; erfunden') + '</span></header>' +
        '<div class="cbody relnotes">' + notes + '</div></section>';
    }],
    ['lizenzen', 'file', 'Über und Lizenzen', function () {
      return '<section class="card"><header><h2>Über openpacketmon</h2></header><div class="cbody tight"><dl class="facts">' +
        '<dt>Version</dt><dd class="mono">' + App.VERSION + '</dd><dt>Lizenz</dt><dd>MIT OR Apache-2.0</dd>' +
        '<dt>Quellcode</dt><dd><a href="#" data-draft-toast="Entwurf: öffnet github.com/tomCSLK/openpacketmon">github.com/tomCSLK/openpacketmon</a></dd>' +
        '<dt>Basiert auf</dt><dd>Sniffnet von Giuliano Bellini (MIT OR Apache-2.0), siehe NOTICE</dd>' +
        '<dt>Geodaten</dt><dd>IP Geolocation by <a href="#" data-draft-toast="Entwurf: öffnet db-ip.com">DB-IP</a> (CC BY 4.0)</dd></dl></div></section>' +
        '<section class="card"><header><h2>Fremdlizenzen</h2><span class="hint">THIRD-PARTY-LICENSES.txt, gekürzt</span></header><div class="cbody"><pre class="lic">openpacketmon enthält Software und Daten Dritter.\n\n' +
        '== Schriften ==\nFira Sans, Fira Code — SIL Open Font License 1.1 (neu mit dieser Oberfläche, ersetzt Inter)\n  Copyright (c) 2012-2015 The Mozilla Foundation and Telefonica S.A.\n  Copyright (c) 2014 The Fira Code Project Authors\n\n' +
        '== Symbole ==\nnach Lucide — ISC License\n\n== Dienstnamen ==\nIANA Service Name and Transport Protocol Port Number Registry — gemeinfrei\n\n== Rust-Crates ==\neframe, egui … MIT OR Apache-2.0\npcap … MIT OR Apache-2.0\n(… 313 Crates, vollständige Texte in der Datei)</pre></div></section>';
    }]
  ];
  function pageSettings() {
    $('#main').innerHTML = pagehead('Einstellungen', 'Gelten für alle Aufzeichnungen.') +
      '<div class="split settings"><nav class="subnav" aria-label="Einstellungen">' + PANELS.map(function (p) { return '<a href="#' + p[0] + '" data-p="' + p[0] + '">' + I(p[1]) + p[2] + '</a>'; }).join('') + '</nav><div>' +
      PANELS.map(function (p) { return '<div class="panel" id="p-' + p[0] + '" hidden>' + p[3]() + '</div>'; }).join('') + '</div></div>';
    function show() {
      const h = (location.hash || '#anzeige').slice(1); const ok = PANELS.some(function (p) { return p[0] === h; }) ? h : 'anzeige';
      $$('.panel').forEach(function (p) { p.hidden = p.id !== 'p-' + ok; });
      $$('.subnav a').forEach(function (a) { if (a.dataset.p === ok) a.setAttribute('aria-current', 'page'); else a.removeAttribute('aria-current'); });
    }
    window.addEventListener('hashchange', show); show();
    // „Was ist neu?“ aus dem Hinweis auf eine neue Version: gleich zu den Release-Notes
    if (/[?&]notes=1/.test(location.search) && $('#relnotes')) $('#relnotes').scrollIntoView({ block: 'start' });
    $('#ps').value = App.store.get('ps') || '30';
    $('#ps').addEventListener('change', function () { App.store.set('ps', this.value); });
    $$('#p-anzeige [data-theme-btn]').forEach(function (b) { b.setAttribute('aria-pressed', String(b.dataset.themeBtn === (App.store.get('theme') || 'system'))); b.addEventListener('click', function () { $$('#draftbar [data-theme-btn="' + b.dataset.themeBtn + '"]')[0].click(); $$('#p-anzeige [data-theme-btn]').forEach(function (o) { o.setAttribute('aria-pressed', String(o === b)); }); }); });
    const lr = $('#lawReset'); if (lr) lr.addEventListener('click', function () { App.store.set('voipOk', ''); $('#lawState').textContent = 'kommt vor dem ersten Anhören oder Speichern'; });
    const on = $('#mcpOn'); if (on) on.addEventListener('change', function () { App.store.set('mcp', this.checked ? 'on' : 'off'); location.reload(); });
    const ct = $('#mcpContent'); if (ct) ct.addEventListener('change', function () { App.store.set('mcp', this.checked ? 'content' : 'on'); location.reload(); });
  }

  /* ---------- Logo ---------- */
  const LOGOS = [
    ['m-a', 'A · Offenes Paket', 'Ein offener Karton, aus dem zwei Pakete springen: Name und Zweck in einem Bild, ohne Lupe oder Bildschirm. Unterscheidet sich klar von opensecmon (Schild) und opennetmon (Bildschirm) und bleibt bei 16 px als Kiste erkennbar.'],
    ['m-b', 'B · Bildschirm mit Paketen', 'Derselbe Bildschirm wie das Zeichen von opennetmon, statt der Pulslinie drei Pakete. Zeigt die Familie am deutlichsten, ist aber leicht mit opennetmon zu verwechseln.'],
    ['m-c', 'C · Lupe über Paketen', 'Pakete unter der Lupe: „hineinschauen“ wie bei Wireshark. Gut lesbar; die Lupe steht in vielen Programmen aber für „Suchen“.'],
    ['m-d', 'D · Paketstrom im Ring', 'Ein Kreis mit einem Strom aus Paketen, der durchläuft – die Leitung, die mitgeschnitten wird. Ruhig und eigenständig, aber abstrakt.'],
    ['m-e', 'E · Klammern mit Bytes', 'Zwei spitze Klammern um zwei Bytes, wie in einem Hex-Dump. Technisch und knapp, erinnert aber eher an ein Entwicklerwerkzeug.']
  ];
  function pageLogo() {
    const cur = App.logoName();
    const sizes = [48, 32, 24, 16];
    const g = function (id, z) { return '<span class="glyph" style="width:' + z + 'px;height:' + z + 'px"><svg viewBox="0 0 24 24"><use href="#i-' + id + '"/></svg></span>'; };
    const ai = function (id, z) { return '<span class="appicon" style="width:' + z + 'px;height:' + z + 'px" title="' + z + ' px"><svg viewBox="0 0 24 24"><use href="#i-' + id + '"/></svg></span>'; };
    $('#main').innerHTML = pagehead('Logo-Entwürfe', 'Im Stil von opensecmon und opennetmon: ein Zeichen im Lucide-Stil (Strichstärke 2, runde Enden) in der Akzentfarbe, als Programmsymbol weiß auf blauem Quadrat. „In Kopfleiste verwenden“ setzt den Entwurf oben ein.') +
      '<section class="card"><header><h2>Familie</h2><span class="hint">Die drei Programme nebeneinander</span></header><div class="cbody"><div class="family">' +
      '<div>' + ai('m-sec', 32) + '<span>opensecmon<br><span class="who">Schild mit Radar</span></span></div>' +
      '<div>' + ai('m-net', 32) + '<span>opennetmon<br><span class="who">Bildschirm mit Puls (gewählt: C)</span></span></div>' +
      '<div class="mine">' + ai(cur, 32) + '<span>openpacketmon<br><span class="who">Entwurf ' + cur.slice(2).toUpperCase() + ' (in der Kopfleiste)</span></span></div>' +
      '<div><img src="../assets/openpacketmon-logo.jpg" alt="Heutiges Logo von openpacketmon" width="54" height="40" style="border-radius:6px;object-fit:cover"><span>heute<br><span class="who">violette Kugel, wird ersetzt</span></span></div></div></div></section>' +
      '<div class="notice info">' + I('info') + '<div>Alle Entwürfe sind einfarbig und kommen mit hellem und dunklem Hintergrund aus. Aus dem gewählten Zeichen würden danach Programmsymbol (PNG 16–512 px, <code>.ico</code> für Windows), Fenstersymbol und das Bild in der README erzeugt; das heutige Logo bleibt nur in der Geschichte des Repos.</div></div>' +
      '<div class="logos">' + LOGOS.map(function (l) {
        return '<section class="card logo"><h2>' + l[1] + (l[0] === 'm-c' ? ' <span class="pill">Gewählt</span>' : '') + '</h2><p class="why">' + l[2] + '</p>' +
          '<div class="row">' + g(l[0], 48) + sizes.map(function (z) { return ai(l[0], z); }).join('') + '</div>' +
          '<div class="fake-top">' + g(l[0], 24) + 'openpacketmon</div>' +
          '<div class="dark-patch">' + g(l[0], 32) + ai(l[0], 32) + ai(l[0], 16) + '<span style="font-size:12px">dunkel</span></div>' +
          '<div class="btngroup" style="margin-top:var(--sp-3)"><button class="btn' + (cur === l[0] ? '' : ' primary') + '" type="button" data-logo="' + l[0] + '"' + (cur === l[0] ? ' aria-pressed="true"' : '') + '>' + (cur === l[0] ? I('check') + 'In Verwendung' : 'In Kopfleiste verwenden') + '</button></div></section>';
      }).join('') + '</div>';
    $$('[data-logo]').forEach(function (b) { b.addEventListener('click', function () { App.store.set('logo', b.dataset.logo); location.reload(); }); });
  }

  /* ---------- Breiten ---------- */
  function pageWidths() {
    const p = q.get('p') || 'pakete';
    const PG = [['uebersicht', 'Übersicht'], ['pakete', 'Pakete'], ['paket', 'Paket'], ['verbindungen', 'Verbindungen'], ['statistik', 'Statistik'], ['anrufe', 'Anrufe'], ['anruf', 'Anruf'], ['aufzeichnung', 'Aufzeichnung'], ['einstellungen', 'Einstellungen']];
    const sets = [[1440, 900, 0.5, '1440 × 900'], [1280, 820, 0.5, '1280 × 820 (Standardgröße des Fensters)'], [1024, 720, 0.5, '1024 × 720'], [760, 480, 0.62, '760 × 480 (Mindestgröße)']];
    $('#main').innerHTML = pagehead('Breiten', 'Dieselbe Seite bei vier Fenstergrößen (verkleinert dargestellt). Maßgeblich ist 1280 × 820, die Standardgröße des Fensters (Nutzer, 2026-11-09); die kleineren Größen zeigen nur, wie die Seite enger wird, und sind nicht eigens gestaltet.',
      '<div class="seg" role="group" aria-label="Seite">' + PG.map(function (x) { return '<a href="breiten.html?p=' + x[0] + '"' + (x[0] === p ? ' aria-current="page"' : '') + '>' + x[1] + '</a>'; }).join('') + '</div>') +
      '<div class="frames">' + sets.map(function (s) {
        return '<figure class="frame"><div class="fw" style="width:' + s[0] * s[2] + 'px;height:' + s[1] * s[2] + 'px"><iframe title="' + s[3] + '" src="' + p + '.html?frame=1" width="' + s[0] + '" height="' + s[1] + '" style="transform:scale(' + s[2] + ')" loading="lazy"></iframe></div><figcaption>' + s[3] + '</figcaption></figure>';
      }).join('') + '</div>';
  }

  /* ---------- Entwurf (Start) ---------- */
  function pageIndex() {
    const sw = [['--bg', 'Hintergrund'], ['--surface', 'Fläche'], ['--surface-2', 'Fläche 2'], ['--border', 'Linie'], ['--text', 'Text'], ['--text-3', 'Text gedämpft'], ['--accent', 'Akzent'], ['--ok', 'In Ordnung'], ['--warn', 'Warnung'], ['--err', 'Fehler'], ['--s1', 'Eingehend'], ['--s2', 'Ausgehend']];
    const rows = [['--row-bad', 'Fehler, Reset, TCP-Probleme'], ['--row-tcp', 'TCP-Auf-/Abbau'], ['--row-dns', 'DNS'], ['--row-local', 'ARP, DHCP, SSDP, NTP'], ['--row-tls', 'TLS'], ['--row-http', 'HTTP'], ['--row-icmp', 'ICMP'], ['--row-udp', 'QUIC, UDP'], ['--row-voip', 'SIP, RTP, RTCP'], ['--cli-bg', 'Stream: Client'], ['--srv-bg', 'Stream: Server'], ['--hl', 'Hex: Auswahl']];
    $('#main').innerHTML = pagehead('openpacketmon – Entwurf der neuen Oberfläche', 'Gestaltung und Farben wie opensecmon und der Entwurf von opennetmon; Inhalt und Funktionen wie openpacketmon ' + App.VERSION + '.') +
      '<div class="notice info">' + I('info') + '<div><b>Kein Anwendungscode.</b> Klickbare Seiten mit erfundenen Daten; die Anwendung zeichnet weiter mit egui. Oben die Entwurfsleiste: Seite, Zustand der Aufzeichnung („läuft“, „pausiert“, „keine“), neue Version („mit Launcher“, „ohne Launcher“, „keins“) und Thema (System, Hell, Dunkel). Läuft die Aufzeichnung, kommen wie im Programm alle 0,3 s neue Pakete dazu.</div></div>' +
      '<h3 style="margin-bottom:var(--sp-2)">Seiten</h3><div class="cards3" style="margin-bottom:var(--sp-5)">' +
      [['aufzeichnung', 'capture', 'Aufzeichnung', 'Quelle, Filter, Paketspeicher; läuft sie, Zustand und Knöpfe'], ['uebersicht', 'overview', 'Übersicht', 'Kacheln, „Auffällig“, Verkehr pro Sekunde, Top-Listen'], ['verbindungen', 'connections', 'Verbindungen', 'Filter mit Vorschlägen, Seiten, Favoriten'],
        ['verbindung', 'connections', 'Verbindung', 'Fakten, Gegenstelle, Sprünge zu Paketen und Stream'], ['pakete', 'packets', 'Pakete', 'durchgehende Liste mit Live-Folgen, Anzeigefilter, Gehe zu'], ['paket', 'layers', 'Paket', 'Schichten und Bytes, Felder markieren ihre Bytes'],
        ['stream', 'stream', 'Stream folgen', 'Client und Server farbig, Text oder Hex'], ['statistik', 'statistics', 'Statistik', 'Reiter Protokolle (Diagramm, Hierarchie, TCP-Analyse, Antwortzeiten) und Anrufe'], ['anrufe', 'phone', 'Anrufe', 'unter Statistik: SIP und RTP, Ablauf, Sprachqualität, Anhören (G.711); neu'], ['hosts', 'hosts', 'Hosts, Dienste, Programme', 'Listen mit Verkehrsbalken und Favoriten'],
        ['benachrichtigungen', 'bell', 'Benachrichtigungen', 'Verkehr der Favoriten'], ['einstellungen', 'settings', 'Einstellungen', 'ein Bereich mit Unternavigation, „Über und Lizenzen“ darin'], ['logo', 'm-c', 'Logo', 'fünf Entwürfe, gewählt C „Lupe über Paketen“'], ['breiten', 'display', 'Breiten', '1440 px bis 760 px; maßgeblich ist 1280 × 820']].map(function (x) {
        return '<a class="tile" href="' + x[0] + '.html"><div class="k">' + I(x[1], 'sm') + x[2] + '</div><div class="s" style="margin-top:4px;white-space:normal">' + x[3] + '</div></a>';
      }).join('') + '</div>' +
      '<h3 style="margin-bottom:var(--sp-2)">Was sich gegenüber heute ändert</h3><section class="card"><div class="cbody"><ul style="margin:0;padding-left:20px;display:grid;gap:6px">' +
      '<li><b>Kopfleiste statt violetter Seitenleiste:</b> die Verkehrsseiten stehen vorn (Übersicht, Verbindungen, Pakete, Statistik, Hosts, Dienste, Programme; bei schmalem Fenster die letzten drei unter „Mehr“, darunter nur Symbole). Benachrichtigungen werden zur Glocke mit Zähler, Einstellungen zum Zahnrad. Die Paketliste bekommt dadurch die volle Breite.</li>' +
      '<li><b>Aufzeichnung immer sichtbar:</b> statt eines Menüpunkts steht rechts oben der Zustand (läuft, pausiert) mit Adapter, Dauer, Pausieren und Beenden; ohne Aufzeichnung ein Knopf „Aufzeichnung starten“.</li>' +
      '<li><b>Statusleiste unten</b> statt des Fußes der Seitenleiste: Pakete, Paketspeicher, verworfene Pakete, Hinweis auf KI-Assistenten (MCP), neue Version, Version, Quellcode, Sniffnet, Lizenzen und der Hinweis auf DB-IP (bleibt immer sichtbar).</li>' +
      '<li><b>Neue Version:</b> Der Hinweis in der Statusleiste öffnet ein kleines Fenster. Ist der Launcher installiert (Linux), führt „Aktualisieren“ zu ihm; er lädt, prüft und installiert, sobald openpacketmon geschlossen ist. Ohne Launcher und unter Windows öffnet der Knopf die Release-Seite. „Was ist neu?“ zeigt die Release-Notes unter Einstellungen → Aktualisierungen (heute nur ein Link zur Release-Seite).</li>' +
      '<li><b>Farben:</b> graue Flächen mit ruhigem Blau statt Violett, Zustände immer mit Symbol und Text. Die Farben der Paketliste bleiben in ihrer Bedeutung, sind aber leiser getönt und für hell und dunkel abgestimmt; Eingehend Blau, Ausgehend Orange.</li>' +
      '<li><b>Schrift:</b> Fira Sans und Fira Code (Adressen, Zeiten, Hex) statt Inter; dichte Zeilen (Tabellen 34 px, Paketliste 26 px).</li>' +
      '<li><b>Einstellungen</b> als ein Bereich mit Unternavigation; „Lizenzen“ wandert dorthin („Über und Lizenzen“), der Link in der Statusleiste bleibt.</li>' +
      '<li><b>Neu:</b> „Auffällig“ auf der Übersicht, Tastenkürzel-Hilfe mit „?“, „/“ für das Filterfeld, Leiste über der Paketliste für das gewählte Paket, Rückfrage beim Beenden. Das gibt es heute nicht; vom Nutzer alle übernommen.</li>' +
      '<li><b>Neu: Anrufe (VoIP).</b> SIP, SDP, RTP und RTCP in der Paketliste (eigene Farbe, Filter <code>sip</code>, <code>rtp</code>, <code>voip.call_id</code> …) und unter Statistik der Reiter „Anrufe“ mit Ablauf, Sprachqualität und Anhören von G.711 (nicht in der Kopfleiste, weil selten gebraucht); vor dem ersten Anhören ein Hinweis auf § 201 StGB.</li>' +
      '<li><b>Logo:</b> neues Zeichen im Stil der beiden anderen Programme, gewählt C „Lupe über Paketen“ (Seite Logo).</li></ul></div></section>' +
      '<h3 style="margin:var(--sp-5) 0 var(--sp-2)">Farben</h3><div class="swatches">' + sw.map(function (x) { return '<div class="sw"><i style="background:var(' + x[0] + ')"></i><div>' + x[1] + '<br><code>' + x[0] + '</code></div></div>'; }).join('') + '</div>' +
      '<h3 style="margin:var(--sp-4) 0 var(--sp-2)">Paketliste, Stream, Hex</h3><div class="swatches">' + rows.map(function (x) { return '<div class="sw"><i style="background:var(' + x[0] + ')"></i><div>' + x[1] + '<br><code>' + x[0] + '</code></div></div>'; }).join('') + '</div>' +
      '<p class="muted small" style="margin-top:var(--sp-5)">Schriften: Fira Sans und Fira Code (Fontsource 5.3.0, SIL OFL 1.1, Lizenztexte in <code>fonts/</code>). Symbole nach Lucide (ISC), für den Entwurf nachgezeichnet. Alle Daten sind erfunden: Adressen aus den Dokumentationsbereichen 192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24 und 2001:db8::/32, MAC-Adressen 00:00:5e:00:53:xx, AS-Nummern 64496–64511, Namen unter example.com/.net/.org.</p>';
  }

  document.addEventListener('DOMContentLoaded', function () {
    const page = document.body.dataset.page;
    const active = { uebersicht: 'uebersicht', verbindungen: 'verbindungen', verbindung: 'verbindungen', pakete: 'pakete', paket: 'pakete', stream: 'pakete', statistik: 'statistik', anrufe: 'statistik', anruf: 'statistik', hosts: 'hosts', dienste: 'dienste', programme: 'programme', benachrichtigungen: 'benachrichtigungen', einstellungen: 'einstellungen' }[page] || '';
    App.shell(active);
    ({ index: pageIndex, aufzeichnung: pageCapture, uebersicht: pageOverview, verbindungen: pageConnections, verbindung: pageConnection, pakete: pagePackets, paket: pagePacket, stream: pageStream, statistik: pageStatistics, anrufe: pageCalls, anruf: pageCall,
      hosts: function () { pageEntries('hosts'); }, dienste: function () { pageEntries('dienste'); }, programme: function () { pageEntries('programme'); },
      benachrichtigungen: pageNotifications, einstellungen: pageSettings, logo: pageLogo, breiten: pageWidths })[page]();
    document.addEventListener('click', function (e) { const b = e.target.closest('[data-back]'); if (b) { e.preventDefault(); App.back(b.dataset.back); } });
  });
})();
