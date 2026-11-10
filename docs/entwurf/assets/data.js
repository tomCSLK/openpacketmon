/* Entwurf openpacketmon – erfundene Beispieldaten.
   Adressen nur aus den Dokumentationsbereichen (192.0.2.0/24 als eigenes Netz, 198.51.100.0/24, 203.0.113.0/24,
   2001:db8::/32), MAC-Adressen aus 00:00:5e:00:53:xx, AS-Nummern 64496–64511, Namen unter example.com/.net/.org.
   Jedes Paket entsteht allein aus seiner Nummer; Verbindungen, Hosts, Dienste, Programme und Diagramme werden
   daraus zusammengezählt, deshalb passen alle Seiten zueinander. */
(function () {
  const store = {
    get: function (k) { try { return localStorage.getItem('opm-entwurf-' + k); } catch (e) { return null; } },
    set: function (k, v) { try { localStorage.setItem('opm-entwurf-' + k, v); } catch (e) { /* egal */ } }
  };
  function hash(n, k) {
    let h = Math.imul(n ^ 0x9e3779b9, 0x85ebca6b) ^ Math.imul((k | 0) + 0x7f4a7c15, 0xc2b2ae35);
    h ^= h >>> 16; h = Math.imul(h, 0x7feb352d); h ^= h >>> 15; h = Math.imul(h, 0x846ca68b); h ^= h >>> 16;
    return (h >>> 0) / 4294967296;
  }
  function rng(seed) { let s = seed >>> 0; return function () { s = (s * 1664525 + 1013904223) >>> 0; return s / 4294967296; }; }

  /* ---------- Zustand des Entwurfs ---------- */
  function mode() { return store.get('cap') || 'running'; }           // running | paused | none
  function setMode(m) { store.set('cap', m); }
  const START = { h: 14, m: 36, s: 5 };                                // Beginn der Aufzeichnung
  const N0 = 48213;                                                    // Pakete beim Öffnen der Seite
  const DUR = 727.4;                                                   // Sekunden bis dahin
  const STEP = DUR / N0;
  const LOCAL = '192.0.2.20', LOCAL6 = '2001:db8::20';
  const MAC = { local: '00:00:5e:00:53:14', router: '00:00:5e:00:53:01', nas: '00:00:5e:00:53:28', printer: '00:00:5e:00:53:32', tv: '00:00:5e:00:53:3d', spk: '00:00:5e:00:53:3e' };
  const ADAPTER = { name: 'enp3s0', desc: 'Intel Ethernet Connection I219-LM' };

  /* ---------- Gegenstellen ---------- */
  const HOSTS = [
    { k: 'www', name: 'www.example.com', ip: '198.51.100.23', asn: 64498, as: 'Example CDN', cc: 'NL' },
    { k: 'video', name: 'video.example.net', ip: '203.0.113.80', asn: 64496, as: 'Example Transit', cc: 'US' },
    { k: 'cdn', name: 'cdn.example.org', ip: '198.51.100.61', asn: 64498, as: 'Example CDN', cc: 'NL' },
    { k: 'mail', name: 'mail.example.com', ip: '203.0.113.25', asn: 64500, as: 'Beispiel Mail', cc: 'DE', fav: true },
    { k: 'upd', name: 'updates.example.net', ip: '198.51.100.140', asn: 64499, as: 'Muster Cloud', cc: 'IE' },
    { k: 'nas', name: 'nas.local', ip: '192.0.2.40', tags: ['lokal'], fav: true, mac: MAC.nas },
    { k: 'api', name: 'api.example.org', ip: '203.0.113.12', asn: 64496, as: 'Example Transit', cc: 'US' },
    { k: 'v6', name: 'files.example.org', ip: '2001:db8:4::17', asn: 64499, as: 'Muster Cloud', cc: 'IE', v6: true },
    { k: 'time', name: 'time.example.net', ip: '198.51.100.123', asn: 64501, as: 'Zeitdienst e. V.', cc: 'DE' },
    { k: 'router', name: 'router.local', ip: '192.0.2.1', tags: ['lokal'], mac: MAC.router },
    { k: 'mdns', name: '224.0.0.251', ip: '224.0.0.251', tags: ['Multicast'] },
    { k: 'ssdp', name: '239.255.255.250', ip: '239.255.255.250', tags: ['Multicast'] },
    { k: 'printer', name: 'drucker.local', ip: '192.0.2.50', tags: ['lokal'], mac: MAC.printer },
    { k: 'ssh', name: 'ssh.example.org', ip: '203.0.113.44', asn: 64497, as: 'Beispiel Hosting', cc: 'DE' },
    { k: 'chat', name: 'chat.example.com', ip: '198.51.100.77', asn: 64496, as: 'Example Transit', cc: 'US' },
    { k: 'tele', name: 'telemetry.example.com', ip: '203.0.113.99', asn: 64496, as: 'Example Transit', cc: 'US' },
    { k: 'tv', name: '192.0.2.61', ip: '192.0.2.61', tags: ['lokal'], mac: MAC.tv },
    { k: 'spk', name: '192.0.2.62', ip: '192.0.2.62', tags: ['lokal'], mac: MAC.spk },
    { k: 'sip', name: 'sip.example.net', ip: '203.0.113.60', asn: 64502, as: 'Beispiel Telefonie', cc: 'DE' },
    { k: 'media', name: 'media.example.net', ip: '203.0.113.61', asn: 64502, as: 'Beispiel Telefonie', cc: 'DE' },
    { k: 'meet', name: 'meet.example.com', ip: '198.51.100.90', asn: 64496, as: 'Example Transit', cc: 'US' },
    { k: 'bcast', name: '192.0.2.255', ip: '192.0.2.255', tags: ['Broadcast'] }
  ];
  const H = {}; HOSTS.forEach(function (h) { H[h.k] = h; });
  const FAV_ADDR = ['203.0.113.44'];

  const PROGS = {
    firefox: '/usr/lib/firefox/firefox', thunderbird: '/usr/lib/thunderbird/thunderbird', 'systemd-resolved': '/usr/lib/systemd/systemd-resolved',
    ssh: '/usr/bin/ssh', 'gvfsd-smb': '/usr/libexec/gvfsd-smb', 'avahi-daemon': '/usr/sbin/avahi-daemon', 'systemd-timesyncd': '/usr/lib/systemd/systemd-timesyncd',
    NetworkManager: '/usr/sbin/NetworkManager', http: '/usr/lib/apt/methods/http', 'cups-browsed': '/usr/sbin/cups-browsed', ping: '/usr/bin/ping',
    linphone: '/usr/bin/linphone'
  };
  const FAV_PROG = ['thunderbird'];

  const CAT = { https: 'Web', http: 'Web', imaps: 'E-Mail', domain: 'Namensauflösung und Diensterkennung', mdns: 'Namensauflösung und Diensterkennung', ssdp: 'Namensauflösung und Diensterkennung', ntp: 'Netzwerkinfrastruktur', bootps: 'Netzwerkinfrastruktur', 'microsoft-ds': 'Dateiübertragung und Freigaben', ssh: 'Fernzugriff', ipp: 'Drucken und Scannen', sip: 'Telefonie' };

  /* ---------- Flüsse: [Host, Protokoll, Art, Dienst, Port, Programm, Gewicht, Anteil eingehend, Aufteilung, Schwankung] ---------- */
  const FL = [
    ['www', 'TCP', 'tls', 'https', 443, 'firefox', 13, .66, 6, [150, 0, .6]],
    ['video', 'UDP', 'quic', 'https', 443, 'firefox', 24, .8, 2, [210, 1.8, .95]],
    ['cdn', 'TCP', 'tls', 'https', 443, 'firefox', 7, .64, 4, [95, 2.4, .7]],
    ['cdn', 'UDP', 'quic', 'https', 443, 'firefox', 4, .75, 3, [120, .7, .6]],
    ['mail', 'TCP', 'imaps', 'imaps', 993, 'thunderbird', 2.2, .58, 2, [300, 3, .8]],
    ['upd', 'TCP', 'http', 'http', 80, 'http', 4, .72, 3, [260, 4.1, .9]],
    ['nas', 'TCP', 'smb', 'microsoft-ds', 445, 'gvfsd-smb', 8, .7, 1, [180, 5, .9]],
    ['api', 'TCP', 'tls', 'https', 443, 'firefox', 1.6, .55, 5, null],
    ['v6', 'TCP', 'tls', 'https', 443, 'firefox', 2.4, .7, 2, [140, 2, .8]],
    ['time', 'UDP', 'ntp', 'ntp', 123, 'systemd-timesyncd', .12, .5, 1, null],
    ['router', 'UDP', 'dns', 'domain', 53, 'systemd-resolved', 2.6, .5, 140, null],
    ['mdns', 'UDP', 'mdns', 'mdns', 5353, 'avahi-daemon', .7, .55, 1, null],
    ['tv', 'UDP', 'ssdp', 'ssdp', 1900, '', .35, 1, 1, null],
    ['spk', 'UDP', 'ssdp', 'ssdp', 1900, '', .25, 1, 1, null],
    ['router', 'ARP', 'arp', '', 0, '', .25, .5, 1, null],
    ['printer', 'TCP', 'ipp', 'ipp', 631, 'cups-browsed', .25, .6, 3, null],
    ['ssh', 'TCP', 'ssh', 'ssh', 22, 'ssh', 1.1, .45, 1, [90, 1, .5]],
    ['chat', 'TCP', 'tls', 'https', 443, 'firefox', 1.4, .5, 3, null],
    ['tele', 'TCP', 'tls', 'https', 443, 'firefox', .5, .3, 2, null],
    ['router', 'ICMP', 'icmp', '', 0, 'ping', .2, .5, 1, null],
    ['router', 'UDP', 'dhcp', 'bootps', 67, 'NetworkManager', .03, .5, 1, null]
  ];
  const FLOWS = FL.map(function (f, i) {
    return { i: i, host: H[f[0]], proto: f[1], kind: f[2], svc: f[3], port: f[4], prog: f[5], w: f[6], din: f[7], split: f[8], wave: f[9],
      lossy: f[2] === 'tls' || f[2] === 'quic' || f[2] === 'http' };
  });

  /* ---------- Verbindungen ---------- */
  const CONNS = []; const CBY = {};
  function addConn(c) { c.packets = 0; c.bytes = 0; c.inBytes = 0; c.outBytes = 0; c.first = null; c.last = null; CONNS.push(c); CBY[c.id] = c; return c; }
  FLOWS.forEach(function (f) {
    f.conns = [];
    for (let s = 0; s < f.split; s += 1) {
      const incoming = f.kind === 'ssdp';
      const sport = f.kind === 'arp' || f.kind === 'icmp' ? 0 : f.kind === 'dhcp' ? 68 : f.kind === 'ntp' ? 123 : f.kind === 'mdns' ? 5353 : 32768 + Math.floor(hash(f.i * 1000 + s, 7) * 28000);
      const v6 = !!f.host.v6;
      const c = addConn({
        id: 'c' + f.i + '-' + s, flow: f, proto: f.proto,
        src: incoming ? f.host.ip : (v6 ? LOCAL6 : LOCAL), sport: incoming ? 1900 : sport,
        dst: incoming ? '239.255.255.250' : (f.kind === 'arp' ? '192.0.2.1' : f.host.ip), dport: incoming ? 1900 : f.port,
        dir: incoming ? 'in' : 'out', svc: f.svc, prog: f.prog, host: incoming ? f.host : f.host
      });
      f.conns.push(c);
    }
  });
  // Verbindungen der Beispielgeschichten (Paketdetails und Stream folgen)
  const ST = {
    dns1: addConn({ id: 'st-dns1', flow: FLOWS[10], proto: 'UDP', src: LOCAL, sport: 41388, dst: '192.0.2.1', dport: 53, dir: 'out', svc: 'domain', prog: 'systemd-resolved', host: H.router }),
    tls: addConn({ id: 'st-tls', flow: FLOWS[0], proto: 'TCP', src: LOCAL, sport: 51544, dst: '198.51.100.23', dport: 443, dir: 'out', svc: 'https', prog: 'firefox', host: H.www }),
    dns2: addConn({ id: 'st-dns2', flow: FLOWS[10], proto: 'UDP', src: LOCAL, sport: 50211, dst: '192.0.2.1', dport: 53, dir: 'out', svc: 'domain', prog: 'systemd-resolved', host: H.router }),
    http: addConn({ id: 'st-http', flow: FLOWS[5], proto: 'TCP', src: LOCAL, sport: 39874, dst: '198.51.100.140', dport: 80, dir: 'out', svc: 'http', prog: 'http', host: H.upd })
  };

  /* ---------- VoIP (Vorhaben 5.7): SIP mit SDP, RTP, RTCP ----------
     Alles erfunden: ein Softphone (linphone) auf diesem Rechner, Anbieter sip.example.net, Sprache über media.example.net,
     dazu ein verschlüsselter Strom ohne SIP (Videokonferenz im Browser). Damit der übrige Verkehr sichtbar bleibt, ist während
     eines Anrufs nur jedes zweite Paket RTP; die Sprachpakete tragen deshalb 60 ms (a=ptime:60, im Programm meist 20 ms). */
  const PROV = '203.0.113.60', MEDIA = '203.0.113.61';
  function voConn(id, sport, dport, host, prog, svc) { return addConn({ id: id, proto: 'UDP', src: LOCAL, sport: sport, dst: host.ip, dport: dport, dir: 'out', svc: svc || '', prog: prog, host: host }); }
  const VO = { sip: voConn('vo-sip', 5060, 5060, H.sip, 'linphone', 'sip') };
  function hstr(s) { let h = 2166136261; for (let i = 0; i < s.length; i += 1) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619); } return h >>> 0; }
  const hx8 = function (v) { return '0x' + (v >>> 0).toString(16).padStart(8, '0'); };
  const ANNA = { name: 'Anna Beispiel', uri: 'sip:anna@sip.example.net' };
  // Anrufe in der Reihenfolge der Aufzeichnung; media: eigener und fremder RTP-Port, Codec (Nummer laut RFC 3551)
  const CALLS = [
    { id: 'E', out: true, rtpOnly: true, peer: { name: '', uri: '' }, media: { lport: 52714, rport: 10000, pt: 111 } },
    { id: 'C', out: true, cid: '7c1e9a40d2b3@192.0.2.20', peer: { name: '', uri: 'sip:vertrieb@example.com' }, ftag: 'a3f91c', ttag: '84c2e0d7', media: { lport: 7076, rport: 0, pt: null } },
    { id: 'A', out: true, cid: '3f2a9c1e7b54@192.0.2.20', peer: { name: '', uri: 'sip:support@example.com' }, ftag: 'c81d07', ttag: '5b9e2a61', media: { lport: 7078, rport: 31456, pt: 8 } },
    { id: 'B', out: false, cid: 'b2c7f3e9-1d4a@203.0.113.60', peer: { name: 'Beispiel GmbH', uri: 'sip:zentrale@example.org' }, ftag: 'as4f1e09c2', ttag: '9d03b7', offer: [0, 8, 101], media: { lport: 7080, rport: 40112, pt: 0 } },
    { id: 'D', out: false, cid: '0e5a8b1c-77f2@203.0.113.60', peer: { name: 'Anonymous', uri: 'sip:anonymous@anonymous.invalid' }, ftag: 'as71c2d4e0', ttag: 'e64a10', media: { lport: 0, rport: 38020, pt: null } },
    { id: 'F', out: true, cid: '91d4e6b0c3a8@192.0.2.20', peer: { name: '', uri: 'sip:technik@example.org' }, ftag: '2fe8b4', ttag: '0c7d5e93', media: { lport: 7082, rport: 35990, pt: 8 } }
  ];
  const VOREG = { id: 'R', out: true, reg: true, cid: '5d0b3e71a9f2@192.0.2.20', peer: ANNA, ftag: 'e17a3b', ttag: '1a7f0c3e', media: {} };
  const CALL = { R: VOREG }; CALLS.forEach(function (c) { CALL[c.id] = c; });
  // SIP-Nachrichten: [Paket, Anruf, Richtung, Methode oder Status, CSeq, CSeq-Methode, Extras]
  // Extras: a = mit Anmeldedaten, s = SDP (offer/answer), t = To-Tag, x = ACK zu einer Fehlerantwort
  const SIPL = [
    [300, 'R', 'out', 'REGISTER', 1, 'REGISTER', {}], [301, 'R', 'in', 401, 1, 'REGISTER', { t: 1 }],
    [302, 'R', 'out', 'REGISTER', 2, 'REGISTER', { a: 1 }], [303, 'R', 'in', 200, 2, 'REGISTER', { t: 1 }],
    [12000, 'C', 'out', 'INVITE', 1, 'INVITE', { s: 'offer' }], [12001, 'C', 'in', 407, 1, 'INVITE', { t: 1 }], [12002, 'C', 'out', 'ACK', 1, 'ACK', { t: 1, x: 1 }],
    [12003, 'C', 'out', 'INVITE', 2, 'INVITE', { s: 'offer', a: 1 }], [12004, 'C', 'in', 100, 2, 'INVITE', {}], [12010, 'C', 'in', 486, 2, 'INVITE', { t: 1 }], [12011, 'C', 'out', 'ACK', 2, 'ACK', { t: 1, x: 1 }],
    [19400, 'A', 'out', 'INVITE', 1, 'INVITE', { s: 'offer', a: 2 }], [19401, 'A', 'in', 100, 1, 'INVITE', {}], [19405, 'A', 'in', 180, 1, 'INVITE', { t: 1 }],
    [19990, 'A', 'in', 200, 1, 'INVITE', { t: 1, s: 'answer' }], [19991, 'A', 'out', 'ACK', 1, 'ACK', { t: 1 }], [26000, 'A', 'out', 'BYE', 2, 'BYE', { t: 1 }], [26001, 'A', 'in', 200, 2, 'BYE', { t: 1 }],
    [30500, 'B', 'in', 'INVITE', 1, 'INVITE', { s: 'offer' }], [30501, 'B', 'out', 100, 1, 'INVITE', {}], [30502, 'B', 'out', 180, 1, 'INVITE', { t: 1 }],
    [30800, 'B', 'out', 200, 1, 'INVITE', { t: 1, s: 'answer' }], [30801, 'B', 'in', 'ACK', 1, 'ACK', { t: 1 }], [34800, 'B', 'in', 'BYE', 2, 'BYE', { t: 1 }], [34801, 'B', 'out', 200, 2, 'BYE', { t: 1 }],
    [40000, 'D', 'in', 'INVITE', 1, 'INVITE', { s: 'offer' }], [40001, 'D', 'out', 100, 1, 'INVITE', {}], [40002, 'D', 'out', 180, 1, 'INVITE', { t: 1 }],
    [40400, 'D', 'in', 'CANCEL', 1, 'CANCEL', {}], [40401, 'D', 'out', 200, 1, 'CANCEL', { t: 1 }], [40402, 'D', 'out', 487, 1, 'INVITE', { t: 1 }], [40403, 'D', 'in', 'ACK', 1, 'ACK', { t: 1, x: 1 }],
    [46200, 'F', 'out', 'INVITE', 1, 'INVITE', { s: 'offer', a: 3 }], [46201, 'F', 'in', 100, 1, 'INVITE', {}], [46204, 'F', 'in', 180, 1, 'INVITE', { t: 1 }],
    [46480, 'F', 'in', 200, 1, 'INVITE', { t: 1, s: 'answer' }], [46481, 'F', 'out', 'ACK', 1, 'ACK', { t: 1 }]
  ];
  const REASON = { 100: 'Trying', 180: 'Ringing', 200: 'OK', 401: 'Unauthorized', 407: 'Proxy Authentication Required', 486: 'Busy Here', 487: 'Request Terminated' };
  const RTPMAP = { 0: 'PCMU/8000', 8: 'PCMA/8000', 101: 'telephone-event/8000' };
  const NONCE = { R: '6a0d93c4e1b7f258', I: '5f3a1c0e9b7d2468' };
  function sdpText(call, kind) {
    const ours = call.out === (kind === 'offer');
    const m = call.media;
    const ip = ours ? LOCAL : MEDIA, port = ours ? m.lport : m.rport;
    const pts = kind === 'offer' ? (call.offer || [8, 0, 101]) : [m.pt, 101];
    const sid = 1000 + hstr(call.id + kind) % 9000;
    return ['v=0', 'o=' + (ours ? 'anna' : '-') + ' ' + sid + ' ' + sid + ' IN IP4 ' + ip, 's=-', 'c=IN IP4 ' + ip, 't=0 0', 'm=audio ' + port + ' RTP/AVP ' + pts.join(' ')]
      .concat(pts.map(function (pt) { return 'a=rtpmap:' + pt + ' ' + RTPMAP[pt]; }), ['a=fmtp:101 0-15', 'a=ptime:60', 'a=sendrecv']).join('\r\n') + '\r\n';
  }
  function sipBuild(e) {
    const s = { n: e[0], call: e[1], dir: e[2], what: e[3], cseq: e[4], cm: e[5], x: e[6] };
    const call = CALL[s.call];
    s.req = typeof s.what === 'string';
    const fromUs = s.dir === 'out', reqFromUs = s.req ? fromUs : !fromUs;
    const user = function (u) { return u.replace(/^sip:/, '').split('@')[0]; };
    const ours = 'sip:anna@' + LOCAL + ':5060', theirs = 'sip:' + user(call.peer.uri) + '@' + PROV + ':5060';
    const caller = call.out ? ANNA : call.peer, callee = call.out ? call.peer : ANNA;
    const txm = s.cm === 'CANCEL' || (s.cm === 'ACK' && s.x.x) ? 'INVITE' : s.cm;
    const branch = 'z9hG4bK' + hstr(call.id + s.cseq + txm + (s.cm === 'ACK' && !s.x.x ? 'ack' : '')).toString(16);
    const ruri = !s.req ? '' : s.what === 'REGISTER' ? 'sip:sip.example.net' : !fromUs ? ours : (s.what === 'INVITE' || s.what === 'CANCEL' || s.x.x) ? callee.uri : theirs;
    s.from = (caller.name ? '"' + caller.name + '" ' : '') + '<' + caller.uri + '>';
    s.to = '<' + callee.uri + '>';
    const H = [];
    H.push(['Via', 'SIP/2.0/UDP ' + (reqFromUs ? LOCAL : PROV) + ':5060;branch=' + branch + (reqFromUs ? ';rport' : '')]);
    if (s.req) H.push(['Max-Forwards', '70']);
    H.push(['From', s.from + ';tag=' + call.ftag], ['To', s.to + (s.x.t ? ';tag=' + call.ttag : '')], ['Call-ID', call.cid], ['CSeq', s.cseq + ' ' + s.cm]);
    if (s.what === 'INVITE' || s.what === 'REGISTER' || (!s.req && s.what === 200 && (s.cm === 'INVITE' || s.cm === 'REGISTER')) || s.what === 180) {
      const mine = s.req ? fromUs : fromUs;
      H.push(['Contact', '<' + (mine ? ours : theirs) + '>' + (s.cm === 'REGISTER' ? ';expires=3600' : '')]);
    }
    if (s.x.a) {
      const reg = s.what === 'REGISTER';
      const nc = ('0000000' + s.x.a).slice(-8);
      H.push([reg ? 'Authorization' : 'Proxy-Authorization', 'Digest username="anna", realm="sip.example.net", nonce="' + (reg ? NONCE.R : NONCE.I) + '", uri="' + ruri + '", response="' +
        hstr(call.id + s.cseq + 'r1').toString(16).padStart(8, '0') + hstr(call.id + 'r2').toString(16).padStart(8, '0') + hstr(s.cseq + call.id + 'r3').toString(16).padStart(8, '0') + hstr('r4' + call.id).toString(16).padStart(8, '0') +
        '", algorithm=MD5, cnonce="' + hstr(call.id + 'cn').toString(16).padStart(8, '0') + '", qop=auth, nc=' + nc]);
    }
    if (s.what === 401) H.push(['WWW-Authenticate', 'Digest realm="sip.example.net", nonce="' + NONCE.R + '", algorithm=MD5, qop="auth"']);
    if (s.what === 407) H.push(['Proxy-Authenticate', 'Digest realm="sip.example.net", nonce="' + NONCE.I + '", algorithm=MD5, qop="auth"']);
    if (s.what === 'REGISTER') H.push(['Expires', '3600']);
    if (s.what === 'INVITE' || (s.what === 200 && s.cm === 'INVITE')) H.push(['Allow', 'INVITE, ACK, CANCEL, BYE, OPTIONS, INFO, UPDATE, NOTIFY']);
    H.push(fromUs ? ['User-Agent', 'Linphone/5.2 (belle-sip)'] : [s.req ? 'User-Agent' : 'Server', 'Beispiel-SBC/3.1']);
    const sdp = s.x.s ? sdpText(call, s.x.s) : '';
    if (sdp) H.push(['Content-Type', 'application/sdp']);
    H.push(['Content-Length', String(sdp.length)]);
    const first = s.req ? s.what + ' ' + ruri + ' SIP/2.0' : 'SIP/2.0 ' + s.what + ' ' + REASON[s.what];
    s.text = [first].concat(H.map(function (h) { return h[0] + ': ' + h[1]; })).join('\r\n') + '\r\n\r\n' + sdp;
    s.sdp = !!sdp; s.ruri = ruri; s.cid = call.cid; s.method = s.req ? s.what : null; s.status = s.req ? null : s.what;
    s.info = (s.req ? 'Request: ' + s.what + ' ' + ruri : 'Status: ' + s.what + ' ' + REASON[s.what] + ' (' + s.cm + ')') + (sdp ? ', with session description' : '');
    s.label = (s.req ? s.what : s.what + ' ' + REASON[s.what]) + (sdp ? ' (SDP)' : '');
    return s;
  }
  const SIPN = {};
  SIPL.forEach(function (e) { const s = sipBuild(e); SIPN[s.n] = s; const c = CALL[s.call]; (c.sip = c.sip || []).push(s.n); });
  // Bezüge: Anfrage ↔ Antworten derselben Transaktion
  SIPL.forEach(function (e) {
    const s = SIPN[e[0]]; s.rels = [];
    CALL[s.call].sip.forEach(function (m) {
      const o = SIPN[m]; if (o.cseq !== s.cseq || o.cm !== s.cm || o.req === s.req) return;
      const dt = time(Math.max(m, s.n)) - time(Math.min(m, s.n));
      const ms = dt >= 1 ? dt.toFixed(1).replace('.', ',') + ' s' : (dt * 1000).toFixed(1).replace('.', ',') + ' ms';
      if (s.req && m > s.n) s.rels.push(['→ Antwort ' + o.what + ' in Paket', m, ms]);
      if (!s.req && m < s.n) s.rels.push(['← Antwort auf Paket', m, ms]);
    });
  });
  // RTP-Ströme je Anruf und Richtung (o = gesendet, i = empfangen). jit: Jitter Ø/max in ms, gap: größter Abstand in ms
  const RTPS = [
    { call: 'E', conn: voConn('vo-srtp', 52714, 10000, H.meet, 'firefox'), rtcp: null, from: 5000, to: 11000, every: 3, pt: 111, step: 4320, enc: true,
      o: { ssrc: 0x9b2e7714, seq0: 40981, ts0: 2830080, gap: 97.3 }, i: { ssrc: 0xc40a3f86, seq0: 712, ts0: 640320, gap: 131.5, loss: 0.003, ooo: 0 } },
    { call: 'A', conn: voConn('vo-rtp-a', 7078, 31456, H.media, 'linphone'), rtcp: voConn('vo-rtcp-a', 7079, 31457, H.media, 'linphone'), from: 19992, to: 26000, every: 2, pt: 8, step: 480, setup: 19990,
      o: { ssrc: 0x5e1f02a7, seq0: 21043, ts0: 1617600, jit: [0.4, 1.9], gap: 62.1 }, i: { ssrc: 0x0b77c3d1, seq0: 3310, ts0: 92400160, jit: [3.1, 9.8], gap: 79.4 } },
    { call: 'B', conn: voConn('vo-rtp-b', 7080, 40112, H.media, 'linphone'), rtcp: voConn('vo-rtcp-b', 7081, 40113, H.media, 'linphone'), from: 30802, to: 34800, every: 2, pt: 0, step: 480, setup: 30800,
      o: { ssrc: 0x2c90e415, seq0: 11872, ts0: 30411520, jit: [0.5, 2.2], gap: 63.0 }, i: { ssrc: 0x7a3361f0, seq0: 50210, ts0: 7718400, jit: [41.7, 183.2], gap: 412.6, loss: 0.04, ooo: 3 } },
    { call: 'F', conn: voConn('vo-rtp-f', 7082, 35990, H.media, 'linphone'), rtcp: voConn('vo-rtcp-f', 7083, 35991, H.media, 'linphone'), from: 46482, to: null, every: 2, pt: 8, step: 480, setup: 46480,
      o: { ssrc: 0x41d8a9e2, seq0: 6021, ts0: 551040, jit: [0.4, 1.7], gap: 61.8 }, i: { ssrc: 0x1f6b0c58, seq0: 28877, ts0: 13104000, jit: [2.4, 7.1], gap: 70.2 } }
  ];
  RTPS.forEach(function (R) {
    CALL[R.call].rtp = R;
    [R.o, R.i].forEach(function (side, k) {
      if (!side.loss) return;
      const a = [0]; let L = 0;   // a[i]: verlorene Pakete vor dem i-ten empfangenen
      for (let i = 1; i <= 2000; i += 1) { const r = hash(i, 90 + k); if (r < side.loss) L += r < side.loss * 0.3 ? 2 : 1; a.push(L); }
      side.lost = a;
    });
  });
  const PTN = { 0: 'ITU-T G.711 PCMU', 8: 'ITU-T G.711 PCMA', 111: 'dynamic (111)' };
  function voip(n) {
    const s = SIPN[n];
    if (s) {
      const p = mk(n, time(n), VO.sip, { dir: s.dir, len: 42 + s.text.length, proto: s.sdp ? 'SIP/SDP' : 'SIP', cls: 'voip', kind: 'sip', info: s.info });
      p.sip = s; return p;
    }
    for (let r = 0; r < RTPS.length; r += 1) {
      const R = RTPS[r];
      if (n < R.from || (R.to != null && n >= R.to)) continue;
      const off = n - R.from;
      if (off % R.every === 0) {
        const k = off / R.every, out = k % 2 === 0, i = k >> 1, side = out ? R.o : R.i;
        const lost = side.lost ? side.lost[Math.min(i, side.lost.length - 1)] : 0;
        const seq = (side.seq0 + i + lost) & 0xffff, ts = (side.ts0 + (i + lost) * R.step) >>> 0;
        const plen = R.enc ? 70 + Math.floor(hash(n, 41) * 110) : R.step;
        const p = mk(n, time(n), R.conn, { dir: out ? 'out' : 'in', len: 54 + plen, proto: 'RTP', cls: 'voip', kind: 'rtp',
          info: 'PT=' + PTN[R.pt] + ', SSRC=' + hx8(side.ssrc).toUpperCase().replace('0X', '0x') + ', Seq=' + seq + ', Time=' + ts + (i === 0 ? ', Mark' : '') });
        p.rtp = { call: R.call, R: R, out: out, i: i, lost: lost, ssrc: side.ssrc, seq: seq, ts: ts, pt: R.pt, mark: i === 0, plen: plen };
        return p;
      }
      if (R.rtcp && off % 334 === 1) {
        const m = (off - 1) / 334, out = m % 2 === 0;
        const p = mk(n, time(n), R.rtcp, { dir: out ? 'out' : 'in', len: 42 + 52 + 28, proto: 'RTCP', cls: 'voip', kind: 'rtcp', info: 'Sender Report, Source Description' });
        p.rtcp = { call: R.call, R: R, out: out, m: m, ssrc: (out ? R.o : R.i).ssrc, about: (out ? R.i : R.o).ssrc };
        return p;
      }
    }
    return null;
  }

  /* ---------- Pakete ---------- */
  function time(n) { return (n - 1 + 0.85 * hash(n, 1)) * STEP; }
  function weightAt(f, t) {
    if (!f.wave) return f.w;
    return f.w * Math.max(0.04, 1 + f.wave[2] * Math.sin(2 * Math.PI * t / f.wave[0] + f.wave[1]));
  }
  function pick(n, t) {
    let sum = 0; const ws = FLOWS.map(function (f) { const w = weightAt(f, t); sum += w; return w; });
    let u = hash(n, 2) * sum;
    for (let i = 0; i < ws.length; i += 1) { u -= ws[i]; if (u <= 0) return FLOWS[i]; }
    return FLOWS[0];
  }
  const DNSN = ['www.example.com', 'cdn.example.org', 'video.example.net', 'api.example.org', 'chat.example.com', 'telemetry.example.com', 'mail.example.com', 'updates.example.net', 'files.example.org', 'time.example.net', 'fonts.example.com', 'img.example.net'];
  const IPOF = {}; HOSTS.forEach(function (h) { IPOF[h.name] = h.ip; }); IPOF['fonts.example.com'] = '198.51.100.24'; IPOF['img.example.net'] = '203.0.113.81';
  const hex4 = function (v) { return '0x' + (v & 0xffff).toString(16).padStart(4, '0'); };

  function mk(n, t, c, o) {
    const out = o.dir === 'out';
    const p = { n: n, t: t, conn: c.id, dir: o.dir, len: o.len + (c.host.v6 && o.len < 1500 ? 20 : 0), proto: o.proto, info: o.info, cls: o.cls || '', an: o.an || '', name: o.name || '', sni: o.sni || '', method: o.method || '', flags: o.flags || '', kind: o.kind };
    if (c.proto === 'ARP') { p.src = out ? MAC.local : MAC.router; p.dst = o.bcast ? 'Broadcast' : (out ? MAC.router : MAC.local); p.ipa = [LOCAL, '192.0.2.1']; }
    else if (c.dir === 'in') { p.src = c.src; p.dst = c.dst; p.sport = c.sport; p.dport = c.dport; p.ipa = [c.src, c.dst]; }
    else {
      p.src = out ? c.src : c.dst; p.dst = out ? c.dst : c.src;
      p.sport = out ? c.sport : c.dport; p.dport = out ? c.dport : c.sport; p.ipa = [c.src, c.dst];
      if (o.mreply) { p.src = H.printer.ip; p.dst = '224.0.0.251'; p.ipa = [p.src, p.dst]; }
    }
    return p;
  }

  function gen(n) {
    const v = voip(n); if (v) return v;
    if (n >= 1200 && n <= 1220) return story(n);
    if (n >= 2400 && n <= 2413) return story(n);
    const t = time(n);
    const f = pick(n, t);
    const W = DUR / f.split;
    const s = f.split > 1 ? Math.min(f.split - 1, Math.floor((t % DUR) / W)) : 0;
    const c = f.conns[s];
    const r = hash(n, 4), r2 = hash(n, 5), r3 = hash(n, 6);
    const din = r2 < f.din;
    const sp = c.sport, dp = c.dport;
    const seq = 1 + Math.floor(hash(n, 8) * 900000), ack = 1 + Math.floor(hash(n, 9) * 90000);
    const win = 400 + Math.floor(hash(n, 10) * 600);
    const arrow = function (o) { return o ? sp + ' → ' + dp : dp + ' → ' + sp; };
    switch (f.kind) {
      case 'tls': case 'imaps': case 'ssh': case 'smb': case 'http': case 'ipp': {
        if (r < 0.010) {
          const k = Math.floor(r3 * 3);
          if (k === 0) return mk(n, t, c, { dir: 'out', len: 74, proto: 'TCP', cls: 'tcp', flags: 'SYN', info: arrow(true) + ' [SYN] Seq=0 Win=64240 Len=0 MSS=1460 SACK_PERM WS=128', kind: f.kind });
          if (k === 1) return mk(n, t, c, { dir: 'in', len: 74, proto: 'TCP', cls: 'tcp', flags: 'SYN ACK', info: arrow(false) + ' [SYN, ACK] Seq=0 Ack=1 Win=65160 Len=0 MSS=1460 SACK_PERM WS=1024', kind: f.kind });
          return mk(n, t, c, { dir: din ? 'in' : 'out', len: 66, proto: 'TCP', cls: 'tcp', flags: 'FIN ACK', info: arrow(!din) + ' [FIN, ACK] Seq=' + seq + ' Ack=' + ack + ' Win=' + win + ' Len=0', kind: f.kind });
        }
        if (f.lossy && r < 0.0175) {
          if (r3 < 0.55) return mk(n, t, c, { dir: 'in', len: 1514, proto: 'TCP', cls: 'bad', an: 'Retransmission', info: '[TCP Retransmission] ' + arrow(false) + ' [ACK] Seq=' + seq + ' Ack=' + ack + ' Win=' + win + ' Len=1448', kind: f.kind });
          if (r3 < 0.9) return mk(n, t, c, { dir: 'out', len: 66, proto: 'TCP', cls: 'bad', an: 'Dup ACK', info: '[TCP Dup ACK ' + (n - 3) + '#1] ' + arrow(true) + ' [ACK] Seq=' + ack + ' Ack=' + seq + ' Win=' + win + ' Len=0', kind: f.kind });
          return mk(n, t, c, { dir: 'in', len: 1514, proto: 'TCP', cls: 'bad', an: 'Out-Of-Order', info: '[TCP Out-Of-Order] ' + arrow(false) + ' [ACK] Seq=' + seq + ' Ack=' + ack + ' Win=' + win + ' Len=1448', kind: f.kind });
        }
        if (f.host.k === 'tele' && r < 0.03) return mk(n, t, c, { dir: 'in', len: 66, proto: 'TCP', cls: 'bad', an: 'RST', flags: 'RST ACK', info: arrow(false) + ' [RST, ACK] Seq=' + seq + ' Ack=' + ack + ' Win=0 Len=0', kind: f.kind });
        if (!din && r3 < 0.68) return mk(n, t, c, { dir: 'out', len: 66, proto: 'TCP', info: arrow(true) + ' [ACK] Seq=' + ack + ' Ack=' + seq + ' Win=' + win + ' Len=0', kind: f.kind });
        const big = din ? (hash(n, 11) < 0.8 ? 1514 : 120 + Math.floor(hash(n, 11) * 1390)) : 90 + Math.floor(hash(n, 11) * 520);
        if (f.kind === 'tls' || f.kind === 'imaps') return mk(n, t, c, { dir: din ? 'in' : 'out', len: big, proto: f.kind === 'imaps' ? 'TLSv1.2' : 'TLSv1.3', cls: 'tls', info: 'Application Data', kind: 'tls-data', sni: c.host.name });
        if (f.kind === 'ssh') return mk(n, t, c, { dir: din ? 'in' : 'out', len: din ? 118 + Math.floor(hash(n, 11) * 900) : 102, proto: 'SSHv2', info: (din ? 'Server' : 'Client') + ': Encrypted packet (len=' + ((din ? 52 + Math.floor(hash(n, 11) * 900) : 36)) + ')', kind: 'ssh' });
        if (f.kind === 'smb') return din ? mk(n, t, c, { dir: 'in', len: 1514, proto: 'SMB2', info: 'Read Response, File: Projekte\\Bericht.pdf', kind: 'smb' }) : mk(n, t, c, { dir: 'out', len: 171, proto: 'SMB2', info: 'Read Request Len:1048576 Off:' + (Math.floor(hash(n, 12) * 400) * 1048576) + ' File: Projekte\\Bericht.pdf', kind: 'smb' });
        if (f.kind === 'ipp') return din ? mk(n, t, c, { dir: 'in', len: 612, proto: 'IPP', cls: 'http', info: 'IPP Response (successful-ok)', kind: 'ipp' }) : mk(n, t, c, { dir: 'out', len: 330, proto: 'IPP', cls: 'http', info: 'IPP Request (Get-Printer-Attributes)', kind: 'ipp', method: 'POST' });
        if (din && r3 < 0.06) return mk(n, t, c, { dir: 'in', len: 942, proto: 'HTTP', cls: 'http', info: 'HTTP/1.1 200 OK (application/x-xz)', kind: 'http-resp', name: c.host.name });
        if (!din && r3 < 0.85) return mk(n, t, c, { dir: 'out', len: 236, proto: 'HTTP', cls: 'http', method: 'GET', info: 'GET /dists/stable/main/binary-amd64/Packages.xz HTTP/1.1', kind: 'http-req', name: c.host.name });
        return mk(n, t, c, { dir: 'in', len: 1514, proto: 'TCP', info: arrow(false) + ' [ACK] Seq=' + seq + ' Ack=' + ack + ' Win=' + win + ' Len=1448 [TCP segment of a reassembled PDU]', kind: 'tcp-seg' });
      }
      case 'quic': {
        if (f.lossy && r < 0.004) return mk(n, t, c, { dir: 'in', len: 1292, proto: 'QUIC', cls: 'udp', info: 'Protected Payload (KP0), DCID=' + (c.sport).toString(16) + 'c1e0a7', kind: 'quic' });
        return mk(n, t, c, { dir: din ? 'in' : 'out', len: din ? 1292 : 60 + Math.floor(hash(n, 11) * 40), proto: 'QUIC', cls: 'udp', info: 'Protected Payload (KP0)' + (din ? '' : ', DCID=' + (c.sport).toString(16) + 'c1e0a7'), kind: 'quic' });
      }
      case 'dns': {
        const name = DNSN[Math.floor(hash(n, 13) * DNSN.length)];
        const id = Math.floor(hash(Math.floor(n / 2), 14) * 65535);
        const aaaa = hash(n, 15) < 0.3;
        if (!din) return mk(n, t, c, { dir: 'out', len: 70 + name.length, proto: 'DNS', cls: 'dns', name: name, info: 'Standard query ' + hex4(id) + ' ' + (aaaa ? 'AAAA ' : 'A ') + name, kind: 'dns-q' });
        return mk(n, t, c, { dir: 'in', len: 86 + name.length, proto: 'DNS', cls: 'dns', name: name, info: 'Standard query response ' + hex4(id) + ' ' + (aaaa ? 'AAAA ' + name + ' AAAA 2001:db8:4::' + (10 + Math.floor(hash(n, 16) * 80)) : 'A ' + name + ' A ' + (IPOF[name] || '198.51.100.9')), kind: 'dns-r' });
      }
      case 'mdns':
        if (din) return mk(n, t, c, { dir: 'in', len: 152, proto: 'MDNS', cls: 'dns', mreply: true, name: 'drucker._ipp._tcp.local', info: 'Standard query response 0x0000 PTR drucker._ipp._tcp.local SRV TXT A 192.0.2.50', kind: 'mdns' });
        return mk(n, t, c, { dir: 'out', len: 87, proto: 'MDNS', cls: 'dns', name: '_ipp._tcp.local', info: 'Standard query 0x0000 PTR _ipp._tcp.local, "QM" question', kind: 'mdns' });
      case 'ssdp': return mk(n, t, c, { dir: 'in', len: 340 + Math.floor(hash(n, 11) * 80), proto: 'SSDP', cls: 'local', info: 'NOTIFY * HTTP/1.1', kind: 'ssdp' });
      case 'arp':
        if (!din) return mk(n, t, c, { dir: 'out', len: 42, proto: 'ARP', cls: 'local', bcast: true, info: 'Who has 192.0.2.1? Tell ' + LOCAL, kind: 'arp' });
        return mk(n, t, c, { dir: 'in', len: 60, proto: 'ARP', cls: 'local', info: '192.0.2.1 is at ' + MAC.router, kind: 'arp' });
      case 'ntp': return mk(n, t, c, { dir: din ? 'in' : 'out', len: 90, proto: 'NTP', cls: 'local', info: 'NTP Version 4, ' + (din ? 'server' : 'client'), kind: 'ntp' });
      case 'dhcp': return mk(n, t, c, { dir: din ? 'in' : 'out', len: 342, proto: 'DHCP', cls: 'local', info: (din ? 'DHCP ACK      ' : 'DHCP Request  ') + '- Transaction ID 0x6a1c2f40', kind: 'dhcp' });
      default: {
        const sq = Math.floor(t / 1) + 1;
        return mk(n, t, c, { dir: din ? 'in' : 'out', len: 98, proto: 'ICMP', cls: 'icmp', info: 'Echo (ping) ' + (din ? 'reply  ' : 'request') + ' id=0x0007, seq=' + sq + '/' + (sq * 256) + ', ttl=' + (din ? 64 : 64), kind: 'icmp' });
      }
    }
  }

  /* Beispielgeschichten: Verbindungsaufbau mit TLS (1200–1220) und unverschlüsseltes HTTP (2400–2413) */
  function story(n) {
    const t = time(n);
    const a = ST.tls, h = ST.http;
    const T = function (c, o) { return mk(n, t, c, o); };
    switch (n) {
      case 1200: return T(ST.dns1, { dir: 'out', len: 75, proto: 'DNS', cls: 'dns', name: 'www.example.com', info: 'Standard query 0x3f21 A www.example.com', kind: 'dns-q', qid: 0x3f21 });
      case 1201: return T(ST.dns1, { dir: 'in', len: 91, proto: 'DNS', cls: 'dns', name: 'www.example.com', info: 'Standard query response 0x3f21 A www.example.com A 198.51.100.23', kind: 'dns-r', qid: 0x3f21 });
      case 1202: return T(a, { dir: 'out', len: 74, proto: 'TCP', cls: 'tcp', flags: 'SYN', info: '51544 → 443 [SYN] Seq=0 Win=64240 Len=0 MSS=1460 SACK_PERM WS=128', kind: 'syn' });
      case 1203: return T(a, { dir: 'in', len: 74, proto: 'TCP', cls: 'tcp', flags: 'SYN ACK', info: '443 → 51544 [SYN, ACK] Seq=0 Ack=1 Win=65160 Len=0 MSS=1460 SACK_PERM WS=1024', kind: 'synack' });
      case 1204: return T(a, { dir: 'out', len: 66, proto: 'TCP', info: '51544 → 443 [ACK] Seq=1 Ack=1 Win=502 Len=0', kind: 'ack' });
      case 1205: return T(a, { dir: 'out', len: 583, proto: 'TLSv1.3', cls: 'tls', sni: 'www.example.com', info: 'Client Hello (SNI=www.example.com)', kind: 'clienthello' });
      case 1206: return T(a, { dir: 'in', len: 66, proto: 'TCP', info: '443 → 51544 [ACK] Seq=1 Ack=518 Win=64 Len=0', kind: 'ack' });
      case 1207: return T(a, { dir: 'in', len: 1514, proto: 'TLSv1.3', cls: 'tls', info: 'Server Hello, Change Cipher Spec, Application Data', kind: 'tls-data' });
      case 1208: return T(a, { dir: 'in', len: 1514, proto: 'TLSv1.3', cls: 'tls', info: 'Application Data', kind: 'tls-data' });
      case 1209: return T(a, { dir: 'out', len: 66, proto: 'TCP', info: '51544 → 443 [ACK] Seq=518 Ack=2897 Win=501 Len=0', kind: 'ack' });
      case 1210: return T(a, { dir: 'out', len: 130, proto: 'TLSv1.3', cls: 'tls', info: 'Change Cipher Spec, Application Data', kind: 'tls-data' });
      case 1211: return T(a, { dir: 'out', len: 412, proto: 'TLSv1.3', cls: 'tls', info: 'Application Data', kind: 'tls-data' });
      case 1212: return T(a, { dir: 'in', len: 1514, proto: 'TLSv1.3', cls: 'tls', info: 'Application Data', kind: 'tls-data' });
      case 1213: return T(a, { dir: 'in', len: 1514, proto: 'TLSv1.3', cls: 'tls', info: 'Application Data', kind: 'tls-data' });
      case 1214: return T(a, { dir: 'out', len: 66, proto: 'TCP', info: '51544 → 443 [ACK] Seq=928 Ack=5793 Win=501 Len=0', kind: 'ack' });
      case 1215: return T(a, { dir: 'in', len: 1514, proto: 'TLSv1.3', cls: 'tls', info: 'Application Data', kind: 'tls-data' });
      case 1216: return T(a, { dir: 'out', len: 78, proto: 'TCP', cls: 'bad', an: 'Dup ACK', info: '[TCP Dup ACK 1214#1] 51544 → 443 [ACK] Seq=928 Ack=5793 Win=501 Len=0 SLE=7241 SRE=8689', kind: 'ack' });
      case 1217: return T(a, { dir: 'in', len: 1514, proto: 'TCP', cls: 'bad', an: 'Retransmission', info: '[TCP Retransmission] 443 → 51544 [ACK] Seq=5793 Ack=928 Win=64 Len=1448', kind: 'retrans' });
      case 1218: return T(a, { dir: 'out', len: 66, proto: 'TCP', info: '51544 → 443 [ACK] Seq=928 Ack=8689 Win=501 Len=0', kind: 'ack' });
      case 1219: return T(a, { dir: 'in', len: 1514, proto: 'TLSv1.3', cls: 'tls', info: 'Application Data', kind: 'tls-data' });
      case 1220: return T(a, { dir: 'out', len: 66, proto: 'TCP', info: '51544 → 443 [ACK] Seq=928 Ack=10137 Win=501 Len=0', kind: 'ack' });
      case 2400: return T(ST.dns2, { dir: 'out', len: 79, proto: 'DNS', cls: 'dns', name: 'updates.example.net', info: 'Standard query 0x91b4 A updates.example.net', kind: 'dns-q', qid: 0x91b4 });
      case 2401: return T(ST.dns2, { dir: 'in', len: 95, proto: 'DNS', cls: 'dns', name: 'updates.example.net', info: 'Standard query response 0x91b4 A updates.example.net A 198.51.100.140', kind: 'dns-r', qid: 0x91b4 });
      case 2402: return T(h, { dir: 'out', len: 74, proto: 'TCP', cls: 'tcp', flags: 'SYN', info: '39874 → 80 [SYN] Seq=0 Win=64240 Len=0 MSS=1460 SACK_PERM WS=128', kind: 'syn' });
      case 2403: return T(h, { dir: 'in', len: 74, proto: 'TCP', cls: 'tcp', flags: 'SYN ACK', info: '80 → 39874 [SYN, ACK] Seq=0 Ack=1 Win=65160 Len=0 MSS=1460 SACK_PERM WS=512', kind: 'synack' });
      case 2404: return T(h, { dir: 'out', len: 66, proto: 'TCP', info: '39874 → 80 [ACK] Seq=1 Ack=1 Win=502 Len=0', kind: 'ack' });
      case 2405: return T(h, { dir: 'out', len: 66 + HTTP_REQ.length, proto: 'HTTP', cls: 'http', method: 'GET', name: 'updates.example.net', info: 'GET /dists/stable/InRelease HTTP/1.1', kind: 'http-story-req' });
      case 2406: return T(h, { dir: 'in', len: 66, proto: 'TCP', info: '80 → 39874 [ACK] Seq=1 Ack=' + (1 + HTTP_REQ.length) + ' Win=126 Len=0', kind: 'ack' });
      case 2407: return T(h, { dir: 'in', len: 1514, proto: 'TCP', info: '80 → 39874 [PSH, ACK] Seq=1 Ack=' + (1 + HTTP_REQ.length) + ' Win=126 Len=1448 [TCP segment of a reassembled PDU]', kind: 'http-story-seg' });
      case 2408: return T(h, { dir: 'out', len: 66, proto: 'TCP', info: '39874 → 80 [ACK] Seq=' + (1 + HTTP_REQ.length) + ' Ack=1449 Win=501 Len=0', kind: 'ack' });
      case 2409: return T(h, { dir: 'in', len: 66 + HTTP_RESP.length - 1448, proto: 'HTTP', cls: 'http', name: 'updates.example.net', info: 'HTTP/1.1 200 OK (text/plain)', kind: 'http-story-resp' });
      case 2410: return T(h, { dir: 'out', len: 66, proto: 'TCP', info: '39874 → 80 [ACK] Seq=' + (1 + HTTP_REQ.length) + ' Ack=' + (1 + HTTP_RESP.length) + ' Win=501 Len=0', kind: 'ack' });
      case 2411: return T(h, { dir: 'out', len: 66, proto: 'TCP', cls: 'tcp', flags: 'FIN ACK', info: '39874 → 80 [FIN, ACK] Seq=' + (1 + HTTP_REQ.length) + ' Ack=' + (1 + HTTP_RESP.length) + ' Win=501 Len=0', kind: 'fin' });
      case 2412: return T(h, { dir: 'in', len: 66, proto: 'TCP', cls: 'tcp', flags: 'FIN ACK', info: '80 → 39874 [FIN, ACK] Seq=' + (1 + HTTP_RESP.length) + ' Ack=' + (2 + HTTP_REQ.length) + ' Win=126 Len=0', kind: 'fin' });
      default: return T(h, { dir: 'out', len: 66, proto: 'TCP', info: '39874 → 80 [ACK] Seq=' + (2 + HTTP_REQ.length) + ' Ack=' + (2 + HTTP_RESP.length) + ' Win=501 Len=0', kind: 'ack' });
    }
  }
  function clock(t, ms) {
    let s = START.h * 3600 + START.m * 60 + START.s + t;
    const hh = Math.floor(s / 3600) % 24, mm = Math.floor(s / 60) % 60, ss = Math.floor(s) % 60;
    const pad = function (v) { return String(v).padStart(2, '0'); };
    return pad(hh) + ':' + pad(mm) + ':' + pad(ss) + (ms ? ',' + String(Math.floor((s % 1) * 1e6)).padStart(6, '0') : '');
  }
  const HTTP_REQ = 'GET /dists/stable/InRelease HTTP/1.1\r\nHost: updates.example.net\r\nUser-Agent: Debian APT-HTTP/1.3 (2.9.8)\r\nAccept: text/*\r\nCache-Control: max-age=0\r\n\r\n';
  const HTTP_BODY = 'Origin: Example\nLabel: Example\nSuite: stable\nCodename: example\nDate: Mon, 09 Nov 2026 08:12:40 UTC\nArchitectures: amd64 arm64\nComponents: main\nDescription: Example package source (invented for this draft)\nSHA256:\n' +
    [['4f0c1a9e2b7d', 1342, 'main/binary-amd64/Packages.xz'], ['9a3e77c0d512', 41877, 'main/binary-amd64/Packages'], ['c6d20b8e4f19', 1290, 'main/binary-arm64/Packages.xz'], ['1be47a90c3d8', 40112, 'main/binary-arm64/Packages'], ['7d11e5a2b6c0', 2214, 'main/i18n/Translation-en.xz']].map(function (x) {
      let hx = ''; const r = rng(parseInt(x[0], 16) % 100000); for (let i = 0; i < 64; i += 1) hx += Math.floor(r() * 16).toString(16);
      return ' ' + hx + ' ' + String(x[1]).padStart(8) + ' ' + x[2];
    }).join('\n') + '\n' + 'Valid-Until: Mon, 16 Nov 2026 08:12:40 UTC\n' + 'X-Note: This content is invented and only serves as an example for "Follow stream".\n' + '#'.repeat(24) + '\n' +
    'Changelogs: https://updates.example.net/changelogs/@CHANGEPATH@_@VERSION@_changelog\nAcquire-By-Hash: yes\n' + 'Signed-By: 0000 0000 0000 0000 0000  0000 0000 0000 0000 0000\n' +
    'Notes:\n' + [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12].map(function (i) { return ' Note ' + i + ': packages of this source are rebuilt daily at 08:00 UTC; the checksums above apply to that day.'; }).join('\n') + '\n';
  const HTTP_RESP = 'HTTP/1.1 200 OK\r\nDate: Mon, 09 Nov 2026 13:36:41 GMT\r\nServer: example-httpd\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: ' + HTTP_BODY.length + '\r\nLast-Modified: Mon, 09 Nov 2026 08:12:40 GMT\r\nETag: "83b-5f1c2a7e"\r\nAccept-Ranges: bytes\r\n\r\n' + HTTP_BODY;

  /* ---------- Zusammenzählen ---------- */
  let N = mode() === 'none' ? 0 : N0;
  const SEC = []; // je Sekunde [ein, aus, Pakete]
  const ANALYSIS = { 'Retransmission': 0, 'Dup ACK': 0, 'Out-Of-Order': 0, 'RST': 0 };
  const PROTO = {};
  let TOTAL = { bytes: 0, inB: 0, outB: 0, packets: 0 };
  function account(p) {
    const c = CBY[p.conn];
    c.packets += 1; c.bytes += p.len; if (p.dir === 'in') c.inBytes += p.len; else c.outBytes += p.len;
    if (c.first == null) c.first = p.t; c.last = p.t;
    const s = Math.floor(p.t);
    if (!SEC[s]) SEC[s] = [0, 0, 0];
    SEC[s][p.dir === 'in' ? 0 : 1] += p.len; SEC[s][2] += 1;
    TOTAL.bytes += p.len; TOTAL.packets += 1; if (p.dir === 'in') TOTAL.inB += p.len; else TOTAL.outB += p.len;
    if (p.an) ANALYSIS[p.an] = (ANALYSIS[p.an] || 0) + 1;
  }
  const PK = [null];                    // Pakete im Speicher des Entwurfs (wie der Paketspeicher der Anwendung)
  for (let n = 1; n <= N; n += 1) { PK[n] = gen(n); account(PK[n]); }
  function grow(k) { for (let i = 0; i < k; i += 1) { N += 1; PK[N] = gen(N); account(PK[N]); } return N; }
  function pkt(n) { return PK[n] || gen(n); }

  function sumBy(key) {
    const m = {};
    CONNS.forEach(function (c) {
      if (!c.packets) return;
      const k = key(c); if (k == null) return;
      if (!m[k]) m[k] = { key: k, inB: 0, outB: 0, bytes: 0, packets: 0, conns: 0, last: 0 };
      const e = m[k]; e.inB += c.inBytes; e.outB += c.outBytes; e.bytes += c.bytes; e.packets += c.packets; e.conns += 1; e.last = Math.max(e.last, c.last);
    });
    return Object.keys(m).map(function (k) { return m[k]; });
  }
  function hostsAgg() { return sumBy(function (c) { return c.host.k; }).map(function (e) { e.host = H[e.key]; return e; }); }
  function servicesAgg() { return sumBy(function (c) { return c.svc ? c.svc + '/' + (c.proto === 'TCP' ? 'tcp' : 'udp') : null; }).map(function (e) { const s = e.key.split('/'); e.name = s[0]; e.tp = s[1]; e.port = (CONNS.find(function (c) { return c.svc === s[0]; }) || {}).dport; e.cat = CAT[s[0]] || 'Sonstige'; return e; }); }
  function programsAgg() { return sumBy(function (c) { return c.prog || 'unbekannt'; }).map(function (e) { e.path = PROGS[e.key] || ''; return e; }); }

  /* ---------- Formate ---------- */
  function fnum(v, dg) { return Number(v).toLocaleString('de-DE', { maximumFractionDigits: dg == null ? 0 : dg, minimumFractionDigits: dg || 0 }); }
  function fbytes(b) {
    if (b < 1000) return fnum(b) + ' B';
    const u = ['KB', 'MB', 'GB', 'TB']; let i = -1; let v = b;
    do { v /= 1000; i += 1; } while (v >= 1000 && i < u.length - 1);
    return fnum(v, v < 10 ? 2 : v < 100 ? 1 : 0) + ' ' + u[i];
  }
  function fbits(b) {
    let v = b * 8; if (v < 1000) return fnum(v) + ' b';
    const u = ['Kb', 'Mb', 'Gb', 'Tb']; let i = -1;
    do { v /= 1000; i += 1; } while (v >= 1000 && i < u.length - 1);
    return fnum(v, v < 10 ? 2 : v < 100 ? 1 : 0) + ' ' + u[i];
  }
  function funit(unit, bytes, packets) { return unit === 'bits' ? fbits(bytes) : unit === 'packets' ? fnum(packets) : fbytes(bytes); }
  function ftime(t) { return t.toFixed(6); }
  function fdur(s) { s = Math.floor(s); const h = Math.floor(s / 3600), m = Math.floor(s / 60) % 60, x = s % 60; return (h ? h + ':' + String(m).padStart(2, '0') : String(m)) + ':' + String(x).padStart(2, '0'); }
  function now() { return time(Math.max(N, 1)); }

  /* ---------- Bytes und Schichten eines Pakets ---------- */
  function macB(m) { return m === 'Broadcast' ? [255, 255, 255, 255, 255, 255] : m.split(':').map(function (x) { return parseInt(x, 16); }); }
  function ip4B(a) { return a.split('.').map(Number); }
  function ip6B(a) {
    const [l, r] = a.split('::'); const L = l ? l.split(':') : []; const R = r ? r.split(':') : [];
    const g = L.concat(new Array(8 - L.length - R.length).fill('0'), R);
    const out = []; g.forEach(function (x) { const v = parseInt(x || '0', 16); out.push(v >> 8, v & 255); }); return out;
  }
  const u16 = function (v) { return [(v >> 8) & 255, v & 255]; };
  const u32 = function (v) { return [(v >>> 24) & 255, (v >>> 16) & 255, (v >>> 8) & 255, v & 255]; };
  const ascii = function (s) { const o = []; for (let i = 0; i < s.length; i += 1) { const c = s.charCodeAt(i); o.push(c < 256 ? c : 63); } return o; };
  function hx(v, w) { return '0x' + v.toString(16).padStart(w || 4, '0'); }

  function detail(n) {
    const p = pkt(n); const c = CBY[p.conn];
    const bytes = []; const layers = [];
    const R = rng(n * 7919 + 13);
    const put = function (arr) { const s = bytes.length; arr.forEach(function (b) { bytes.push(b & 255); }); return [s, bytes.length]; };
    const L = function (title, range, fields, extra) { const l = { title: title, range: range, fields: fields || [] }; if (extra) Object.assign(l, extra); layers.push(l); return l; };
    const F = function (k, v, range) { return { k: k, v: v, range: range }; };
    const out = p.dir === 'out';
    const srcMac = c.proto === 'ARP' ? p.src : (out ? MAC.local : (c.host.mac || MAC.router));
    const dstMac = c.proto === 'ARP' ? p.dst : (out ? (p.dst.indexOf('224.') === 0 ? '01:00:5e:00:00:fb' : p.dst.indexOf('239.') === 0 ? '01:00:5e:7f:ff:fa' : (c.host.mac || MAC.router)) : MAC.local);
    const v6 = c.host.v6;
    // Ethernet
    const e0 = put(macB(dstMac)), e1 = put(macB(srcMac)), e2 = put(c.proto === 'ARP' ? [8, 6] : v6 ? [0x86, 0xdd] : [8, 0]);
    L('Ethernet II, Src: ' + srcMac + ', Dst: ' + dstMac, [0, 14], [F('Destination', dstMac, e0), F('Source', srcMac, e1), F('Type', c.proto === 'ARP' ? 'ARP (0x0806)' : v6 ? 'IPv6 (0x86dd)' : 'IPv4 (0x0800)', e2)]);
    if (c.proto === 'ARP') {
      const req = p.info.indexOf('Who') === 0;
      const st = bytes.length;
      const a0 = put([0, 1]), a1 = put([8, 0]), a2 = put([6, 4]), a3 = put([0, req ? 1 : 2]);
      const a4 = put(macB(req ? MAC.local : MAC.router)), a5 = put(ip4B(req ? LOCAL : '192.0.2.1')), a6 = put(req ? [0, 0, 0, 0, 0, 0] : macB(MAC.local)), a7 = put(ip4B(req ? '192.0.2.1' : LOCAL));
      L('Address Resolution Protocol (' + (req ? 'request' : 'reply') + ')', [st, bytes.length], [F('Hardware type', 'Ethernet (1)', a0), F('Protocol type', 'IPv4 (0x0800)', a1), F('Hardware size / Protocol size', '6 / 4', a2), F('Opcode', req ? 'request (1)' : 'reply (2)', a3), F('Sender MAC address', req ? MAC.local : MAC.router, a4), F('Sender IP address', req ? LOCAL : '192.0.2.1', a5), F('Target MAC address', req ? '00:00:00:00:00:00' : MAC.local, a6), F('Target IP address', req ? '192.0.2.1' : LOCAL, a7)]);
      while (bytes.length < p.len) bytes.push(0);
      if (bytes.length > 42) L('Padding', [42, bytes.length], []);
      return { p: p, c: c, bytes: bytes, layers: frame(p, layers, bytes) };
    }
    const tcp = c.proto === 'TCP', udp = c.proto === 'UDP';
    const l4 = tcp ? 6 : udp ? 17 : 1;
    const st = bytes.length;
    if (v6) {
      const i0 = put([0x60, 0, 0, 0]), i1 = put(u16(p.len - 54)), i2 = put([l4]), i3 = put([out ? 64 : 56]);
      const i4 = put(ip6B(p.src)), i5 = put(ip6B(p.dst));
      L('Internet Protocol Version 6, Src: ' + p.src + ', Dst: ' + p.dst, [st, bytes.length], [F('Version', '6', [i0[0], i0[0] + 1]), F('Payload Length', String(p.len - 54), i1), F('Next Header', tcp ? 'TCP (6)' : 'UDP (17)', i2), F('Hop Limit', String(out ? 64 : 56), i3), F('Source Address', p.src, i4), F('Destination Address', p.dst, i5)]);
    } else {
      const ident = Math.floor(R() * 65535), ttl = out ? 64 : (c.host.tags ? 64 : 55);
      const i0 = put([0x45]), i1 = put([0]), i2 = put(u16(p.len - 14)), i3 = put(u16(ident)), i4 = put([0x40, 0]), i5 = put([ttl]), i6 = put([l4]), i7 = put(u16(Math.floor(R() * 65535))), i8 = put(ip4B(p.src)), i9 = put(ip4B(p.dst));
      L('Internet Protocol Version 4, Src: ' + p.src + ', Dst: ' + p.dst, [st, bytes.length], [F('Version / Header Length', '4 / 20 bytes (5)', i0), F('Differentiated Services', '0x00 (DSCP: CS0, ECN: Not-ECT)', i1), F('Total Length', String(p.len - 14), i2), F('Identification', hx(ident) + ' (' + ident + ')', i3), F('Flags', "0x2, Don't fragment", i4), F('Time to Live', String(ttl), i5), F('Protocol', tcp ? 'TCP (6)' : udp ? 'UDP (17)' : 'ICMP (1)', i6), F('Header Checksum', hx(bytes[i7[0]] * 256 + bytes[i7[0] + 1]) + ' [nicht geprüft]', i7), F('Source Address', p.src, i8), F('Destination Address', p.dst, i9)]);
    }
    let rels = [];
    let payloadLen = 0;
    if (tcp) {
      const syn = !!p.flags && p.flags.indexOf('SYN') >= 0;
      const hl = syn ? 40 : 32;
      const ts = bytes.length;
      const m = /Seq=(\d+)/.exec(p.info), a = /Ack=(\d+)/.exec(p.info), w = /Win=(\d+)/.exec(p.info);
      const syn0 = !!p.flags && p.flags.indexOf('SYN') >= 0;
      const seq = m ? +m[1] : n === 2409 ? 1449 : 1, ack = a ? +a[1] : n === 2409 ? 1 + HTTP_REQ.length : syn0 ? 0 : 1, win = w ? +w[1] : 501;
      payloadLen = p.len - 14 - (v6 ? 40 : 20) - hl;
      const fl = p.flags || (payloadLen > 0 ? 'PSH ACK' : 'ACK');
      const fv = (fl.indexOf('FIN') >= 0 ? 1 : 0) | (fl.indexOf('SYN') >= 0 ? 2 : 0) | (fl.indexOf('RST') >= 0 ? 4 : 0) | (fl.indexOf('PSH') >= 0 ? 8 : 0) | (fl.indexOf('ACK') >= 0 ? 16 : 0);
      const t0 = put(u16(p.sport)), t1 = put(u16(p.dport)), t2 = put(u32(seq + 0x6c1a0000)), t3 = put(u32(ack + 0x1f3b0000)), t4 = put([hl * 4, fv]), t5 = put(u16(win)), t6 = put(u16(Math.floor(R() * 65535))), t7 = put([0, 0]), t8 = put(syn ? [2, 4, 5, 180, 4, 2, 8, 10].concat(u32(3122400000 + n * 3), u32(fv & 16 ? 1844200000 + n : 0), [1, 3, 3, 7]) : [1, 1, 8, 10].concat(u32(3122400000 + n * 3), u32(1844200000 + n)));
      const flagText = fl.split(' ').join(', ');
      const fields = [F('Source Port', String(p.sport), t0), F('Destination Port', String(p.dport), t1), F('Sequence Number', seq + ' (relativ)', t2), F('Acknowledgment Number', ack + ' (relativ)', t3), F('Header Length / Flags', hl + ' bytes (' + hl / 4 + ') / ' + hx(fv, 3) + ' (' + flagText + ')', t4), F('Window', String(win), t5), F('Checksum', hx(bytes[t6[0]] * 256 + bytes[t6[0] + 1]) + ' [nicht geprüft]', t6), F('Urgent Pointer', '0', t7), F('Options', syn ? '20 bytes: MSS 1460, SACK erlaubt, Timestamps, NOP, Window Scale 7' : '12 bytes: NOP, NOP, Timestamps', t8)];
      if (p.an) fields.push({ k: 'TCP-Analyse', v: ({ 'Retransmission': 'Wiederholtes Segment (Retransmission)', 'Dup ACK': 'Doppeltes ACK, ein Segment fehlt', 'Out-Of-Order': 'Segment außer der Reihe', 'RST': 'Verbindung abgebrochen (RST)' })[p.an], range: [ts, ts + hl], warn: true });
      if (n === 1205) rels = [['→ bestätigt in Paket', 1206, '18,4 ms']];
      else if (n === 1206) rels = [['← ACK für Paket', 1205, '18,4 ms']];
      else if (n === 1204) rels = [['Verbindungsaufbau (SYN bis ACK):', null, '19,1 ms']];
      else if (n === 1217) rels = [['Wiederholt Segment aus Paket', 1215, '212,6 ms']];
      else if (n === 2405) rels = [['→ beantwortet in Paket', 2409, '41,2 ms'], ['→ bestätigt in Paket', 2406, '17,9 ms']];
      else if (n === 2409) rels = [['← Antwort auf Paket', 2405, '41,2 ms']];
      else if (p.dir === 'in' && payloadLen > 0) rels = [['→ bestätigt in Paket', n + 1 + Math.floor(hash(n, 30) * 3), (12 + hash(n, 31) * 30).toFixed(1).replace('.', ',') + ' ms']];
      L('Transmission Control Protocol, Src Port: ' + p.sport + ', Dst Port: ' + p.dport + ', Seq: ' + seq + (ack ? ', Ack: ' + ack : '') + ', Len: ' + payloadLen, [ts, bytes.length], fields, { rels: rels });
    } else if (udp) {
      const us = bytes.length;
      payloadLen = p.len - 14 - (v6 ? 40 : 20) - 8;
      const u0 = put(u16(p.sport)), u1 = put(u16(p.dport)), u2 = put(u16(payloadLen + 8)), u3 = put(u16(Math.floor(R() * 65535)));
      L('User Datagram Protocol, Src Port: ' + p.sport + ', Dst Port: ' + p.dport, [us, bytes.length], [F('Source Port', String(p.sport), u0), F('Destination Port', String(p.dport), u1), F('Length', String(payloadLen + 8), u2), F('Checksum', hx(bytes[u3[0]] * 256 + bytes[u3[0] + 1]) + ' [nicht geprüft]', u3)]);
    } else {
      const is = bytes.length; const req = p.dir === 'out';
      const c0 = put([req ? 8 : 0]), c1 = put([0]), c2 = put([0x4f, 0x21]), c3 = put([0, 7]), sq = Math.floor(p.t) + 1, c4 = put(u16(sq));
      const data = []; for (let i = 0; i < 56; i += 1) data.push(i < 8 ? Math.floor(R() * 256) : 8 + i);
      const c5 = put(data);
      L('Internet Control Message Protocol', [is, bytes.length], [F('Type', req ? '8 (Echo (ping) request)' : '0 (Echo (ping) reply)', c0), F('Code', '0', c1), F('Checksum', '0x4f21 [nicht geprüft]', c2), F('Identifier', '0x0007', c3), F('Sequence Number', String(sq), c4), F('Data', '56 bytes', c5)]);
      return { p: p, c: c, bytes: bytes, layers: frame(p, layers, bytes) };
    }
    // Anwendung
    const as = bytes.length;
    const fill = function (k) { while (bytes.length < p.len) bytes.push(k === 'z' ? 0 : Math.floor(R() * 256)); };
    if (payloadLen <= 0) return { p: p, c: c, bytes: bytes, layers: frame(p, layers, bytes) };
    if (p.kind === 'clienthello') {
      const name = ascii(p.sni);
      const r0 = put([22]), r1 = put([3, 1]), r2 = put(u16(512));
      const h0 = put([1]), h1 = put([0, 1, 252]), h2 = put([3, 3]);
      const rnd = []; for (let i = 0; i < 32; i += 1) rnd.push(Math.floor(R() * 256)); const h3 = put(rnd);
      const sid = [32]; for (let i = 0; i < 32; i += 1) sid.push(Math.floor(R() * 256)); const h4 = put(sid);
      const h5 = put([0, 8, 0x13, 1, 0x13, 2, 0x13, 3, 0xc0, 0x2b]); const h6 = put([1, 0]);
      const h7 = put(u16(441));
      const x0 = put([0, 0].concat(u16(name.length + 5), u16(name.length + 3), [0], u16(name.length))); const x1 = put(name);
      const x2 = put([0, 0x10, 0, 14, 0, 12, 2, 0x68, 0x32, 8].concat(ascii('http/1.1')));
      const x3 = put([0, 0x2b, 0, 3, 2, 3, 4]);
      const x4s = bytes.length; fill('z'); const x4 = [x4s, bytes.length];
      L('Transport Layer Security', [as, bytes.length], [F('Content Type', 'Handshake (22)', r0), F('Version', 'TLS 1.0 (0x0301)', r1), F('Length', '512', r2), F('Handshake Type', 'Client Hello (1)', h0), F('Length', '508', h1), F('Version', 'TLS 1.2 (0x0303)', h2), F('Random', rnd.slice(0, 6).map(function (b) { return b.toString(16).padStart(2, '0'); }).join('') + '…', h3), F('Session ID', '32 bytes', h4), F('Cipher Suites', '4 suites: TLS_AES_128_GCM_SHA256, TLS_AES_256_GCM_SHA384, …', h5), F('Compression Methods', 'null', h6), F('Extensions Length', '441', h7), F('Server Name (SNI)', p.sni, [x0[0], x1[1]]), F('ALPN', 'h2, http/1.1', x2), F('Supported Versions', 'TLS 1.3, TLS 1.2', x3), F('Padding', (x4[1] - x4[0]) + ' bytes', x4)]);
    } else if (p.kind === 'tls-data') {
      const len = payloadLen - 5;
      const r0 = put([23]), r1 = put([3, 3]), r2 = put(u16(len)); const ds = bytes.length; fill();
      L('Transport Layer Security', [as, bytes.length], [F('Content Type', 'Application Data (23)', r0), F('Version', 'TLS 1.2 (0x0303)', r1), F('Length', String(len), r2), F('Encrypted Application Data', len + ' bytes (verschlüsselt)', [ds, bytes.length])]);
    } else if (p.kind === 'dns-q' || p.kind === 'dns-r' || p.kind === 'mdns') {
      const resp = p.kind === 'dns-r' || (p.kind === 'mdns' && p.dir === 'in');
      const id = p.qid != null ? p.qid : (p.kind === 'mdns' ? 0 : parseInt((/0x([0-9a-f]{4})/.exec(p.info) || [0, '0'])[1], 16));
      const name = p.name || 'www.example.com';
      const qn = []; name.split('.').forEach(function (part) { qn.push(part.length); ascii(part).forEach(function (b) { qn.push(b); }); }); qn.push(0);
      const d0 = put(u16(id)), d1 = put(resp ? [0x81, 0x80] : [1, 0]), d2 = put([0, 1, 0, resp ? 1 : 0, 0, 0, 0, 0]);
      const d3 = put(qn), d4 = put([0, 1, 0, 1]);
      const fields = [F('Transaction ID', hx(id), d0), F('Flags', resp ? '0x8180 Standard query response, No error' : '0x0100 Standard query', d1), F('Questions / Answer RRs', resp ? '1 / 1' : '1 / 0', d2), F('Query', name + ': type ' + (/AAAA/.test(p.info) ? 'AAAA' : p.kind === 'mdns' ? 'PTR' : 'A') + ', class IN', [d3[0], d4[1]])];
      if (resp) {
        const ipm = /\sA\s([0-9.]+)$/.exec(p.info);
        const ans = ipm ? ip4B(ipm[1]) : [192, 0, 2, 50];
        const d5 = put([0xc0, 0x0c, 0, 1, 0, 1].concat(u32(300), [0, 4], ans));
        fields.push(F('Answer', name + ': type A, class IN, addr ' + (ipm ? ipm[1] : '192.0.2.50') + ', TTL 300', d5));
        if (p.kind === 'dns-r') fields.push({ k: 'Antwortzeit', v: (n === 1201 ? '4,8' : (2 + hash(n, 40) * 30).toFixed(1).replace('.', ',')) + ' ms', range: d0 });
      }
      fill('z');
      L(p.kind === 'mdns' ? 'Multicast Domain Name System (' + (resp ? 'response' : 'query') + ')' : 'Domain Name System (' + (resp ? 'response' : 'query') + ')', [as, bytes.length], fields, { rels: n === 1200 ? [['→ beantwortet in Paket', 1201, '4,8 ms']] : n === 1201 ? [['← Antwort auf Paket', 1200, '4,8 ms']] : [] });
    } else if (p.kind === 'http-story-resp') {
      // Letztes Segment der Antwort: wie Wireshark erst die zusammengesetzten Segmente, dann HTTP aus dem ganzen Puffer
      // (die Felder liegen größtenteils in Paket 2407 und haben hier keine Bytes).
      put(ascii(HTTP_RESP.slice(1448, 1448 + payloadLen)));
      L('[2 zusammengesetzte TCP-Segmente (' + HTTP_RESP.length + ' bytes): #2407(1448), #' + n + '(' + payloadLen + ')]', [as, bytes.length],
        [F('Segment 2407', '1448 bytes (Bytes 0–1447)'), F('Segment ' + n, payloadLen + ' bytes (Bytes 1448–' + (HTTP_RESP.length - 1) + ', in diesem Paket)', [as, bytes.length])], { rels: [['Erstes Segment in Paket', 2407, '']] });
      const head = HTTP_RESP.split('\r\n\r\n')[0].split('\r\n');
      const fields = head.slice(0, 9).map(function (line, i) { return F(i === 0 ? 'Status Line' : line.split(':')[0], i === 0 ? line : line.slice(line.indexOf(':') + 1).trim()); });
      fields.push(F('File Data', (HTTP_RESP.length - HTTP_RESP.indexOf('\r\n\r\n') - 4) + ' bytes', [as + Math.max(0, HTTP_RESP.indexOf('\r\n\r\n') + 4 - 1448), bytes.length]));
      L('Hypertext Transfer Protocol', [as, bytes.length], fields, { rels: [['Zusammengesetzt mit Paket', 2407, '']] });
    } else if (p.kind === 'http-story-req' || p.kind === 'http-req' || p.kind === 'http-resp' || p.kind === 'ssdp' || p.kind === 'ipp') {
      let text;
      if (p.kind === 'http-story-req') text = HTTP_REQ;
      else if (p.kind === 'http-story-resp') text = HTTP_RESP.slice(1448);
      else if (p.kind === 'http-req') text = 'GET /dists/stable/main/binary-amd64/Packages.xz HTTP/1.1\r\nHost: ' + p.name + '\r\nUser-Agent: Debian APT-HTTP/1.3 (2.9.8)\r\nAccept: */*\r\n\r\n';
      else if (p.kind === 'ssdp') text = 'NOTIFY * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nCACHE-CONTROL: max-age=1800\r\nLOCATION: http://' + p.src + ':49152/description.xml\r\nNT: upnp:rootdevice\r\nNTS: ssdp:alive\r\nSERVER: Example/1.0 UPnP/1.0 Beispielgerät/2.4\r\nUSN: uuid:00000000-0000-4000-8000-0000000000' + p.src.split('.')[3] + '::upnp:rootdevice\r\n\r\n';
      else if (p.kind === 'ipp') text = p.dir === 'out' ? 'POST /ipp/print HTTP/1.1\r\nContent-Type: application/ipp\r\nHost: drucker.local:631\r\nContent-Length: 194\r\n\r\n' : 'HTTP/1.1 200 OK\r\nContent-Type: application/ipp\r\nContent-Length: 472\r\n\r\n';
      else text = 'HTTP/1.1 200 OK\r\nDate: Mon, 09 Nov 2026 13:41:02 GMT\r\nServer: example-httpd\r\nContent-Type: application/x-xz\r\nContent-Length: 1342\r\n\r\n';
      const fields = []; let pos = 0;
      text.split('\r\n').some(function (line, i, all) {
        const s = bytes.length; put(ascii(line + (i < all.length - 1 ? '\r\n' : '')));
        pos += line.length + 2;
        if (line === '') return true;
        if (fields.length < 9) fields.push(F(i === 0 ? (p.dir === 'out' && p.kind !== 'ssdp' ? 'Request Line' : 'Status Line') : line.split(':')[0], i === 0 ? line : line.slice(line.indexOf(':') + 1).trim(), [s, bytes.length]));
        return bytes.length >= p.len;
      });
      if (bytes.length < p.len) { const s = bytes.length; put(ascii(text.slice(pos).slice(0, p.len - bytes.length))); if (bytes.length > s) fields.push(F(p.kind === 'http-story-resp' ? 'File Data (Fortsetzung)' : 'Body', (bytes.length - s) + ' bytes', [s, bytes.length])); }
      fill('z'); bytes.length = Math.min(bytes.length, p.len);
      L(p.kind === 'ssdp' ? 'Simple Service Discovery Protocol' : p.kind === 'ipp' ? 'Hypertext Transfer Protocol (IPP)' : 'Hypertext Transfer Protocol', [as, bytes.length], fields, { rels: p.kind === 'http-story-resp' ? [['Zusammengesetzt mit Paket', 2407, '']] : [] });
    } else if (p.kind === 'http-story-seg') {
      put(ascii(HTTP_RESP.slice(0, payloadLen)));
      L('TCP-Nutzdaten (Teil einer HTTP-Antwort)', [as, bytes.length], [F('Daten', payloadLen + ' bytes', [as, bytes.length])], { rels: [['→ zusammengesetzt in Paket', 2409, '']] });
    } else if (p.kind === 'quic') {
      const short = p.dir === 'in';
      const q0 = put([short ? 0x43 : 0x4b]); const cid = []; for (let i = 0; i < 8; i += 1) cid.push(Math.floor(R() * 256)); const q1 = put(cid); const ps = bytes.length; fill();
      L('QUIC IETF', [as, bytes.length], [F('Header Form', 'Short Header (0)', q0), F('Destination Connection ID', cid.map(function (b) { return b.toString(16).padStart(2, '0'); }).join(''), q1), F('Protected Payload', (bytes.length - ps) + ' bytes (verschlüsselt)', [ps, bytes.length])]);
    } else if (p.kind === 'ntp') {
      const n0 = put([p.dir === 'in' ? 0x24 : 0x23]), n1 = put([p.dir === 'in' ? 2 : 0]); fill('z');
      L('Network Time Protocol (NTP Version 4, ' + (p.dir === 'in' ? 'server' : 'client') + ')', [as, bytes.length], [F('Flags', p.dir === 'in' ? '0x24, Leap Indicator: no warning, Version: 4, Mode: server' : '0x23, Version: 4, Mode: client', n0), F('Peer Clock Stratum', p.dir === 'in' ? 'secondary reference (2)' : 'unspecified (0)', n1), F('Transmit Timestamp', 'Nov  9, 2026 ' + clock(p.t, true).replace(',', '.') + ' UTC+1', [as + 40, as + 48])]);
    } else if (p.kind === 'dhcp') {
      const b0 = put([p.dir === 'in' ? 2 : 1, 1, 6, 0]), b1 = put([0x6a, 0x1c, 0x2f, 0x40]); fill('z');
      L('Dynamic Host Configuration Protocol (' + (p.dir === 'in' ? 'ACK' : 'Request') + ')', [as, bytes.length], [F('Message type', p.dir === 'in' ? 'Boot Reply (2)' : 'Boot Request (1)', b0), F('Transaction ID', '0x6a1c2f40', b1), F('Client MAC address', MAC.local, [as + 28, as + 34]), F('Host Name', 'arbeitsplatz', [as + 250, as + 262])]);
    } else if (p.kind === 'sip') {
      const sp = p.sip; const cut = sp.text.indexOf('\r\n\r\n');
      const fields = [];
      sp.text.slice(0, cut).split('\r\n').forEach(function (line, i) {
        const r = put(ascii(line + '\r\n'));
        fields.push(F(i === 0 ? (sp.req ? 'Request-Line' : 'Status-Line') : line.split(':')[0], i === 0 ? line : line.slice(line.indexOf(':') + 1).trim(), r));
      });
      put(ascii('\r\n'));
      L('Session Initiation Protocol (' + sp.what + ')', [as, bytes.length], fields, { rels: sp.rels });
      if (sp.sdp) {
        const ss = bytes.length;
        const NM = { v: 'Version', o: 'Owner/Creator', s: 'Session Name', c: 'Connection Information', t: 'Time Description', m: 'Media Description', a: 'Media Attribute' };
        const sf = sp.text.slice(cut + 4).split('\r\n').filter(Boolean).map(function (line) { const r = put(ascii(line + '\r\n')); return F(NM[line[0]] + ' (' + line[0] + ')', line.slice(2), r); });
        L('Session Description Protocol', [ss, bytes.length], sf);
      }
    } else if (p.kind === 'rtp') {
      const r = p.rtp, S = r.R;
      const b0 = put([0x80]), b1 = put([(r.mark ? 0x80 : 0) | r.pt]), b2 = put(u16(r.seq)), b3 = put(u32(r.ts)), b4 = put(u32(r.ssrc));
      const ps = bytes.length;
      if (S.enc) fill(); else { const base = r.pt === 8 ? 0xd5 : 0xff; const pay = []; for (let i = 0; i < r.plen; i += 1) pay.push(R() < 0.5 ? base ^ Math.floor(R() * 24) : base ^ Math.floor(R() * 4)); put(pay); }
      const fields = [F('Version', 'RFC 1889 Version (2)', b0), F('Padding / Extension / CSRC count', 'False / False / 0', b0), F('Marker', r.mark ? 'True (erstes Paket des Stroms)' : 'False', b1),
        F('Payload type', S.enc ? 'dynamisch (111), Codec unbekannt' : PTN[r.pt] + ' (' + r.pt + ')', b1), F('Sequence number', String(r.seq), b2), F('Timestamp', String(r.ts), b3),
        F('Synchronization Source identifier', hx8(r.ssrc) + ' (' + r.ssrc + ')', b4),
        F('Payload', S.enc ? (bytes.length - ps) + ' bytes (verschlüsselt oder unbekannt)' : (bytes.length - ps) + ' bytes, G.711 ' + (r.pt === 8 ? 'A-law' : 'µ-law') + ', ' + (r.plen / 8) + ' ms Sprache', [ps, bytes.length])];
      if (S.enc) fields.push({ k: 'Erkannt', v: 'ohne SIP, an der Form: Version 2, gleichbleibende SSRC, fortlaufende Nummern (Einstellung „RTP auch ohne SIP erkennen“)' });
      L('Real-Time Transport Protocol', [as, bytes.length], fields, { rels: S.setup ? [['Ausgehandelt per SDP in Paket', S.setup, '']] : [] });
    } else if (p.kind === 'rtcp') {
      const r = p.rtcp, S = r.R, side = r.out ? S.i : S.o;   // berichtet wird über den Strom der Gegenseite
      const got = 1 + Math.round(r.m * 334 / 4), lost = side.lost ? side.lost[Math.min(got, side.lost.length - 1)] : 0;
      const frac = side.loss ? Math.round(side.loss * 1.3 * 256) : 0, jit = Math.round((side.jit ? side.jit[0] : 1) * 8);
      const h0 = put([0x81, 200]), h1 = put(u16(12)), h2 = put(u32(r.ssrc)), h3 = put(u32(0xec9b7a00 + r.m * 5).concat(u32(Math.floor(R() * 4294967295)))), h4 = put(u32(((r.out ? S.o : S.i).ts0 + got * S.step) >>> 0));
      const h5 = put(u32(got)), h6 = put(u32(got * S.step)), h7 = put(u32(r.about)), h8 = put([frac].concat([0, (lost >> 8) & 255, lost & 255])), h9 = put(u32(side.seq0 + got + lost)), h10 = put(u32(jit)), h11 = put(u32(0).concat(u32(0)));
      const cname = r.out ? 'anna@192.0.2.20' : 'media@203.0.113.61'; const sd = put([0x81, 202].concat(u16(5), u32(r.ssrc), [1, cname.length], ascii(cname), [0, 0]));
      fill('z');
      L('Real-time Transport Control Protocol (Sender Report)', [as, as + 52], [F('Version / Reception report count', '2 / 1', h0), F('Packet type', 'Sender Report (200)', h0), F('Length', '12 (52 bytes)', h1), F('Sender SSRC', hx8(r.ssrc), h2),
        F('Timestamp, MSW / LSW', 'Nov  9, 2026 ' + clock(p.t, true).replace(',', '.') + ' UTC', h3), F('RTP timestamp', String(bytes[h4[0]] * 16777216 + bytes[h4[0] + 1] * 65536 + bytes[h4[0] + 2] * 256 + bytes[h4[0] + 3]), h4),
        F("Sender's packet count", String(got), h5), F("Sender's octet count", String(got * S.step), h6), F('Report: SSRC', hx8(r.about), h7),
        F('Fraction lost / Cumulative lost', frac + ' / 256 (' + (frac / 2.56).toFixed(1).replace('.', ',') + ' %) / ' + lost, h8), F('Extended highest sequence number', String(side.seq0 + got + lost), h9),
        F('Interarrival jitter', jit + ' (' + (jit / 8).toFixed(1).replace('.', ',') + ' ms bei 8000 Hz)', h10), F('Last SR / Delay since last SR', '0 / 0', h11)]);
      L('Real-time Transport Control Protocol (Source description)', [sd[0], sd[1]], [F('SSRC', hx8(r.ssrc), [sd[0] + 4, sd[0] + 8]), F('CNAME', cname, [sd[0] + 10, sd[0] + 10 + cname.length])]);
    } else {
      const ds = bytes.length; fill();
      L(p.kind === 'smb' ? 'SMB2 (Server Message Block Protocol version 2)' : p.kind === 'ssh' ? 'SSH Protocol' : 'Daten', [as, bytes.length], [F(p.kind === 'smb' ? 'Command' : 'Packet', p.kind === 'smb' ? (p.dir === 'in' ? 'Read Response (8)' : 'Read Request (8)') : (bytes.length - ds) + ' bytes (verschlüsselt)', [ds, Math.min(bytes.length, ds + 64)])]);
    }
    bytes.length = Math.min(bytes.length, p.len);
    const v = p.sip || p.rtp || p.rtcp;
    return { p: p, c: c, bytes: bytes, layers: frame(p, layers, bytes), call: v && CALL[v.call] && !CALL[v.call].reg ? CALL[v.call] : null };
  }
  function frame(p, layers, bytes) {
    const f = { title: 'Frame ' + p.n + ': ' + p.len + ' bytes on wire (' + p.len * 8 + ' bits), ' + bytes.length + ' bytes captured', range: [0, bytes.length], fields: [
      { k: 'Arrival Time', v: 'Nov  9, 2026 ' + clock(p.t, true).replace(',', '.') + ' CET' },
      { k: 'Time since first packet', v: ftime(p.t) + ' seconds' },
      { k: 'Frame Length', v: p.len + ' bytes' },
      { k: 'Interface', v: ADAPTER.name }
    ] };
    return [f].concat(layers);
  }

  /* ---------- Filter (eigene Grammatik, wie der Anzeigefilter) ---------- */
  const PFIELDS = {
    // Protokolle
    eth: 'p', ip: 'p', ipv6: 'p', tcp: 'p', udp: 'p', icmp: 'p', arp: 'p', dns: 'p', mdns: 'p', tls: 'p', quic: 'p', http: 'p', ssdp: 'p', ntp: 'p', dhcp: 'p', smb2: 'p', ssh: 'p',
    sip: 'p', sdp: 'p', rtp: 'p', rtcp: 'p',
    // Felder
    'ip.addr': 'ip', 'ip.src': 'ip', 'ip.dst': 'ip', 'tcp.port': 'n', 'udp.port': 'n', 'port': 'n', 'frame.len': 'n', 'frame.number': 'n', 'domain': 's', 'tls.sni': 's',
    'tcp.flags.syn': 'b', 'tcp.flags.rst': 'b', 'tcp.analysis.flags': 'b', 'tcp.analysis.retransmission': 'b', 'tcp.analysis.duplicate_ack': 'b', 'tcp.analysis.out_of_order': 'b',
    'dns.qry.name': 's', 'http.host': 's', 'http.request.method': 's', 'eth.addr': 's',
    'sip.method': 's', 'sip.status_code': 'n', 'sip.call_id': 's', 'sip.from': 's', 'sip.to': 's', 'rtp.ssrc': 's', 'rtp.seq': 'n', 'rtp.p_type': 'n', 'voip.call_id': 's'
  };
  const CFIELDS = {
    tcp: 'p', udp: 'p', icmp: 'p', arp: 'p', ipv6: 'p', favorite: 'b',
    'ip.addr': 'ip', 'ip.src': 'ip', 'ip.dst': 'ip', remote: 'ip', port: 'n', srcport: 'n', dstport: 'n', 'tcp.port': 'n', 'udp.port': 'n', proto: 's', direction: 's', service: 's', program: 's', 'program.path': 's', domain: 's', 'as.name': 's', 'as.number': 'n', country: 's', packets: 'n', bytes: 'n'
  };
  function pval(p, f) {
    const c = CBY[p.conn]; const pr = p.proto.toLowerCase();
    switch (f) {
      case 'eth': return true;
      case 'ip': return c.proto !== 'ARP' && !c.host.v6;
      case 'ipv6': return !!c.host.v6;
      case 'tcp': return c.proto === 'TCP';
      case 'udp': return c.proto === 'UDP';
      case 'icmp': return c.proto === 'ICMP';
      case 'arp': return c.proto === 'ARP';
      case 'dns': return pr === 'dns' || pr === 'mdns';
      case 'tls': return pr.indexOf('tls') === 0;
      case 'http': return pr === 'http' || pr === 'ipp';
      case 'ssh': return pr === 'sshv2';
      case 'mdns': case 'quic': case 'ssdp': case 'ntp': case 'dhcp': case 'smb2': return pr === f;
      case 'ip.addr': return c.proto === 'ARP' ? [] : [p.src, p.dst];
      case 'ip.src': return p.src; case 'ip.dst': return p.dst;
      case 'tcp.port': return c.proto === 'TCP' ? [p.sport, p.dport] : null;
      case 'udp.port': return c.proto === 'UDP' ? [p.sport, p.dport] : null;
      case 'port': return [p.sport, p.dport];
      case 'frame.len': return p.len; case 'frame.number': return p.n;
      case 'domain': return [p.name, p.sni, c.host.name].filter(Boolean);
      case 'tls.sni': return p.kind === 'clienthello' ? p.sni : null;
      case 'dns.qry.name': return (pr === 'dns' || pr === 'mdns') ? p.name : null;
      case 'http.host': return (pr === 'http') ? (p.name || c.host.name) : null;
      case 'http.request.method': return p.method || null;
      case 'tcp.flags.syn': return p.flags.indexOf('SYN') >= 0;
      case 'tcp.flags.rst': return p.flags.indexOf('RST') >= 0;
      case 'tcp.analysis.flags': return !!p.an;
      case 'tcp.analysis.retransmission': return p.an === 'Retransmission';
      case 'tcp.analysis.duplicate_ack': return p.an === 'Dup ACK';
      case 'tcp.analysis.out_of_order': return p.an === 'Out-Of-Order';
      case 'eth.addr': return [MAC.local, c.host.mac || MAC.router];
      case 'sip': return !!p.sip;
      case 'sdp': return !!p.sip && p.sip.sdp;
      case 'rtp': return !!p.rtp;
      case 'rtcp': return !!p.rtcp;
      case 'sip.method': return p.sip ? (p.sip.method || null) : null;
      case 'sip.status_code': return p.sip ? p.sip.status : null;
      case 'sip.call_id': return p.sip ? p.sip.cid : null;
      case 'sip.from': return p.sip ? p.sip.from : null;
      case 'sip.to': return p.sip ? p.sip.to : null;
      case 'rtp.ssrc': return p.rtp ? hx8(p.rtp.ssrc) : null;
      case 'rtp.seq': return p.rtp ? p.rtp.seq : null;
      case 'rtp.p_type': return p.rtp ? p.rtp.pt : null;
      case 'voip.call_id': { const v = p.sip || p.rtp || p.rtcp; const k = v && CALL[v.call]; return k && k.cid && !k.reg ? k.cid : null; }
      default: return null;
    }
  }
  function cval(c, f) {
    switch (f) {
      case 'tcp': case 'udp': case 'icmp': case 'arp': return c.proto.toLowerCase() === f;
      case 'ipv6': return !!c.host.v6;
      case 'favorite': return isFav(c);
      case 'ip.addr': return [c.src, c.dst];
      case 'remote': return c.dir === 'in' ? c.src : c.dst;
      case 'port': return [c.sport, c.dport];
      case 'srcport': return c.sport; case 'dstport': return c.dport;
      case 'ip.src': return c.src; case 'ip.dst': return c.dst;
      case 'tcp.port': return c.proto === 'TCP' ? [c.sport, c.dport] : null;
      case 'udp.port': return c.proto === 'UDP' ? [c.sport, c.dport] : null;
      case 'proto': return c.proto;
      case 'direction': return c.dir === 'in' ? 'incoming' : 'outgoing';
      case 'service': return c.svc || null;
      case 'program': return c.prog ? [c.prog, PROGS[c.prog] || ''] : null;
      case 'program.path': return PROGS[c.prog] || null;
      case 'domain': return c.host.name;
      case 'as.name': return c.host.as || null;
      case 'as.number': return c.host.asn || null;
      case 'country': return c.host.cc || null;
      case 'packets': return c.packets; case 'bytes': return c.bytes;
      default: return null;
    }
  }
  function isFav(c) { return !!c.host.fav || FAV_ADDR.indexOf(c.dst) >= 0 || FAV_ADDR.indexOf(c.src) >= 0 || FAV_PROG.indexOf(c.prog) >= 0; }

  function parse(src, fields) {
    const toks = []; const re = /\s*(\(|\)|\{|\}|==|!=|<=|>=|<|>|&&|\|\||!|"[^"]*"?|[^\s(){}=!<>"]+)/y;
    let m; re.lastIndex = 0;
    while (re.lastIndex < src.length) {
      const at = re.lastIndex; m = re.exec(src);
      if (!m) { if (/^\s*$/.test(src.slice(at))) break; return { err: [at + 1, 'unexpected', src.slice(at).trim()] }; }
      if (m[1] === undefined) break;
      toks.push({ v: m[1], at: at + (m[0].length - m[1].length) + 1 });
    }
    if (!toks.length) return { ast: null };
    let i = 0;
    const peek = function () { return toks[i] ? toks[i].v.toLowerCase() : null; };
    function or() { let l = and(); while (peek() === 'or' || peek() === '||') { i += 1; l = { op: 'or', l: l, r: and() }; } return l; }
    function and() { let l = not(); while (peek() === 'and' || peek() === '&&') { i += 1; l = { op: 'and', l: l, r: not() }; } return l; }
    function not() { if (peek() === 'not' || peek() === '!') { i += 1; return { op: 'not', e: not() }; } return prim(); }
    function prim() {
      const t = toks[i];
      if (!t) throw ['missing', src.length + 1];
      if (t.v === '(') { i += 1; const e = or(); if (peek() !== ')') throw ['close', t.at]; i += 1; return e; }
      const f = t.v.toLowerCase();
      if (!(f in fields)) throw ['unknown', t.at, t.v];
      i += 1;
      const op = peek();
      if (['==', '!=', '<', '<=', '>', '>=', 'contains', 'in', 'eq', 'ne'].indexOf(op) < 0) return { op: 'has', f: f };
      const ot = toks[i]; i += 1;
      if (fields[f] === 'p' || fields[f] === 'b') throw ['operator', ot.at, op, f];
      if (op === 'in') {
        if (peek() !== '{') throw ['missing', toks[i] ? toks[i].at : src.length + 1];
        i += 1; const vals = [];
        while (toks[i] && toks[i].v !== '}') { vals.push(toks[i].v.replace(/"/g, '')); i += 1; }
        if (!toks[i]) throw ['close', ot.at]; i += 1;
        return { op: 'in', f: f, vals: vals };
      }
      const vt = toks[i]; if (!vt) throw ['missing', src.length + 1];
      if (vt.v.charAt(0) === '"' && (vt.v.length < 2 || vt.v.slice(-1) !== '"')) throw ['string', vt.at];
      i += 1;
      const val = vt.v.replace(/^"|"$/g, '');
      if (fields[f] === 'n' && !/^\d+$/.test(val)) throw ['value', vt.at, val, f];
      if (fields[f] === 'ip' && !/^[0-9a-f:.]+(\/\d+)?$/i.test(val)) throw ['value', vt.at, val, f];
      return { op: op === 'eq' ? '==' : op === 'ne' ? '!=' : op, f: f, v: val };
    }
    try {
      const ast = or();
      if (i < toks.length) throw ['unexpected', toks[i].at, toks[i].v];
      return { ast: ast };
    } catch (e) {
      if (!Array.isArray(e)) throw e;
      return { err: e };
    }
  }
  const ERRTXT = {
    unknown: function (e) { return 'unbekanntes Feld oder Protokoll „' + e[2] + '“'; },
    missing: function () { return 'hier fehlt ein Feld oder Wert'; },
    value: function (e) { return '„' + e[2] + '“ passt nicht zu ' + e[3]; },
    operator: function (e) { return '„' + e[2] + '“ geht nicht mit ' + e[3]; },
    unexpected: function (e) { return '„' + e[2] + '“ ist hier nicht erwartet (fehlt and/or?)'; },
    close: function () { return 'die Klammer wird nicht geschlossen'; },
    string: function () { return 'das Anführungszeichen wird nicht geschlossen'; }
  };
  function errText(e) { return 'Fehler an Stelle ' + e[1] + ': ' + ERRTXT[e[0]](e); }
  function ip2n(a) { const p = a.split('.'); if (p.length !== 4) return null; return ((+p[0] << 24) >>> 0) + (+p[1] << 16) + (+p[2] << 8) + (+p[3]); }
  function cmpOne(x, op, v, type) {
    if (x == null || x === '') return false;
    if (type === 'ip' && (op === '==' || op === '!=') && v.indexOf('/') > 0) {
      const [net, bits] = v.split('/'); const a = ip2n(String(x)), b = ip2n(net);
      if (a == null || b == null) return op === '!=';
      const mask = +bits === 0 ? 0 : (~0 << (32 - +bits)) >>> 0;
      const hit = ((a & mask) >>> 0) === ((b & mask) >>> 0);
      return op === '==' ? hit : !hit;
    }
    if (type === 'n') { const a = +x, b = +v; return op === '==' ? a === b : op === '!=' ? a !== b : op === '<' ? a < b : op === '<=' ? a <= b : op === '>' ? a > b : a >= b; }
    const a = String(x).toLowerCase(), b = String(v).toLowerCase();
    if (op === 'contains') return a.indexOf(b) >= 0;
    if (op === '==') return a === b;
    if (op === '!=') return a !== b;
    return op === '<' ? a < b : op === '<=' ? a <= b : op === '>' ? a > b : a >= b;
  }
  function evalAst(ast, obj, getter, fields) {
    if (!ast) return true;
    switch (ast.op) {
      case 'or': return evalAst(ast.l, obj, getter, fields) || evalAst(ast.r, obj, getter, fields);
      case 'and': return evalAst(ast.l, obj, getter, fields) && evalAst(ast.r, obj, getter, fields);
      case 'not': return !evalAst(ast.e, obj, getter, fields);
      case 'has': { const v = getter(obj, ast.f); return Array.isArray(v) ? v.length > 0 : v != null && v !== false && v !== ''; }
      case 'in': { const v = getter(obj, ast.f); const xs = Array.isArray(v) ? v : [v]; return xs.some(function (x) { return ast.vals.some(function (y) { return cmpOne(x, '==', y, fields[ast.f]); }); }); }
      default: {
        const v = getter(obj, ast.f); const xs = Array.isArray(v) ? v : [v];
        if (ast.op === '!=') return xs.filter(function (x) { return x != null; }).every(function (x) { return cmpOne(x, '!=', ast.v, fields[ast.f]); }) && xs.some(function (x) { return x != null; });
        return xs.some(function (x) { return cmpOne(x, ast.op, ast.v, fields[ast.f]); });
      }
    }
  }
  function packetFilter(src) {
    const r = parse(src || '', PFIELDS); if (r.err) return { err: errText(r.err) };
    return { test: function (p) { return evalAst(r.ast, p, pval, PFIELDS); }, empty: !r.ast };
  }
  function connFilter(src) {
    const r = parse(src || '', CFIELDS); if (r.err) return { err: errText(r.err) };
    return { test: function (c) { return evalAst(r.ast, c, cval, CFIELDS); }, empty: !r.ast };
  }
  const HINTS = {
    not: 'Kehrt die Bedingung um', and: 'Beide Bedingungen', or: 'Eine der Bedingungen', contains: 'enthält Text',
    ip: 'IPv4-Pakete', ipv6: 'IPv6-Pakete', tcp: 'Pakete mit TCP', udp: 'Pakete mit UDP', dns: 'Namensauflösung', tls: 'Verschlüsselte Verbindungen', http: 'Unverschlüsseltes Web',
    quic: 'HTTP/3 und andere QUIC-Verbindungen', arp: 'Adressauflösung im lokalen Netz', icmp: 'Ping und Fehlermeldungen', dhcp: 'Adressvergabe', mdns: 'Namen im lokalen Netz',
    ssdp: 'Gerätesuche im lokalen Netz', ntp: 'Zeitabgleich', 'ip.addr': 'Absender oder Empfänger', 'ip.src': 'Absender', 'ip.dst': 'Empfänger', port: 'Quell- oder Zielport',
    'tcp.port': 'TCP-Port, Quelle oder Ziel', 'udp.port': 'UDP-Port, Quelle oder Ziel', 'eth.addr': 'MAC-Adresse, Absender oder Empfänger', domain: 'Domainname, z. B. example.com',
    'frame.number': 'Nummer des Pakets', 'frame.len': 'Länge des Pakets in Bytes', 'tcp.flags.syn': 'Verbindungsaufbau', 'tcp.flags.rst': 'Verbindung abgebrochen',
    'tcp.analysis.flags': 'Pakete mit einem Befund der TCP-Analyse', 'tcp.analysis.retransmission': 'Erneut gesendete Segmente', 'tcp.analysis.duplicate_ack': 'Doppelte Bestätigung',
    'tcp.analysis.out_of_order': 'Segment außer der Reihe', 'dns.qry.name': 'Angefragter Name', 'tls.sni': 'Servername beim Verbindungsaufbau', 'http.host': 'Aufgerufener Host',
    'http.request.method': 'Methode, z. B. GET', remote: 'Adresse der Gegenstelle', proto: 'Protokoll, z. B. tcp', srcport: 'Quellport', dstport: 'Zielport',
    direction: 'outgoing oder incoming', service: 'Dienst, z. B. https', 'as.name': 'Netzbetreiber der Gegenstelle', 'as.number': 'Nummer des Netzbetreibers', country: 'Land als Kürzel, z. B. DE',
    program: 'Programm, Name oder Pfad', 'program.path': 'Pfad des Programms', packets: 'Pakete der Verbindung', bytes: 'Datenmenge der Verbindung in Bytes', favorite: 'Verbindung zu einem Favoriten',
    smb2: 'Dateifreigaben (SMB)', ssh: 'Fernzugriff (SSH)', eth: 'Alle Pakete mit Ethernet',
    sip: 'Telefonie: Anrufe aufbauen und beenden (SIP)', sdp: 'SIP mit Beschreibung der Sprachverbindung', rtp: 'Sprache und Video (RTP)', rtcp: 'Berichte zu RTP (Verlust, Jitter)',
    'sip.method': 'SIP-Anfrage, z. B. INVITE oder BYE', 'sip.status_code': 'SIP-Antwort, z. B. 486 (besetzt)', 'sip.call_id': 'Kennung eines Anrufs (Call-ID)', 'sip.from': 'Anrufer (From)', 'sip.to': 'Angerufener (To)',
    'rtp.ssrc': 'Kennung eines RTP-Stroms, z. B. 0x5e1f02a7', 'rtp.seq': 'Laufende Nummer im RTP-Strom', 'rtp.p_type': 'Codec-Nummer, z. B. 8 für G.711 A-law', 'voip.call_id': 'Alle Pakete eines Anrufs: SIP und die ausgehandelten RTP/RTCP-Ströme'
  };

  /* ---------- Anrufe: Zusammenfassung aus den Paketen im Speicher (im Programm die Analyse über Paketgrenzen) ---------- */
  function callStats(id) {
    const k = CALL[id], R = k.rtp;
    const st = { call: k, id: id, sip: 0, rtp: 0, rtcp: 0, bytes: 0, msgs: [], o: { n: 0, exp: 0, lost: 0, loss: 0 }, i: { n: 0, exp: 0, lost: 0, loss: 0 }, rtpFirst: null, rtpLast: null };
    const from = k.sip ? k.sip[0] : R.from;
    const to = Math.min(N, R && R.to == null ? N : Math.max(k.sip ? k.sip[k.sip.length - 1] : 0, R ? R.to : 0));
    if (from > N) return null;
    for (let n = from; n <= to; n += 1) {
      const p = pkt(n), v = p.sip || p.rtp || p.rtcp;
      if (!v || v.call !== id) continue;
      st.bytes += p.len;
      if (p.sip) { st.sip += 1; st.msgs.push(p.sip); }
      else if (p.rtcp) st.rtcp += 1;
      else {
        st.rtp += 1; const sd = p.rtp.out ? st.o : st.i;
        sd.n += 1; sd.exp = Math.max(sd.exp, p.rtp.i + p.rtp.lost + 1); sd.last = n; if (sd.first == null) sd.first = n;
        if (st.rtpFirst == null) st.rtpFirst = n; st.rtpLast = n;
      }
    }
    const find = function (f) { return st.msgs.filter(f)[0] || null; };
    const inv = find(function (m) { return m.what === 'INVITE'; });
    const ok = find(function (m) { return m.what === 200 && m.cm === 'INVITE'; });
    const fail = find(function (m) { return !m.req && m.cm === 'INVITE' && m.what >= 300 && m.what !== 401 && m.what !== 407; });
    const cancel = find(function (m) { return m.what === 'CANCEL'; });
    const bye = find(function (m) { return m.what === 'BYE'; });
    const ring = find(function (m) { return m.what === 180; });
    st.first = from; st.t0 = time(from);
    if (k.rtpOnly) { st.state = 'rtp'; st.t1 = time(st.rtpLast || from); }
    else if (cancel) { st.state = k.out ? 'cancelled' : 'missed'; st.t1 = time(cancel.n); st.endN = cancel.n; }
    else if (fail) { st.state = 'rejected'; st.code = fail.what; st.t1 = time(fail.n); st.endN = fail.n; }
    else if (ok && bye) { st.state = 'ended'; st.t1 = time(bye.n); st.endN = bye.n; st.byUs = bye.dir === 'out'; }
    else if (ok) { st.state = 'running'; st.t1 = now(); }
    else { st.state = ring ? 'ringing' : 'setup'; st.t1 = now(); }
    st.inv = inv; st.ok = ok; st.ring = ring;
    st.tAnswer = ok ? time(ok.n) : null;
    st.talk = ok ? st.t1 - st.tAnswer : (k.rtpOnly ? st.t1 - st.t0 : 0);
    st.wait = (ok ? st.tAnswer : st.t1) - st.t0;
    st.packets = st.sip + st.rtp + st.rtcp;
    if (R) {
      [['o', R.o], ['i', R.i]].forEach(function (x) {
        const sd = st[x[0]], def = x[1];
        sd.ssrc = def.ssrc; sd.lost = Math.max(0, sd.exp - sd.n); sd.loss = sd.exp ? sd.lost / sd.exp * 100 : 0;
        sd.jit = def.jit || null; sd.gap = def.gap; sd.ooo = def.ooo || 0;
        sd.rate = !sd.n ? null : (sd.loss >= 3 || (sd.jit && sd.jit[0] >= 50)) ? 'bad' : (sd.loss >= 1 || (sd.jit && sd.jit[0] >= 30)) ? 'fair' : 'good';
      });
      const order = { good: 0, fair: 1, bad: 2 };
      st.rate = [st.o.rate, st.i.rate].filter(Boolean).sort(function (a, b) { return order[b] - order[a]; })[0] || null;
      st.codec = R.pt === 8 ? 'G.711 A-law' : R.pt === 0 ? 'G.711 µ-law' : 'unbekannt';
      st.playable = !R.enc && st.rtp > 0;
    } else { st.rate = null; st.codec = k.media.pt != null ? '' : '–'; st.playable = false; }
    return st;
  }
  function calls() { return CALLS.map(function (c) { return callStats(c.id); }).filter(Boolean); }

  /* ---------- Benachrichtigungen ---------- */
  const NOTES = [
    ['14:48:02', 'nas.local', 1260000], ['14:47:31', 'mail.example.com', 48200], ['14:46:12', '203.0.113.44 (ssh.example.org)', 23800], ['14:45:07', 'nas.local', 2410000],
    ['14:43:55', 'thunderbird', 61300], ['14:42:40', 'mail.example.com', 31000], ['14:40:18', 'nas.local', 884000], ['14:38:02', '203.0.113.44 (ssh.example.org)', 12400], ['14:36:41', 'thunderbird', 9800]
  ];

  /* ---------- Adapter (Seite Aufzeichnung) ---------- */
  const ADAPTERS = [
    { name: 'enp3s0', desc: 'Intel Ethernet Connection I219-LM', addr: ['192.0.2.20', '2001:db8::20', 'fe80::200:5eff:fe00:5314'], act: 0.8 },
    { name: 'wlp2s0', desc: 'Intel Wi-Fi 6 AX201', addr: [], act: 0 },
    { name: 'any', desc: 'Alle Adapter (Linux)', addr: [], act: 0.85 },
    { name: 'lo', desc: 'Loopback', addr: ['127.0.0.1', '::1'], act: 0.15 },
    { name: 'docker0', desc: 'Virtuelle Brücke', addr: ['172.17.0.1'], act: 0.02 },
    { name: 'bluetooth0', desc: 'Bluetooth-Adapter', addr: [], act: 0, noip: true },
    { name: 'nflog', desc: 'Linux Netfilter-Protokoll', addr: [], act: 0, noip: true }
  ];

  window.D = {
    store: store, hash: hash, rng: rng, mode: mode, setMode: setMode,
    START: START, N0: N0, LOCAL: LOCAL, ADAPTER: ADAPTER, ADAPTERS: ADAPTERS, HOSTS: HOSTS, H: H, PROGS: PROGS, FAV_ADDR: FAV_ADDR, FAV_PROG: FAV_PROG,
    CONNS: CONNS, CBY: CBY, ST: ST, SEC: SEC, ANALYSIS: ANALYSIS, NOTES: NOTES, HINTS: HINTS, PFIELDS: PFIELDS, CFIELDS: CFIELDS,
    count: function () { return N; }, total: function () { return TOTAL; }, grow: grow, gen: gen, pkt: pkt, time: time, now: now, clock: clock,
    detail: detail, HTTP_REQ: HTTP_REQ, HTTP_RESP: HTTP_RESP, isFav: isFav,
    CALLS: CALLS, CALL: CALL, VO: VO, calls: calls, callStats: callStats, hx8: hx8,
    hostsAgg: hostsAgg, servicesAgg: servicesAgg, programsAgg: programsAgg,
    packetFilter: packetFilter, connFilter: connFilter,
    fnum: fnum, fbytes: fbytes, fbits: fbits, funit: funit, ftime: ftime, fdur: fdur
  };
})();
