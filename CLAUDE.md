# CLAUDE.md – openpacketmon (Sniffnet-Fork)

> Name: **openpacketmon** („Packet Monitoring“), festgelegt am 2026-10-28; Crate, Binary, Paket und GitHub-Repo (`tomCSLK/openpacketmon`, zuerst privat) heißen so.
> Logo: Originale außerhalb des Repos (`~/Bilder/openpacketmon_Logo_v2.jpeg` 2400×1792 mit Schriftzug, `openpacketmon_icon_v2.jpeg` 2048² für Icons ab 48 px); `Logo_v1`/`icon_v1` verworfen. Favicon/kleine Icons: `static/favicon.svg`.

Sprache: Deutsch. Fork und Weiterentwicklung von **Sniffnet** (https://github.com/GyulyVGC/sniffnet), einem
Netzwerkverkehrs-Monitor in Rust (iced-GUI, libpcap). Maßgeblich für Stand, Lizenzprüfung und Vorgehen ist **`PLAN.md`**.

## Status (2026-11-02)

- **MCP-Anbindung, Stufen 1 bis 4 erledigt** (2026-11-02, `src/mcp/`, `PLAN.md` 5.4, vom Nutzer freigegeben): `openpacketmon mcp` (stdio, `rmcp`) fragt das offene Fenster über `127.0.0.1` mit Token aus `mcp-access.json` (`0600`). Standardmäßig aus, Einstellungskarte „KI-Assistenten (MCP)“. Sieben Lesewerkzeuge: `capture_status`, `overview`, `connections`, `packets`, `packet_detail`, `statistics`, `filter_help`, ohne Nutzdaten. Alle Antworten laufen durch `finish` in `src/mcp/tools.rs` (Bereinigung in `src/mcp/privacy.rs`, Cookies, Anmeldeköpfe und URL-Queries ausgeblendet); Filter laufen auf einem eigenen Index der Tür. Geprüft mit Tests und einer Testinstanz, auch mit echtem Claude Code. Stufe 3: `follow_stream` und `packet_bytes` nur mit dem Schalter „Paketinhalte erlauben“ (nicht gespeichert, endet mit dem Fenster), Geheimnisse auch in den Bytes durch `x` ersetzt (`privacy::Masker`), Hinweis unten in der Seitenleiste; Test-Paket **0.3.8**; eingeschalteter Fall mit 0.3.9 geprüft (eigene HTTP-Anfrage mit erfundenen Geheimnissen, nur `x` in Stream, Hex und Bytes; Hinweis orange, nach erneutem Öffnen grün, `PLAN.md` 5.4). Stufe 4 (Test-Paket **0.3.9**): Filter der Tür sehen ausgeblendete Werte als ausgeblendet (`FilterIndex::with_values`, `privacy::hide_in_values`, kein Filter-Orakel mehr); Fenster und `openpacketmon mcp` weisen sich gegenseitig per HMAC-SHA256 über Nonces aus, der Token geht nie über die Leitung (`protocol.rs`, `AskError::Stranger`, Clients bis 0.3.8 werden abgewiesen); Zufallstests (`fuzz_masker_with_random_pieces`, `fuzz_tools_with_random_arguments`), Angriffe als Tests und auf eine Testinstanz (24 von 24), Messung mit 1 Mio. Paketen (Sperre rund 50 ms; `transport_payload` hält jetzt nach der Transportschicht an). Ergebnisse in `PLAN.md` 5.4 Stufe 4. `README.md` „opens no network port“ gilt nur, solange der Zugang aus ist. Vor dem Release: Sicherheitsdurchsicht einschließen.
- **Vor dem Release erledigt, ohne den Haltepunkt zu berühren** (2026-11-02, Test-Paket **0.3.10**, `PLAN.md` 4.10): `deny.toml` neu bewertet (Ausnahmen bleiben, Lizenztexte der egui-Schriften jetzt in `THIRD-PARTY-LICENSES.txt`), `test_increments_ping_sequence` stabil (eigener Zähler, `latency.rs` gekennzeichnet), Sicherheitsdurchsicht (Helfer übersetzt BPF erst ohne Rechte, Update-Prüfung mit Zeitlimit, sonst ohne Befund; offen zur Entscheidung: Rechte des Helfers nur für eine Gruppe), erster Trockenlauf `dev/release.sh` bestanden, Installationstest in Containern (Ubuntu 24.04, Debian 13) bestanden. Von Sniffnet bis 2026-11-02 keine Antwort.
- **Adapter ohne IP-Verkehr ausgegraut** (2026-11-02, Test-Paket **0.3.10**): Bluetooth, NFLOG, NFQUEUE grau und nicht wählbar mit Hinweis statt Kurve, Start auf solchen Quellen mit Begründung (`CaptureError::NoIp`). `PLAN.md` 5.5.
- **Aus dem Test von 0.3.8** (2026-11-02, Test-Paket **0.3.9**): Verkehrsbalken als eine Form ohne Kerbe an der Naht, Schrift aller Knöpfe genau in der Mitte (`widgets::add_button`, egui richtete sie je nach Layout oben oder mittig aus). `PLAN.md` 5.5.
- **Oberfläche ist jetzt egui, Stufen 1 bis 4 erledigt** (2026-11-01, `src/gui/`, `PLAN.md` 5.5 „Stand egui“, Test-Pakete **0.3.5** bis **0.3.7**, kein Release): `openpacketmon` öffnet das Fenster (Wireshark-Muster: Fenster schließen beendet das Programm, Rückfrage bei laufender Aufzeichnung). **Entfernt:** `src/web/`, `app/` (`openpacketmon-app`), `templates/`, `static/`, systemd-Dienst, Optionen `--port`/`--no-browser`/`--gui`. Alles, was unten über Browser-Oberfläche, Fenster-Programm, Startlink, Dienst, `templates/` oder `static/` steht, ist **Verlauf**. Geprüft über Tests, gerenderte Bilder und kurze Starts; **die Bedienung im echten Fenster, Live-Mitschnitt, Ton, Dateidialog, Skalierung und Geo-Download sind im Nutzertest nicht einzeln bestätigt; der Nutzer fand nach 0.3.6 nichts Weiteres (2026-11-01).** Aus dem Test von 0.3.5: weißes Aufblitzen der Paketliste beim Folgen behoben und Filterleisten enden am Seitenrand (beides in 0.3.6, `PLAN.md` 5.5). Aus dem Test von 0.3.6 umgesetzt (in 0.3.7): Kopieren in den Paketdetails, Ladeanzeige, Hinweistext zum Paketspeicher, Kartenkopf der Benachrichtigungen, Zähler wieder rechteckig, Top-Listen der Übersicht klickbar, kleinerer Griff am Lautstärkeregler (`PLAN.md` 5.5).
- Aufbau heute: `openpacketmon-capture` (Helfer mit Rechten, `capture/`) → `src/engine/`, `src/dissect/`, `src/filter/` → `src/view/` (Seitendaten, ohne Oberfläche) → `src/gui/`. Paketspeicher wählbar bis 2 GiB und 2 Millionen Pakete. Stirbt der Mitschnitt-Helfer, meldet das die Oberfläche (`CaptureStopped`).

- Phase: **1, 2 und 3 erledigt** (2026-10-28): privat auf https://github.com/tomCSLK/openpacketmon, dieser Ordner ist das Repo
  (seit 4.9 neues Repo mit einem Anfangs-Commit; Bezug zum Original ist Commit `ab4bf739d464`). Umbenannt, iced-Oberfläche entfernt, Kern als `src/engine/`;
  **Browser-UI** (`src/web/`, `templates/`, `static/`, `locales/`) auf `127.0.0.1:7380` mit Startlink-Anmeldung, `.deb` mit
  systemd-User-Dienst. Details `PLAN.md` Phase 2 und 3.
- **Fenster-Programm `openpacketmon-app`** (2026-10-28, `app/`): eigenes Fenster (wry/WebKitGTK) mit Symbol im Infobereich,
  ohne Mitschnittrechte, startet bei Bedarf den Server; Startmenü öffnet es; installiert und vom Nutzer getestet (2026-10-29).
  Details `PLAN.md` „Fenster-Programm“.
- **Phase 5.1 Stufe 1 erledigt** (2026-10-28): Paketspeicher (`src/engine/packets.rs`), eigene Zerlegung (`src/dissect/`),
  Seiten Pakete/Paketdetails mit Hex-Dump, PCAP-Download, Einstellungen „Paketspeicher“. Details `PLAN.md` 5.1.
- **Phase 5.1 Stufe 2 erledigt** (2026-10-29): Anwendungsprotokolle in `src/dissect/` (`app.rs`, `dns.rs`, `dhcp.rs`,
  `tls.rs`, `text.rs`, `misc.rs`): DNS/mDNS/LLMNR, DHCP, TLS mit SNI (auch über mehrere TCP-Segmente zusammengesetzt),
  HTTP/1, SSDP, NTP, QUIC-Kopf. Details `PLAN.md` 5.1.
- **Phase 5.1 Stufe 3 erledigt** (2026-10-29): Live-Paketliste (0,3 s, `/api/packets`), Anzeigefilter mit eigener Grammatik
  (`src/filter/`), Treffer-Zwischenspeicher (`src/view/filter_index.rs`), Farbregeln, Stream folgen (`src/view/stream.rs`).
  Details `PLAN.md` 5.1.
- **Phase 5.1 Stufe 4 erledigt** (2026-10-29): TCP-Analyse (`src/engine/analysis.rs`: Retransmission, Lücke, außer der
  Reihe, Dup-ACK, Nullfenster, Keep-Alive, RTT, Handshake- und DNS-Zeit) mit Filterfeldern `tcp.analysis.*`/`dns.time`,
  Seite „Statistik“ (`src/view/statistics.rs`: Diagramm, Protokollhierarchie, Befunde, Antwortzeiten). Details `PLAN.md` 5.1.
- **Fehler behoben** (2026-10-29): Live-Mitschnitt schnitt Pakete auf 200 Bytes ab; jetzt ganze Pakete, wenn der
  Paketspeicher sie behält. Paketspeicher (vollständig/Kopfdaten/aus, Anzahl) auch auf der Seite Aufzeichnung wählbar.
- **Nachbesserungen aus dem Nutzertest** (2026-10-29): Seitenleiste mit Seitenzahlen, Seitenfeld und „Gehe zu Paket“
  (Strg+G) für Pakete und Verbindungen (`src/web/pager.rs`, `templates/pager.html`); Statistik springt beim Nachladen
  nicht mehr nach oben (kleine Fragmente, Diagramm per `setData`, `live.js` hält die Scrollposition); Top-Listen der
  Übersicht bleiben in der Karte, gekürzte Prozessnamen aus dem Pfad ergänzt. Details `PLAN.md` (Phase 3 Punkt 6,
  5.1 Stufe 4 Rückmeldungen).
- **Violette Seitenleiste** (2026-10-29): Farben aus dem Designsystem `../design-system` (nur Violett-Skala), hell Verlauf
  `#5d12af` → `#220f4b` mit heller Kachel für die aktive Seite, dunkel Flächen violett getönt. Abweichung vom Stil von
  opennetmon, dort in `PLAN.md` zur Übernahme vorgemerkt. Details `PLAN.md` Phase 3 Punkt 7.
- Lizenzprüfung erledigt (`PLAN.md` Abschnitt 1): Fork ist erlaubt (`MIT OR Apache-2.0`).
- Entscheidungen (`PLAN.md` Abschnitt 0): privates GitHub-Repo, später öffentlich; Lizenz wie das Original; Funktionsumfang wie das
  Original; **neue Oberfläche im Stil von opennetmon**; Installation auf dem **Client**; Abspaltung vom Original.
- Außerdem entschieden: **nur Linux** (vorerst; **Windows später vorgesehen**), **lokaler Dienst + Browser-UI** ohne Server-Installation
  (keine Handy-Darstellung nötig, 2026-10-28), Start **ohne Upstream-Historie**,
  opennetmon-Dateien für den Fork unter MIT/Apache **freigegeben**.
- **`PLAN.md` Phase 3 Punkte 9–12 umgesetzt** (2026-10-29, Commit `481f367`, Test-Paket 0.2.0): durchgehende Paketliste wie Wireshark (`static/packets.js`, `/api/packets`), Verbindungen mit
  **einem Filterfeld** in der Grammatik des Anzeigefilters (`Filter::parse_connections`, `CONNECTION_FIELDS`),
  Favoriten = Hosts, Programme und **IP-Adressen** (keine Dienste), Benachrichtigungen nur noch für Favoriten (Grenzwert,
  Blacklist-Meldung und Webhook entfernt), **Mini-Ansicht entfernt** (auch im Fenster-Programm). Details `PLAN.md` Phase 3.
- **Rückmeldungen zu 0.2.0 umgesetzt** (2026-10-29, Commit `481f367`, Test-Paket 0.2.1): `PLAN.md` Phase 3 Punkt 13 (alle Leisten mit
  einer Höhe, `--control-h`/`--control-h-sm`, eigene Auswahlboxen, flachere Kartenköpfe), Punkt 14 (**IP-Blacklist
  ganz entfernt**, Variante C, inkl. Crate `prefix-trie`), Punkt 15 (Verbindungsseite: Knopfleiste im Seitenkopf mit
  Pakete/Statistik/Verbindungen der Adresse/des Hosts, Latenz in der Faktenliste).
- **Phase 4 geplant** (2026-10-29, `PLAN.md` Phase 4, Umsetzung je Schritt nach Freigabe): IANA-Dienstliste statt nmap,
  keine eingebauten GeoLite-DBs (DB-IP Lite per Download), Testdaten prüfen, `NOTICE` und
  Kopfzeilen für geänderte Upstream-Dateien (vor dem Neuschreiben der Historie), `THIRD-PARTY-LICENSES.txt`, `cargo deny`,
  `SECURITY.md`/`CONTRIBUTING.md`, Sicherheitstest, dann **Historie neu schreiben** (neues Repo mit einem Anfangs-Commit,
  altes Repo als `openpacketmon-old` privat, Sicherung als Bundle) und umschalten. Entschieden: E-Mail in den Commits
  bleibt, Marke vom Nutzer geprüft (keine Konflikte).
  Ebenfalls entschieden: Geodaten **DB-IP Lite** (Variante A), `CLAUDE.md`/`PLAN.md` öffentlich wie bei opennetmon.
  **4.4 erledigt** (2026-10-29): `NOTICE`, Kopfzeilen in 47 Dateien des Originals, `dev/check-notices.sh`.
  **4.1 erledigt** (2026-10-29): Dienstliste aus der IANA-Registry (CC0) statt nmap,
  `dev/update-services.py` erzeugt `services.txt` und gleicht `categories.txt` ab (11 526 Ports, 6167 Namen;
  z. B. `bootps` statt `dhcps`, `ssdp` statt `upnp`).
  **4.2 erledigt** (2026-10-29): keine eingebauten GeoLite2-DBs mehr; Einstellungen „Geodaten“ laden DB-IP Lite
  (CC BY 4.0) auf Klick nach `~/.local/share/openpacketmon/geo` (`src/mmdb/geo.rs`), sonst `/var/lib/GeoIP` oder eigener
  Pfad; Link „IP Geolocation by DB-IP“ in der Seitenleiste. Testinstanzen mit `XDG_DATA_HOME` im Scratchpad starten.
  **4.3 erledigt** (2026-10-29): IPinfo-Beispieldatenbanken in `resources/test/` (CC BY 4.0) behalten,
  Herkunft in `resources/test/README.md`; übrige Testdaten sind eigen.
  **4.5 erledigt** (2026-10-29): `THIRD-PARTY-LICENSES.txt` aus `dev/third-party-licenses.py` (313 Crates, uPlot, IANA,
  DB-IP, IPinfo), im `.deb` mit `NOTICE` unter `/usr/share/doc/openpacketmon/`, Seite `/licenses` (Link „Lizenzen“).
  Nach Änderungen an Abhängigkeiten neu erzeugen und mitcommitten (`dev/build-deb.sh` hält sonst an).
  **4.6 erledigt** (2026-10-29): `deny.toml`, `cargo deny --workspace check` (Lizenzen, Quellen,
  Sicherheitsmeldungen; in der CI statt `cargo audit`), CI prüft auch `THIRD-PARTY-LICENSES.txt`.
  **4.7 erledigt** (2026-10-29): `SECURITY.md`, `CONTRIBUTING.md` (u. a. kein Wireshark-Code, KI-Hilfe offenlegen, DCO),
  PR- und Issue-Vorlagen, CI-Job `dco`; README, `CLAUDE.md`, `PLAN.md` durchgesehen, Suchläufe ohne Befund.
  **4.8 erledigt** (2026-10-29): Code-Review, Zufallstests (`src/fuzz.rs`, lang mit `FUZZ_ITERATIONS=1000000
  cargo test --release fuzz`) und 190 Angriffe auf eine Testinstanz (Port 7390, präparierte PCAP). Behoben: Absturz
  im DNS-über-TCP-Zerleger (ein Paket genügte), IPFIX-Vorlagen mit Tausenden leerer Feldern (CPU/Speicher), halbe
  HTTP-Anfragen ohne Timeout (`web::serve` jetzt mit hyper und `header_read_timeout` 10 s), verdrehte Bereiche bei
  kaputten IPv6-Erweiterungen, `back`-Pfade mit Tab, quadratische Favoriten-Prüfung, `xdg-open` nur für http(s).
  Details `PLAN.md` 4.8. Test-Paket **0.2.5** mit diesen Korrekturen gebaut (2026-10-29).
  **4.9 erledigt** (2026-10-29): alte Historie als Bundle in `~/Dokumente/openpacketmon-history-bis-2026-10-29.bundle`,
  altes Repo privat als `tomCSLK/openpacketmon-old`, neues privates `tomCSLK/openpacketmon` mit einem Anfangs-Commit;
  Tag `upstream-base` nur noch im Bundle, Zweig `design-muster` nur lokal. Details `PLAN.md` 4.9.
- **4.10 vorbereitet, Haltepunkt** (2026-10-30, `PLAN.md` 4.10): Der IPFIX-Befund steckt weiter in Sniffnet und wurde an
  `security@sniffnet.app` gemeldet. **Repo bleibt privat, bis Sniffnet geantwortet hat oder ein Termin vereinbart ist.**
  `.deb` mit `depends = "$auto, libcap2-bin"`, neues `dev/release.sh` (Release-Titel `vX.Y.Z`, sonst bleibt die
  Update-Prüfung stumm; noch nicht ausgeführt), README und CHANGELOG für das erste Release überarbeitet.
  Commit `55f5213`, CI grün; Test-Paket **0.2.7** (nur die korrigierten Abhängigkeiten dazu). Entschieden: nach der Antwort
  von Sniffnet umschalten und **0.3.0** veröffentlichen. Die vollständige Liste der offenen Schritte steht in `PLAN.md` 4.10.
  Nutzertest 0.2.7 läuft seit 2026-10-30. Herkunft der Vorlage `../design-system` geklärt: eigene Idee, mit Claude Design erstellt.
  Nutzertest vorläufig abgeschlossen (2026-10-31). Aus dem Test umgesetzt (2026-10-31, `PLAN.md` Phase 3 Punkte 17, 18, Test-Paket
  **0.2.8**): **Zurück-Knopf** links neben dem Titel aller Seiten (`static/app.js`: Schritte durch den Verlauf, überspringt Einträge
  gleicher Adresse nach Formularen; Blättern mit `data-replace` ohne eigenen Schritt; Alt+Links; im Fenster-Programm noch nicht geprüft)
  und **Statistik-Knopf auf der Seite Verbindungen**: `c=` (Verbindungsfilter) neben `f=` auf `/statistics`, `/packets`, `/api/packets`,
  `/packets.pcap` (`packets::Selection`, `FilterIndex::with_connections`). Der Zurück-Knopf ist seit dem Test ein Pfeil ohne Rahmen im
  Rand links neben dem Titel. **Vorschläge in den Filterfeldern** (Punkt 19, `static/suggest.js`, `src/view/suggest.rs`,
  `/api/filter/names`, `/api/filter/values`): Felder, Operatoren, Verknüpfungen, Erklärungen (`filter-hint-*`), im Verbindungsfilter echte
  Werte. Enter ohne gewählte Zeile wendet weiter den Filter an. Test-Paket **0.2.9**, im Fenster-Programm noch nicht geprüft.
- **Mitschnitt-Helfer `openpacketmon-capture`** (2026-11-01, `capture/`, `src/networking/types/capture_helper.rs`, `PLAN.md` 5.5
  „Stand Helfer“, Test-Paket **0.3.0**, kein Release): Nur er hat die Rechte, gibt sie nach dem Öffnen des Adapters ab und reicht Pakete über eine Pipe weiter;
  `openpacketmon` läuft ohne Sonderrechte. Das `.deb` setzt `setcap` auf den Helfer. Live-Test `capture/tests/live.rs` im Netz-Namensraum.
- **Seitendaten in `src/view/`** (2026-11-01, `PLAN.md` 5.5 „Stand Seitendaten“): Alles, was die Seiten zeigen, liegt ohne Webserver, HTML und
  Adressen in `src/view/` (Links als `Jump`, Zeiten als `Timestamp`), `src/web/` rendert nur noch und behält Handler, Formulare und Live-Protokoll. Keine sichtbare
  Änderung: 631 Seiten je Sprache vor und nach dem Umbau gleich. Neue Seitendaten gehören nach `src/view/`; der Test `the_view_knows_nothing_of_the_browser_pages` hält die Grenze.
- **Umstellung auf egui** (Wunsch des Nutzers, 2026-10-31, `PLAN.md` 5.5, erledigt 2026-11-01): native Oberfläche statt Browser-Technik.
  Entschieden: **Wireshark-Muster** (Fenster schließen beendet das Programm, mit Rückfrage; kein Infobereich, kein Autostart) und
  **erstes Release 0.4.0 oder 0.5.0** statt 0.3.0; der Haltepunkt gilt unverändert. Ebenfalls entschieden: **Plan B** mit kleinem
  Mitschnitt-Helfer und egui-Programm ohne Rechte, ohne Webschicht; Paketspeicher bleibt im RAM mit wählbarer Grenze. Probebau im Scratchpad bestanden (flüssig mit 2 Millionen
  Zeilen, 0,4 % CPU im Leerlauf, Ergebnis in `PLAN.md` 5.5): wgpu und eingebaute Schrift Inter; Bedienung von Hand steht noch aus.
- **Reihenfolge** (Nutzer, 2026-10-31, zweimal geändert 2026-11-02): MCP-Anbindung (erledigt) → **Windows-Version** (vorgezogen, weil andere Tester sie brauchen; `PLAN.md` 5.3, `docs/windows.md`) → erstes Release (0.4.0 oder 0.5.0, wartet am Haltepunkt). Der Haltepunkt gilt auch für Windows.
- **Windows-Rechner eingerichtet** (2026-11-02, `docs/windows.md` 1): Rust, cargo-deny, Npcap 1.90 (WinPcap-Kompatibilitätsmodus) und Npcap SDK (`%USERPROFILE%\npcap-sdk`, Benutzervariable `LIB`) nachinstalliert. Erster `cargo check --workspace` im Zweig `windows`: ein Fehler (`libc::timeval` in `capture_helper.rs`, behoben), jetzt grün mit vier Warnungen über Linux-Code; `cargo build --workspace` linkt, beide Programme starten; `cargo test --workspace` nach drei Korrekturen grün (386 bestanden; `geo_dir` unter Windows `%LOCALAPPDATA%\openpacketmon\geo`, offene Datei im Test geschlossen, MCP-Umschalttest wartet bis zur ersten Antwort). `cargo clippy -- -D warnings` unter Windows grün. Gefunden: Mitschnitt-Helfer erbten unter Windows das Handle importierter PCAP-Dateien (`docs/windows.md` 4 „Helfer erbt offene Dateien“); gelöst mit „Stop wartet“ (nächster Punkt). Smart App Control blockiert ab und zu frisch gebaute Dateien, dann neu linken (`docs/windows.md` oben). Release-Fassung baut (2026-11-02): ohne Konsolenfenster (`windows_subsystem` nur im Release, Helfer mit `CREATE_NO_WINDOW`), mit Icon und Versionsangaben (`dev/make-ico.py`, `winresource` in `build.rs`); Kommandozeilen-Ausgaben dort nur mit `| Out-Host`. Fenster startet, Bedienung und Mitschnitt noch nicht ausprobiert.
- **Windows Stufe A und Installer** (2026-11-02, freigegeben, nicht committet, `PLAN.md` 5.3, `docs/windows.md`): Adapter mit Beschreibung statt `\Device\NPF_{…}`; `Engine::stop` wartet, bis der Mitschnitt seine Dateien geschlossen hat (`CaptureEnded`), importierte PCAP-Datei sofort frei, Tests räumen auf und starten keine echten Helfer (`capture_helper::tests::REAL_HELPERS`); `wpcap.dll` verzögert geladen, ohne Npcap Dialog (`src/npcap.rs`); C-Laufzeit statisch (`.cargo/config.toml`); Helfer-Live-Test auf Npcap-Loopback. Unter Windows zeichnet das Fenster mit DirectX 12, sonst OpenGL (mit Vulkan blinkte es bei Mausbewegungen, Intel Arc; `WGPU_BACKEND` gilt weiter). **Installer:** Inno Setup 6, `resources/packaging/windows/openpacketmon.iss`, gebaut mit `powershell -ExecutionPolicy Bypass -File dev\build-installer.ps1` (Version aus `Cargo.toml`, nennt der Nutzer), standardmäßig nur für den Benutzer; Tester-Anleitung `docs/windows-tester.md`. `THIRD-PARTY-LICENSES.txt` deckt jetzt Linux und Windows ab, `deny.toml` prüft beide Ziele. Offen: Mitschnitt im Fenster (Nutzer), sauberes System (Windows-Sandbox), CI-Job Windows, MCP-Integrationstests unter Windows nicht von den echten Einstellungen getrennt (`confy` ignoriert dort `XDG_CONFIG_HOME`). Systemsprache: unter Windows über `sys-locale` (2026-11-02). Mitschnitt im Fenster vom Nutzer getestet, bisher ohne Probleme. **Erste Tester-Fassung 0.3.11** (Windows-Installer, 2026-11-02, SHA-256 in `docs/windows.md` 6). MCP unter Windows geprüft (alle Lesewerkzeuge, `mcp-access.json` nur für Benutzer, SYSTEM und Administratoren lesbar).
- **Nutzertest 0.3.10** (2026-11-02): Bedienung über längere Zeit, Mitschnitt mit BPF-Filter, Ton, Dateidialoge, Skalierung, Geo-Download und die grauen Adapter ohne Befund. README beschreibt jetzt, wie man den Mitschnitt-Helfer auf eine Gruppe beschränkt (im Container geprüft).
- **MCP-Anbindung** (`PLAN.md` 5.4): Claude soll den Mitschnitt abfragen können, etwa beim Untersuchen von Geräten im Heimnetz.
  Paketinhalte sind dabei ungeprüfte Eingaben für das Sprachmodell. Von Anfang an plattformneutral schreiben, damit sie unter
  Windows ohne Umbau läuft.
- **Windows-Version** (begonnen 2026-11-02, vor dem ersten Release, enthält MCP): Alles für die Fortsetzung auf einem Windows-Rechner
  steht in `docs/windows.md` (Software, Einrichten, Linux-Bindungen im Code, Tests, Installer, Code-Signatur, CI, Reihenfolge).
  Der Installer soll, wenn möglich, signiert werden: zuerst SignPath Foundation, sonst Certum Open Source; ein OV-Zertifikat ist
  dem Nutzer zu teuer (2026-11-02). **Signatur angehalten** (2026-11-02): Smart App Control der Tester blockiert den unsignierten
  Installer, die kostenlosen Wege brauchen ein öffentliches Projekt; der Nutzer klärt den Haltepunkt ab (`docs/windows.md` 6.1).
- **Hinweis zur Oberflächentechnik** (Stand 2026-10-31, Hintergrund der Umstellung aus `PLAN.md` 5.5): Die Craft-Programme (WordCraft, DeckCraft,
  GridCraft, PdfCraft, PhotoCraft u. a. sowie der ArtCraft Launcher, Organisation `github.com/storytold`, MIT OR Apache-2.0) sind Rust-Programme
  mit **egui + eframe 0.37** (Fenster über `winit`, Zeichnen über `egui-wgpu`/`egui_glow`; egui und winit aus eigenen Forks mit festen Commits;
  geprüft in der jeweiligen `Cargo.toml` auf `main`). Laut deren READMEs laufen sie nativ unter Linux, Windows, macOS, FreeBSD und im Browser
  (WebAssembly); nicht selbst getestet, Stand „early alpha“.
  Zum Entwickeln mit egui gibt es den MCP-Server `egui_mcp` (Rerun, `github.com/rerun-io/egui_crates`, MIT/Apache; steuert eine laufende
  egui-App über `eframe/inspection`); er ist **nicht** installiert und wird nur auf ausdrücklichen Wunsch eingerichtet.
- **Bedienung nachgebessert** (2026-10-30, `PLAN.md` Phase 3 Punkt 16): Verbindungsseite ohne
  Knopfleiste, Sprünge direkt in den Zeilen („Pakete 77 · anzeigen · Statistik“, `Fact.links`); Paketdetails mit
  Blätter-Gruppe und „Verbindung: Pakete, Statistik, Stream, Details“; Hinweis auf die Verbindung über Pakete und
  Statistik (`packets::filtered_connection`, `templates/connection_note.html`); Richtungsumschalter von Stream folgen
  repariert; Keep-Alives auch ohne gesehenen Verbindungsaufbau, neuer Befund `tcp.analysis.keep_alive_ack`.
  Test-Paket **0.2.6** gebaut.
- Name **openpacketmon** und Logo festgelegt (2026-10-28). Nächste Schritte: Nachbesserungen aus dem Nutzertest (`PLAN.md` Phase 3),
  Phase 4 (vor dem Öffentlich-Schalten), Ideen aus `PLAN.md` 5.2 – jeweils erst nach Freigabe. Phase 5.1 **„kleines Wireshark“** ist mit Stufe 4 abgeschlossen.
- Wireshark ist GPL: daraus nie Code, Zerleger oder Filtergrammatik übernehmen.

## Arbeitsregeln

- Erst planen, Umsetzung nur nach ausdrücklicher Freigabe durch den Nutzer.
- Committen, Pushen und Pakete bauen nur auf Ansage des Nutzers; die Version nennt der Nutzer. Nach einer Änderung Doku
  nachziehen, lokal prüfen und berichten, nicht auf die CI warten.
- **Repo nicht öffentlich schalten und nichts veröffentlichen, solange der Haltepunkt in `PLAN.md` 4.10 gilt** (auch auf
  anderen Rechnern).
- **Nichts auf GitHub anlegen oder hochladen, bevor Name und Logo feststehen.** Repo zuerst privat (kein GitHub-Fork, siehe `PLAN.md` Phase 1).
- Vor dem Öffentlich-Schalten `PLAN.md` Phase 4 vollständig abarbeiten (u. a. GeoIP zur Laufzeit, Fremdlizenzen, Sicherheitstest).
- Oberfläche: Farben und Aufbau wie opennetmon, in `src/gui/theme.rs`. Neue Seitendaten gehören nach `src/view/`, Zeichnen nach `src/gui/`; Texte nur über `locales/`.
- Upstream-Fixes nur gezielt übernehmen (`git format-patch`/`git am` aus einem temporären Upstream-Klon; Bezug ist
  Upstream-Commit `ab4bf739d464`, den Tag `upstream-base` gibt es nur noch im Bundle), nie die Upstream-Historie einspielen.
- Heimnetz-Details und Lokales nur in `CLAUDE.local.md` (gitignored), wie bei opennetmon.
- Lizenz- und Copyright-Hinweise des Originals nie entfernen; geänderte Upstream-Dateien kennzeichnen (Apache-2.0 §4b):
  Wer eine bisher **unveränderte** Datei des Originals ändert (oder Code daraus in eine neue Datei übernimmt), setzt oben
  die zwei Zeilen „Modified for openpacketmon (2026), see NOTICE. Based on `<pfad>` of Sniffnet …“ (wie in den übrigen
  Dateien) und trägt die Datei in `NOTICE` ein; Dateien ohne Kommentarmöglichkeit nur in `NOTICE`. Prüfen mit
  `dev/check-notices.sh` (klont das Original bei Commit `ab4bf739d464`; findet keine Umbenennungen/Kopien; neu
  geschriebene Dateien an Pfaden des Originals stehen dort in der Liste `replaced`).
- Npcap niemals mitliefern (proprietär). Keine Fremd-Assets ohne geklärte Lizenz hinzufügen.
- Beiträge an upstream nur klein und mit Offenlegung der KI-Hilfe (siehe `CONTRIBUTING.md` des Originals).
- Nach jeder Änderung `PLAN.md` und den Status hier sowie in `~/Claude/Projekte/CLAUDE.md` nachziehen.

## Upstream-Eckdaten

- Autor Giuliano Bellini, Release v1.5.1 (2026-07-22), geprüft `main` @ `ab4bf73` (2026-10-20).
- Rust Edition 2024, Workspace mit `lib/sniffnet-packet-parser`; Build unter Linux braucht `libpcap-dev` und `libasound2-dev` (Ton).
- Prüfen: `cargo deny --workspace check` (cargo-deny 0.20.2; ohne `--workspace` fehlt `openpacketmon-capture`), `cargo test --workspace`, `cargo clippy -- -D warnings` (ohne `--all-targets`, wie Upstream; der Testcode hat viele Pedantic-Meldungen), `cargo fmt --all -- --check`.
- Paket: **`dev/build-deb.sh`** (2026-10-29) baut alles mit der Version aus `Cargo.toml` und nennt den Installationsbefehl.
  **Versionen wie bei opennetmon** (Wunsch des Nutzers): `0.x.y` ohne Datum, vor jedem neuen Test-Paket in `Cargo.toml`
  und `capture/Cargo.toml` erhöhen, danach `cargo update --workspace --offline` (sonst scheitert die Lizenzprüfung im Skript an
  `Cargo.lock`) – `y` bei Kleinigkeiten, `x` bei größeren Schritten (erstes Release wird `0.4.0` oder `0.5.0`, siehe `PLAN.md` 5.5;
  Ablauf mit `dev/release.sh` in `PLAN.md` 4.10). Das Skript hält an, wenn die
  Version nicht neuer als die installierte ist, weil `apt install ./….deb` ein Paket gleicher Version stillschweigend
  überspringt („… wird an Stelle von … gewählt“); `--force` baut trotzdem, dann `apt install --reinstall`. Von Hand:
  `cargo build --release --workspace && cargo deb` (`cargo deb` allein baut `openpacketmon-capture` nicht neu). Jeder `cargo build`
  entfernt die `setcap`-Rechte des Binarys (Live-Mitschnitt aus `target/` dann nur nach `sudo setcap cap_net_raw,cap_net_admin=eip target/…/openpacketmon-capture`; das Hauptprogramm braucht keine Rechte).
