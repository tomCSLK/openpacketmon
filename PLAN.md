# PLAN.md – Fork von Sniffnet

**Stand: 2026-11-10.**

- **v0.4.0** ist das erste öffentliche Release (2026-11-02, Linux; signierter Windows-Installer seit 2026-11-03), Repo https://github.com/tomCSLK/openpacketmon
  öffentlich. Oberfläche egui (5.5), Mitschnitt-Helfer mit Rechten, MCP-Anbindung (5.4), Windows (5.3, Reste in `docs/windows.md` 8).
- **Test-Paket 0.5.0** (2026-11-10, Commit „Version 0.5.0; …“, nicht veröffentlicht) mit der neuen Oberfläche (5.6) und VoIP (5.7);
  Test durch den Nutzer steht aus.
- **Neue Oberfläche im Stil von opensecmon (5.6) umgesetzt** (2026-11-10, Stufen A–D nach dem abgenommenen Entwurf; Abweichungen dort).
- **VoIP** (5.7, „go“ am 2026-11-10): SIP, SDP, RTP und RTCP in Paketliste und Filter, unter Statistik der Reiter „Anrufe“ mit Ablauf,
  Sprachqualität und Anhören von G.711; Entwurf abgenommen. **Stufen 1–5 umgesetzt** (Zerleger, Zuordnung zu Anrufen, Filter,
  Zeilenfarbe, Protokollhierarchie; Reiter „Anrufe“, Seite „Anruf“, Einstellungen → Anrufe; Anhören von G.711 mit Wellenform, WAV und
  Hinweis zu § 201; MCP-Werkzeug `calls`, Rufnummern über MCP auf die letzten zwei Ziffern gekürzt, Ton nie über MCP). Als Nächstes
  der Test mit einem echten Anruf.
- **Offen außerhalb:** Pull Request mit der Korrektur des Befunds aus 4.8 beim Original (Einzelheiten nur in `CLAUDE.local.md`).
- Die Abschnitte 2 bis 4 und die Stände in 5.1 bis 5.5 sind **Verlauf** (die Lizenzprüfung in 1 gilt weiter); was dort über Browser-Oberfläche, `src/web/`, `templates/`,
  `static/`, Fenster-Programm oder privates Repo steht, gilt nicht mehr.

Upstream: https://github.com/GyulyVGC/sniffnet – Autor Giuliano Bellini, letztes Release **v1.5.1** (2026-07-22),
geprüfter Stand `main` @ `ab4bf73` (2026-10-20).

> Hinweis: Die Lizenzprüfung ist eine sorgfältige technische Auswertung der Lizenztexte, keine Rechtsberatung.

---

## 0. Entscheidungen des Nutzers (2026-10-28)

| Thema | Entscheidung |
|---|---|
| GitHub | Zuerst **privates** Repo, später öffentlich schaltbar. **Öffentlich seit 2026-11-02** (4.10). |
| Lizenz | **Wie das Original: `MIT OR Apache-2.0`**, auch für eigene Änderungen. |
| Funktionsumfang | **Wie das Original** (Liste in Abschnitt 2.1). Bewusst entfernt (2026-10-29): Mini-Ansicht, Benachrichtigungen nach Datenmenge, Blacklist und Webhook, Favoriten für Dienste, die **IP-Blacklist** ganz. |
| Oberfläche | **Komplett neu, im Stil von opennetmon.** Fonts und Icons des Originals sind damit irrelevant. **Neu geplant (2026-11-09): im Stil von opensecmon** (5.6). |
| Einsatzort | **Auf dem Client** installiert (Arbeitsrechner), nicht auf dem Server. **Keine Server-Installation vorgesehen** (2026-10-28): Bedienung nur am selben Rechner, keine Handy-Darstellung nötig. |
| Name | **openpacketmon** (2026-10-28). Crate, Binary, Paket und GitHub-Repo `tomCSLK/openpacketmon`. Markenregister prüft der Nutzer. |
| Logo | **`~/Bilder/openpacketmon_Logo_v2.jpeg`** (2400×1792, dunkler Hintergrund: Netz-Kugel mit Paket und Pulslinie, Schriftzug „openpacketmon – PACKET MONITORING“). Original bleibt außerhalb des Repos; `_v1` ist verworfen. Mit der neuen Oberfläche ersetzt durch **Logo C „Lupe über Paketen“** (5.6). |
| Verhältnis zu upstream | **Abspalten** (eigenständige Weiterentwicklung, kein regelmäßiges Mergen). Erweiterungen planen wir gemeinsam. |
| Plattform | **Zunächst nur Linux** (Zorin OS / Ubuntu, `.deb`). **Windows später vorgesehen** (2026-10-28; Npcap installiert der Nutzer selbst, wird nie mitgeliefert); macOS offen. **Windows seit 2026-11-02** (5.3, signierter Installer). |
| UI-Variante | **A – lokaler Dienst + Browser** (Abschnitt 3); **dazu B – eigenes Fenster mit Symbol im Infobereich** (beschlossen und umgesetzt 2026-10-28, Abschnitt 4 „Fenster-Programm“). **Ersetzt am 2026-11-01 durch die native egui-Oberfläche** (5.5). |
| Git-Historie | **Ohne Upstream-Historie** starten; erster Commit = unveränderter Stand `ab4bf73` mit Quellenangabe. |
| opennetmon-Dateien | **Freigegeben:** `app.css`, Templates, Skripte u. Ä. aus opennetmon dürfen im Fork unter `MIT OR Apache-2.0` stehen (Fremddateien wie uPlot behalten ihre Lizenz). |

---

## 1. Lizenzprüfung

### 1.1 Ergebnis in Kürze

| Frage | Antwort |
|---|---|
| Darf man forken, ändern, weiterentwickeln? | **Ja**, ohne Einschränkung. |
| Darf man privat und später öffentlich auf GitHub veröffentlichen, auch mit Paketen? | **Ja**, mit Lizenz- und Copyright-Hinweisen. |
| Darf der Fork „Sniffnet“ heißen und das Logo nutzen? | **Nein** (nicht empfohlen) → eigener Name und Logo (1.3). |
| Was muss vor dem **Öffentlich-Schalten** erledigt sein? | `services.txt` (nmap-Lizenz) ersetzen, GeoLite2 klären, Sounds klären/ersetzen, Drittlizenzen beilegen (1.4–1.5). |

### 1.2 Sniffnet selbst: `MIT OR Apache-2.0`

Belege: `LICENSE-MIT`, `LICENSE-APACHE`, `Cargo.toml` (`license = "MIT OR Apache-2.0"` für den ganzen Workspace inkl.
`lib/sniffnet-packet-parser`), `resources/packaging/LICENSE` („dual licensed … you may choose“). Copyright „2022 – Bellini Giuliano“.
Keine `NOTICE`-Datei, kein CLA, keine Zusatzklauseln.

Pflichten:

- **MIT:** Copyright-Vermerk und Lizenztext beibehalten.
- **Apache-2.0:** Lizenztext beilegen (§4a), geänderte Dateien sichtbar als geändert kennzeichnen (§4b),
  vorhandene Copyright-Hinweise behalten (§4c). §6: keine Markenrechte.

Umsetzung im Fork: `LICENSE-MIT` und `LICENSE-APACHE` behalten und um den eigenen Copyright-Vermerk ergänzen
(„Copyright (c) 2022 Giuliano Bellini; Copyright (c) 2026 Thomas Cieslik“), README-Abschnitt „Based on Sniffnet by Giuliano Bellini“,
übernommene und geänderte Dateien mit Kopfzeile kennzeichnen.

**Übernahme aus opennetmon (AGPL-3.0):** `app.css`, Templates, Icons und Skripte von opennetmon dürfen in den MIT/Apache-Fork
übernommen werden, weil der Nutzer laut `git shortlog` **alleiniger Autor** von opennetmon ist (113 Commits, nur `tomCSLK`) und
seine eigenen Dateien zusätzlich unter MIT/Apache freigeben kann. Fremdanteile darin behalten ihre Lizenz: uPlot (MIT),
Fluent-Crates (Apache/MIT). Keine Dateien übernehmen, die von Dritten stammen, ohne deren Lizenz zu prüfen.

### 1.3 Name, Logo, Online-Dienste des Originals

- Marke „Sniffnet“: bei Websuche keine Eintragung gefunden (keine Garantie). Unabhängig davon eigener Name, eigenes Logo;
  Markenregister (DPMA, EUIPO, USPTO) prüft der Nutzer wie bei opennetmon selbst.
- Im Fork umstellen oder entfernen: Update-Prüfung gegen `api.github.com/repos/GyulyVGC/sniffnet` (`src/utils/check_updates.rs`),
  Links auf sniffnet.app/Wiki/Roadmap/Socials, `.github/FUNDING.yml`, `adsfund.json`, `giscus.json`, `.well-known/`, SignPath-Hinweis,
  `info@sniffnet.app`, Paket-Metadaten und Maintainer, `resources/repository/*`, `resources/thesis.pdf`, `resources/audits/*`.

### 1.4 Mitgelieferte Fremddaten

Alle sind per `include_bytes!` bzw. `build.rs` fest ins Binary eingebaut. Durch die neue Oberfläche **entfallen** iced-Fonts
(Sarasa Gothic, `icons.ttf`), Kategorie-Icons und Länderflaggen des Originals.

| Bestandteil | Lizenz | Bewertung | Maßnahme |
|---|---|---|---|
| `resources/services/services.txt` (Port → Dienst, aus `nmap-services`) | **Nmap Public Source License 0.95** (Copyleft) | **mittel bis hoch.** NPSL §3 zählt Programme, die Nmap-Datendateien „lesen oder enthalten“, als abgeleitete Werke; der Dateikopf verlangt eine verträgliche Open-Source-Lizenz. | Solange das Repo privat ist, unkritisch. **Vor dem Öffentlich-Schalten** durch die IANA Service Name and Port Number Registry ersetzen. Folge: die nmap-Einträge zu Trojanern/Würmern fehlen dann (Funktionsumfang leicht kleiner). |
| `resources/DB/GeoLite2-*.mmdb` (MaxMind) | CC BY-SA 4.0 + GeoLite-EULA (2026-02-12) | **mittel.** Namensnennung Pflicht; §6.1 Pflichten an Empfänger weitergeben; §6.3 neue Versionen binnen 30 Tagen nutzen. | Nicht ins Binary einbauen, sondern zur Laufzeit laden: eigener kostenloser MaxMind-Account + Download in der App bzw. per `geoipupdate`, oder IPinfo Lite (CC BY-SA 4.0, vom Code in `src/mmdb` schon lesbar). Attribution in „Über“ und README. |
| `resources/sounds/*.mp3` | **nicht dokumentiert** | unklar | Durch CC0-Sounds ersetzen oder im Browser über die Notification-API ohne eigene Sounds arbeiten. |
| Länderflaggen (HatScripts circle-flags) | MIT | gering | Falls in der neuen UI genutzt: Hinweis beilegen. |
| Themes (Dracula, Nord …) | nur Farbwerte | gering | entfällt mit neuer UI |

### 1.5 Rust-Abhängigkeiten

`cargo metadata --locked`: 701 Pakete, durchweg permissiv (MIT/Apache, BSD, ISC, Zlib, Unicode-3.0, BSL-1.0, CC0, CDLA-Permissive-2.0);
MPL-2.0 nur `symphonia*`, `freedesktop_entry_parser`, `option-ext` (unverändert nutzen = unkritisch). Kein Ausschlussgrund.
Mit der neuen UI fallen iced, plotters, rodio/symphonia usw. weg; dazu kommen die opennetmon-Crates (axum, askama, rust-embed, fluent …).
Für Pakete: `THIRD-PARTY-LICENSES.txt` automatisch erzeugen (Skript wie `../opennetmon/dev/third-party-licenses.py` oder `cargo-about`),
`cargo deny check licenses` in der CI.

### 1.6 Native Bibliotheken

`libpcap` (BSD) unkritisch. Windows: **Npcap** ist proprietär, nie mitliefern (Nutzer installiert selbst).

---

## 2. Bestandsaufnahme des Originals

### 2.1 Funktionsumfang (zu übernehmen)

Aus README und `src/`:

1. Netzwerkadapter wählen, Aufzeichnung starten/stoppen; PCAP-Datei importieren, Aufzeichnung als PCAP exportieren
2. Filter: BPF-Ausdruck, IP-Version, Protokoll, Ports/Adressen
3. Übersicht: Statistiken (Bytes/Pakete ein/aus, verworfene Pakete), Echtzeit-Diagramm der Verkehrsintensität
4. Hosts: Domain (Reverse-DNS), ASN, Land; lokales Netz erkennen; Bogon-Adressen
5. Dienste: 6000+ Dienste über Port (siehe 1.4 zur Datenquelle), Kategorien
6. Programme: welcher Prozess erzeugt den Verkehr (`listeners`-Crate)
7. Verbindungen durchsuchen und im Detail ansehen („Inspect“), Sortierung, Suche
8. Favoriten für Hosts, Dienste, Programme (seit 2026-10-29: Hosts, Programme und IP-Adressen, keine Dienste)
9. Benachrichtigungen: Schwellwerte (Bytes/Pakete), Favoriten, IP-Blacklist; Protokoll; Remote-Benachrichtigung per Webhook
   (seit 2026-10-29 nur noch Favoriten, Phase 3 Punkt 11)
10. IP-Blacklists importieren – auf Wunsch des Nutzers entfernt (2026-10-29, Phase 3 Punkt 14)
11. Mini-Ansicht im Hintergrund („Thumbnail“) – auf Wunsch des Nutzers entfernt (2026-10-29)
12. IPFIX-Export (`src/networking/ipfix`), Latenz/Ping
13. Einstellungen dauerhaft speichern; Sprachen (14 im Original; im Fork zuerst DE/EN wie opennetmon)
14. CLI-Optionen (`src/cli`)

### 2.2 Code-Aufteilung und was bleibt

| Modul | Zeilen | Im Fork |
|---|---:|---|
| `networking` (Capture, Parsing, Hosts, Dienste, Programme, IPFIX) | 10 500 | **übernehmen** (Kern) |
| `lib/sniffnet-packet-parser` | 960 | **übernehmen** |
| `mmdb`, `countries` | 1 600 | übernehmen, Laden auf Laufzeit umbauen |
| `notifications`, `report`, `utils`, `cli` | 2 500 | übernehmen, an neue UI anpassen |
| `gui` (iced), `chart` (plotters) | 14 500 | **ersetzen** durch Web-UI im opennetmon-Stil |
| `translations` (Rust-Code, 14 Sprachen) | 5 100 | ersetzen durch Fluent-Kataloge (DE/EN) wie opennetmon |

Das Original koppelt Kern und iced-Oberfläche über den Zustand `Sniffer` und iced-Messages; diese Kopplung muss beim Umbau
aufgelöst werden (Kern liefert Daten über Kanäle/geteilten Zustand, UI liest sie). Das ist der größte Einzelaufwand.

### 2.3 Technik

Rust Edition 2024, `unsafe_code = "forbid"`, Clippy pedantic, 243 Tests, Workflows `rust.yml`, `package.yml`, `codeql.yml` u. a.
Pakete `.deb`/`.rpm` mit `setcap cap_net_raw,cap_net_admin`. Git-Historie 348 MB (vor allem alte MMDB-Stände und Bilder).

---

## 3. Architektur der Oberfläche (Verlauf)

Am 2026-10-28 gewählt: **A – lokaler Dienst + Browser** (axum/askama wie opennetmon, nur auf `127.0.0.1`, Startlink mit Token), noch am selben
Tag ergänzt um **B – eigenes Fenster** (wry/WebKitGTK). Verworfen: iced behalten und nur umstylen. Seit **2026-11-01 ersetzt durch die native
egui-Oberfläche** mit kleinem Mitschnitt-Helfer (5.5); die Abwägung von damals steht in der Git-Historie.

---

## 4. Phasenplan

### Phase 0 – Klären (erledigt)

Erledigt (2026-10-28).

### Phase 1 – Repo anlegen (erledigt am 2026-10-28)

1. GitHub erlaubt **keine privaten Forks öffentlicher Repos** → statt „Fork“-Knopf ein **neues privates Repo** `tomCSLK/openpacketmon` anlegen.
   Später einfach auf öffentlich umschalten. Vorteil: kein „forked from“-Zwang, passt zur Abspaltung.
2. Historie: **ohne die 348-MB-Historie** starten (entschieden). Erster Commit = unveränderter Upstream-Stand `ab4bf73`
   (Commit-Nachricht und README nennen Quelle und Commit), Tag `upstream-base`. Einzelne spätere Upstream-Fixes bei Bedarf
   per `git format-patch`/`git am` übernehmen.
3. Lokal unter Linux bauen und testen (`libpcap-dev` u. a.), Ausgangslage dokumentieren.

Umgesetzt (2026-10-28): Repo `tomCSLK/openpacketmon` privat angelegt (kein Fork, Wiki aus), **GitHub Actions abgeschaltet**
(die Upstream-Workflows bauen sonst bei jedem Push auch für macOS/Windows; wieder einschalten, wenn in Phase 2 eigene Workflows
stehen – `dependabot.yml` kommt dann ebenfalls weg). Lokaler Ordner `Sniffnet_Fork/` (heute `openpacketmon/`) ist das Repo.
Commit `ce5fa2d` = Upstream `ab4bf739d464…` Datei für Datei identisch (605 Dateien, Blob-Hashes verglichen), Tag `upstream-base`;
danach `CLAUDE.md`, `PLAN.md` und `.gitignore`-Zusatz. GitHub warnt beim Push vor `resources/logos/merch/merch-inkscape.svg`
(59 MB, Grenze 100 MB) – fällt in Phase 2 weg.

**Ausgangslage `upstream-base`** (2026-10-28, Ubuntu, Rust 1.99.1, `libpcap-dev` 1.10.4, `libasound2-dev` 1.2.11, `libfontconfig1-dev` 2.15.0):

| Prüfung | Ergebnis |
|---|---|
| `cargo build --locked` | ok, 2:43 min (Debug, kalt); Binary 660 MB mit Debug-Infos |
| `cargo test --locked --workspace` | ok, **256 Tests** (254 `sniffnet`, 2 `sniffnet-packet-parser`), 0 Fehler |
| `cargo fmt --all -- --check` | ok |
| `cargo clippy --locked -- -D warnings` (wie Upstream-CI) | ok |
| `cargo clippy --all-targets -- -D warnings` | **nicht** ok: ~430 Pedantic-Meldungen im Testcode (`unwrap`, `assert_eq!` mit bool …) – Upstream prüft Tests nicht mit Clippy; als Maßstab gilt der Aufruf ohne `--all-targets` |
| `sniffnet --version` / `--help` | `sniffnet 1.5.1`, CLI-Optionen `-a`, `-c`, `-r` |

Die iced-Oberfläche selbst wurde nicht gestartet (Mitschnitt braucht Root bzw. `setcap cap_net_raw,cap_net_admin=eip`).

### Phase 2 – Aufräumen und Kern freilegen (erledigt am 2026-10-28)

1. Name, Paketname, Logo, Links, Update-Prüfung umstellen; Upstream-Spenden-/Werbedateien entfernen (1.3).
   Logo-Dateien wie bei opennetmon aus dem Original ableiten: `static/logo-mark.png` (nur die Kugel, Seitenleiste/Touch-Icon),
   `static/favicon-32.png`/`-64.png` (Kugel vereinfacht prüfen, die feinen Linien verschwimmen klein), `static/logo-wide.jpg`, `docs/assets/openpacketmon-logo.jpg` (README),
   Desktop-Icons für `.deb` (`/usr/share/icons/hicolor/*/apps/openpacketmon.png`).
   Parallelbetrieb mit dem Original sicherstellen: eigenes Konfigurationsverzeichnis (`~/.config/openpacketmon`, nicht mehr
   `~/.config/sniffnet`), eigene Pfade für Binary, `.desktop`-Datei und Icons, keine Paketkonflikte mit dem Debian-Paket `sniffnet`
   (das Original bleibt auf dem Arbeitsrechner als Referenz installiert).
2. Kern (`networking`, Parser, `mmdb`, `notifications`, `report`) von iced lösen: eigene Datenstruktur, die die UI abfragt.
3. iced, plotters, rodio und die alten Übersetzungen entfernen.

Umgesetzt (2026-10-28):

- **2.1 Umbenennen/Aufräumen** (Commit „Rename to openpacketmon …“): Paket/Binary/`.desktop`/Icons/Konfiguration heißen
  `openpacketmon` (`~/.config/openpacketmon/conf.toml`), Version 0.1.0, `publish = false` (auch für den Parser, der seinen Namen
  `sniffnet-packet-parser` als Herkunftshinweis behält). Logo-Ableitungen in `static/`, `docs/assets/`, `resources/logos/icon.png`,
  `resources/packaging/linux/graphics/openpacketmon_*.png` (aus `~/Bilder/openpacketmon_Logo_v2.jpeg`, Ausschnitt Kugel 820×820 ab (202,490),
  breit ab (180,455) 2060×891). Favicons 16/32 px waren nur eine blaue Kugel → am 2026-10-28 ersetzt:
  **`static/favicon.svg`** (eigene vereinfachte Fassung: Ring mit Verlauf, Paket-Würfel, Pulslinie; daraus `favicon-32/-64.png` und
  die Desktop-Icons bis 42 px, gerendert mit Chrome headless), ab 48 px (`logo-mark.png`, Desktop-Icons 48–1024) aus
  **`~/Bilder/openpacketmon_icon_v2.jpeg`** (flache Fassung, 2048², Ausschnitt (120,120)–(1928,1928); vom Nutzer gegenüber `icon_v1`
  gewählt). `resources/logos/` entfällt (Fenster-Icon nicht mehr nötig). Originale liegen nur in `~/Bilder`; Kopien im Projektordner
  sind per `.gitignore` ausgeschlossen.
  Entfernt: Spenden/Website/Audits/Thesis/Merch, Windows-/macOS-/AppImage-/PPA-Pakete, Upstream-Workflows, Issue-Vorlagen,
  Community-Dokumente. Eigener CI-Workflow `ci.yml` (nur Linux: fmt, clippy, test), **Actions wieder eingeschaltet**.
  README/CHANGELOG neu, Copyright-Zeile in `LICENSE-MIT` und `resources/packaging/LICENSE` ergänzt.
  Update-Prüfung gegen `tomCSLK/openpacketmon` (Netztest `#[ignore]`, bis es ein Release gibt).
  `.gitattributes` des Originals behandelte `*.jpg` als Text (hatte das README-Logo um 1 Byte beschädigt) → Bild-/PCAP-Typen jetzt binär.
  Dependabot-PR #1 (geerbt) geschlossen; dessen Branch hat Dependabot selbst entfernt (auf GitHub nur noch `main`).
- **2.2/2.3 Kern freigelegt, alte Oberfläche entfernt** (Commit „Replace the iced UI with a UI-independent engine“):
  `src/gui`, `src/chart`, `src/translations`, Flaggen/Icons/Fonts/Themes/Sounds und die Crates iced, plotters(-iced2), rodio, rfd,
  picon, splines, gag sind weg (701 → **304 Crates**, Quelltext ~35 000 → ~15 500 Zeilen).
  Konfigurationstypen liegen in `src/config/` (Fenster-, Theme-, Sprach- und Skalierungsfelder entfallen; alte Dateien laden trotzdem).
  Neues Modul **`src/engine/`**: `Engine` (Arc<Mutex<EngineState>>, Hintergrund-Threads für Aufzeichnung und Adapter-Vorschau,
  tokio-Handle für Blacklist laden, Latenz, Update-Prüfung) und `EngineState` (übernimmt die Aufzeichnungslogik des alten `Sniffer`:
  Start/Stopp, Capture-ID, Backend-Nachrichten, Hosts/Programme, Benachrichtigungen, Favoriten, Suche mit Seiten).
  `TrafficHistory` (Sekundenwerte, live 1 h, Dateien werden verdichtet) ersetzt die Diagrammdaten, `DevicePreview` die Vorschaugrafen.
  Benachrichtigungstöne spielt der Kern nicht mehr ab, er meldet sie als `sound_event` (Browser entscheidet). Remote-Texte fest Englisch.
  Programm-Icons fallen vorerst weg (`picon` hing an iced); `Program::icon_key` bleibt für Phase 3.
  **Vorläufige Kommandozeile** bis Phase 3: `--adapter`, `--import <pcap>`, `--list-adapters`, Zusammenfassung alle 5 s, Top-Listen am Ende.
  `#![allow(dead_code)]` in `main.rs` ist vorübergehend (die UI nutzt die übrige Engine-API) → in Phase 3 entfernen und Reste löschen.
- **Tests**: 218 + 2 grün (GUI-Tests entfallen; Kern-Tests des `Sniffer` portiert; neu: Verlauf, Vorschau, **Ende-zu-Ende-Test**
  mit selbst erzeugter PCAP-Datei durch die ganze Engine, Fehlerfall). Probelauf `--import` mit echten Adressen: rDNS, ASN, Land,
  Dienste korrekt. Live-Aufzeichnung ohne Rechte meldet sauber „Operation not permitted“; **mit Rechten getestet** (2026-10-28, `enp0s31f6`,
  ~2 min: 41 Hosts aus LAN und Internet, rDNS/ASN/Land, Dienste, Programme `chrome`/`claude`, Strg+C beendet sauber mit Top-Listen,
  die seitdem nach Datenmenge sortiert sind). Jeder `cargo build` ersetzt das Binary und entfernt die mit `setcap` gesetzten Rechte.
- `cargo audit` (2026-10-28): zwei TLS-Meldungen über `reqwest` behoben (`rustls-webpki` 0.103.16, RUSTSEC-2026-0382;
  `webpki-root-certs` 1.0.11, RUSTSEC-2026-0383). CI-Job `audit` läuft bei jedem Push und montags (wie opennetmon).
- Apache-2.0 §4b (geänderte Dateien kennzeichnen) wird vor dem Öffentlich-Schalten erledigt (Phase 4), solange nichts weitergegeben wird.

### Phase 3 – Neue Oberfläche (Variante A, erledigt am 2026-10-28)

1. axum-Server auf `127.0.0.1`, Designsystem und Grundgerüst aus opennetmon übernehmen (Seitenleiste, Kacheln, Tabellen, hell/dunkel, Handy-Ansicht).
2. Seiten nach 2.1: Start/Adapterwahl, Übersicht mit Live-Graphen (uPlot), Verbindungen (Inspect), Hosts, Dienste, Programme,
   Benachrichtigungen, Einstellungen. Live-Aktualisierung per Polling oder Server-Sent Events.
3. Fluent-Übersetzungen DE/EN.
4. Dienst-Einrichtung: `.deb` mit systemd-Unit und `setcap`; Startmenü-Eintrag öffnet den Browser.

Umgesetzt (2026-10-28, freigegeben mit „weiter mit phase 3“):

- **Server** (`src/web/`): axum 0.8 + askama 0.16, nur `127.0.0.1`, Standardport **7380** (`--port`). Zweiter Start erkennt die
  laufende Instanz über `/healthz` und öffnet nur den Browser mit dem gespeicherten Startlink; `--no-browser` für den Dienst.
  Beenden per Strg+C, SIGTERM oder **Einstellungen → openpacketmon beenden** (`POST /quit`); danach Startlink löschen, Konfiguration speichern.
- **Sicherheit**: Zugangsschlüssel (32 Zufallsbytes) im Startlink, gespeichert in `$XDG_RUNTIME_DIR/openpacketmon/start-link-<port>` (0600, Ordner 0700);
  Sitzungen im Speicher (max. 16), Cookie `HttpOnly`/`SameSite=Strict`, CSRF-Token je Sitzung, Host-Prüfung (421, gegen DNS-Rebinding),
  Origin-Prüfung bei Schreibzugriffen (403), CSP `default-src 'self'`, `X-Frame-Options: DENY`, `Referrer-Policy: same-origin`
  (mit `no-referrer` schickt Chrome `Origin: null`), `Cache-Control: no-store`. Statische Dateien per `rust-embed` mit ETag.
- **Seiten**: Aufzeichnung (Adapter mit Aktivitätskurven der letzten 30 s, Datei, IPFIX-Kollektor, BPF-Filter, PCAP-Export; laufend: Quelle,
  Filter, Exportdatei, Link-Typ, Empfangsadressen des Kollektors, verworfene Pakete, Pausieren/Beenden), Übersicht (Kacheln, uPlot-Diagramm,
  Top-5 Hosts/Dienste/Programme), Mini-Ansicht (`/mini`, kleines Fenster), Verbindungen (Suche, Filter, Seiten 25–250, Sortierung,
  Detailseite mit Latenzmessung), Hosts/Dienste/Programme (Suche, Sortierung, Favoriten), Benachrichtigungen (Badge in der Seitenleiste,
  Töne per WebAudio im Browser), Einstellungen (Sprache, Seitengröße, Benachrichtigungen, Webhook, IP-Blacklist, MMDB-Pfade, Update-Prüfung).
  Einheit Bytes/Bits/Pakete in jedem Seitenkopf.
- **Live**: Polling `/api/live` jede Sekunde (nur neue Sekundenwerte über Capture-ID/Generation/since), Tabellen-Fragmente alle 5 s
  (nicht bei Hover/Fokus).
- **Design** aus opennetmon (`static/app.css`, `theme.js`, `app.js`, `time.js`, uPlot 1.6.32 MIT), hell/dunkel, Handy-Ansicht;
  Übersetzungen DE/EN mit Fluent (`locales/`), Tests prüfen gleiche IDs in beiden Sprachen.
- **Aufgeräumt**: `#![allow(dead_code)]` entfernt, nur von der alten Oberfläche genutzter Code gelöscht (u. a. `ThumbnailHost`,
  `Notification`, `CaseInsensitiveString`, `Program::icon_key`, Hilfsfunktionen in `formatted_strings`); reine Test-Helfer
  stehen unter `#[cfg(test)]`. Neue Konfigurationsfelder `language`, `page_size`; Standardname der PCAP-Exportdatei `openpacketmon.pcap`.
- **Paket**: `cargo deb` erzeugt `openpacketmon_0.1.0-1_amd64.deb` mit `/usr/bin/openpacketmon` (postinst setzt `cap_net_raw,cap_net_admin`),
  `.desktop`, Icons und **systemd-User-Dienst** `/usr/lib/systemd/user/openpacketmon.service` (`--no-browser`, ohne `NoNewPrivileges`,
  das die Capabilities aufheben würde). Aktivieren: `systemctl --user enable --now openpacketmon`. Binary 31 MB, davon 21 MB GeoLite2 (Phase 4).
- **Tests**: 244 + 2 grün (Router, Anmeldung, CSRF, Host/Origin, alle Seiten mit und ohne Aufzeichnung, Favoriten, Einstellungen,
  Beenden, Übersetzungen); clippy und fmt sauber. Im Browser (Chrome) geprüft: Anmeldung, Aufzeichnung aus Datei, alle Seiten,
  Benachrichtigungen mit Grenzwert und Favorit, Mini-Ansicht, Englisch, Dunkelmodus, Handy-Ansicht (400 px), Adapterliste.
  Live-Mitschnitt aus der Oberfläche braucht die Capabilities (Debug-Binary: `sudo setcap cap_net_raw,cap_net_admin=eip target/debug/openpacketmon`).

#### Rückmeldungen aus dem Nutzertest (ab 2026-10-28)

Der Nutzer testet das installierte `.deb`. Punkte 1, 3 und 4 umgesetzt am 2026-10-28 (Freigabe „fange jetzt damit an, was am sinnvollsten wäre“); Punkt 2 ist mit Phase 5.1 Stufe 1 erledigt (2026-10-28).

1. ✅ **Zähler „Benachrichtigungen“ in der Seitenleiste** (erledigt: Rechteck mit 5 px Rundung, schrumpft nicht, ab 100 „99+“ mit genauer Zahl im Tooltip; vierstellig passte nicht neben das lange Wort): Ab drei Stellen steht die Zahl nicht mehr im roten Kreis.
   Wunsch: rotes Rechteck mit leicht abgerundeten Ecken statt Kreis. Ansatz: `.nav-badge` in `static/app.css` mit
   `border-radius` von etwa 4–5 px statt `999px`, `flex-shrink: 0` und `white-space: nowrap`, damit der Link das Feld nicht
   zusammendrückt; Breite wächst mit der Zahl. Zu klären beim Umsetzen: ab einer Grenze „999+“ anzeigen (in `templates/app.html`
   und `static/live.js`).
2. ✅ **Rohdaten der Pakete auf der Verbindungsseite, wie bei Wireshark** (Wunsch 2026-10-28; **erledigt mit Phase 5.1 Stufe 1**:
   Knopf „Pakete dieser Verbindung“ führt zur gefilterten Paketliste mit Details, Hex-Dump und PCAP-Download; Entscheidungen
   siehe dort. Der folgende Ansatz war der erste Entwurf). Neue Funktion, geht über das
   Original hinaus (Sniffnet zählt Pakete nur und verwirft die Bytes). Ansatz:
   - Aufzeichnung: In `parse_packets.rs` liegen Zeitstempel, Länge und Bytes jedes Pakets vor (`packet.header`, `packet.data`).
     Je Verbindung einen Ringpuffer der letzten N Pakete mitführen (z. B. 50 Pakete, Bytes je Paket gekappt, Gesamtgrenze
     für den Speicher, älteste Verbindungen zuerst verwerfen) und mit den übrigen Verkehrsdaten an die Engine geben.
   - Detailseite: Paketliste (Nr., Zeit, Richtung, Länge, Kurzinfo wie TCP-Flags/Seq/Ack); aufklappbar je Paket die
     **Schichten** (Ethernet, VLAN, IPv4/IPv6, TCP/UDP/ICMP mit ihren Feldern; Parser `sniffnet-packet-parser`/`etherparse`
     kann das) und der **Hex-Dump** mit ASCII-Spalte (Offset | 16 Bytes hex | Text), Markierung der Schicht beim Überfahren
     als spätere Ausbaustufe.
   - Zusätzlich „Als PCAP herunterladen“ für diese Verbindung (öffnet sich in Wireshark), da die Bytes ohnehin vorliegen.
   - Grenzen: IPFIX-Kollektor liefert nur Flussdaten, dort keine Rohdaten (Hinweis anzeigen). Bei Capture-Dateien
     dieselbe Pufferung beim Einlesen.
   - Datenschutz: Inhalte unverschlüsselter Verbindungen (HTTP, DNS) werden sichtbar; nur im Speicher, nur lokal; Schalter
     in den Einstellungen („Paketinhalte speichern“, Standard an oder aus → klären).
   - Zu klären: Anzahl Pakete je Verbindung, Speichergrenze, ob nur Kopfdaten oder auch Nutzdaten, Standard des Schalters.
3. ✅ **Verbindungstabelle zu schmal, man muss waagerecht schieben** (2026-10-28; erledigt: `.page.wide` bis 1840 px für Übersicht, Verbindungen, Hosts/Dienste/Programme; „–“ mit Tooltip statt „nicht zutreffend“, Gegenstelle/Programm einzeilig mit „…“, Zeiten von heute in Tabellen nur als Uhrzeit). Ursache: `.page` ist aus opennetmon auf
   `max-width: 1180px` begrenzt, auch auf breiten Bildschirmen. Ansatz:
   - Tabellenseiten (Verbindungen, Hosts, Dienste, Programme, evtl. Übersicht) bekommen eine breite Variante (`.page.wide`,
     volle Breite bzw. Grenze um 1800 px); Formularseiten (Einstellungen, Aufzeichnung) bleiben schmal.
   - Tabelle kompakter: „nicht zutreffend“ (Dienst/Programm bei ARP u. ä.) als „–“ mit Tooltip statt zweizeiligem Text;
     Zeitstempel, Adressen und Hostnamen ohne Zeilenumbruch (`nowrap`), Hostnamen notfalls mit „…“ kürzen und vollem Namen
     im Tooltip; ggf. Datum weglassen, wenn es heute ist (nur Uhrzeit).
   - Waagerechtes Schieben bleibt nur für schmale Fenster.
4. ✅ **Handy-Darstellung entfällt** (erledigt: Handy-Regeln, Menüknopf und Text `nav-menu` entfernt; dabei alle nicht genutzten opennetmon-Regeln aus `app.css` entfernt, 768 → ~420 Zeilen; Mindestbreite 760 px, unter 1000 px weniger Innenabstand und einspaltige Formulare; Seitenleiste bleibt 248 px breit, schmaler passt „Benachrichtigungen“ nicht) (Entscheidung 2026-10-28: lokale Anwendung auf dem Client, später auch Windows, keine
   Server-Installation). Ansatz: in `static/app.css` die Regeln für Handys entfernen (`@media (max-width: 640px)`/`560px`,
   Menüknopf `.menu-toggle` mit `app.js`-Teil und Eintrag in `templates/app.html`, Text `nav-menu`); `viewport`-Meta
   bleibt harmlos. Empfehlung: eine **Mindestbreite** für schmale Desktop-Fenster behalten (halber Bildschirm ≈ 700–900 px:
   Seitenleiste schmaler bzw. nur Symbole, Kacheln umbrechen, Tabellen waagerecht schiebbar) und die Mini-Ansicht für kleine
   Fenster. Zusammen mit Punkt 3 umsetzen. Auch die README erwähnt die Handy-Ansicht nicht mehr.
5. ✅ **Startlink je Port** (beim Testen gefunden, 2026-10-28): Zwei Instanzen auf verschiedenen Ports (z. B. installiertes Paket
   auf 7380 und ein Testserver) teilten sich `start-link`; der zweite überschrieb bzw. löschte beim Beenden den Link des ersten.
   Jetzt `start-link-<port>`.
6. ✅ **Übersicht: Top-Listen werden rechts abgeschnitten** (2026-10-29; **erledigt am 2026-10-29**, Freigabe „setz alle drei
   Punkte um“: Top-Listen mit `table-layout: fixed`, Name/Pfad mit „…“ und Tooltip (inkl. Markierungen wie „Bogon“), volle
   Listen brechen Pfade um, gekürzte Prozessnamen aus dem Pfad ergänzt; im Headless-Chrome mit langem Namen/Pfad geprüft,
   kein waagerechtes Schieben mehr): In der Karte
   „Top-Programme“ macht ein langer Pfad ohne Leerzeichen (z. B. `/home/…/.local/share/claude/versions/2.1.312`) die
   Namensspalte so breit, dass die Spalte „Gesamt“ aus der Karte geschoben wird. Man muss waagerecht schieben, die
   Summen sind halb verdeckt. Dasselbe kann bei langen Hostnamen in „Top-Hosts“ passieren. Ursache: `td.wrap` kann
   einen Pfad nicht umbrechen, `.table-wrap` schiebt dann waagerecht (`templates/entry_table.html`, `.cols.top` in
   `static/app.css`).
   Ansatz:
   - In `.cols.top`: `table-layout: fixed` mit festen Breiten für Stern, Balken und Gesamt; die Namensspalte nimmt den
     Rest. Name und Pfad einzeilig mit „…“ (`overflow: hidden; text-overflow: ellipsis; white-space: nowrap`), voller
     Text im Tooltip (`title`). Kein waagerechtes Schieben mehr.
   - Auf den vollen Seiten Hosts/Dienste/Programme den Pfad mit `overflow-wrap: anywhere` umbrechen statt zu schieben.
   - Nebenbefund im Bild: „evolution-calen“ ist der vom Kernel auf 15 Zeichen gekürzte Prozessname. Ist der Name 15
     Zeichen lang und beginnt der Dateiname des Pfads damit, den Dateinamen anzeigen (`evolution-calendar-factory`,
     `program_label` in `src/web/entries.rs`).
   - Prüfen im Headless-Chrome bei schmalen und breiten Fenstern (drei Karten nebeneinander bzw. untereinander), hell/dunkel.
7. ✅ **Violette Seitenleiste, dunkles Design violett getönt** (2026-10-29, erledigt; Freigabe „B gefällt mir, übernimm es nach
   main“): Der Nutzer brachte das Designsystem `../design-system` („Ops Console“: Violett-Skala, Roboto, Material Symbols,
   Pillen-Knöpfe) mit. Zwei Musterseiten im lokalen Zweig `design-muster` (nicht gepusht): erst Farben und Konturen der ganzen
   Vorlage, dann nur die Seitenleiste. Übernommen nur die Farben: **hell** Seitenleiste mit Verlauf violet-800 → violet-950
   (`#5d12af` → `#220f4b`), aktive Seite als helle Kachel (`#efe7f8`) mit violetter Schrift, Fokusring in der Leiste lime;
   **dunkel** Leiste `#2a0b52` → `#0e0620`, Flächen leicht violett getönt und etwas heller (Variante B; Variante A „Flächen wie
   bisher, Leiste fast schwarz“ verworfen). Schrift, Größen, Knöpfe, Karten, Akzent- und Statusfarben bleiben. Neue Variablen
   `--sidebar-active-text`, `--sidebar-hover`, `--sidebar-focus` in `static/app.css`. Zur Übernahme in opennetmon in dessen
   `PLAN.md` beschrieben. Herkunft der Vorlage geklärt (Nutzer, 2026-10-30): eigene Idee, angeregt von den Farben eines
   anderen Programms, mit Claude Design erstellt; hier nur Farbwerte übernommen, keine Assets.

8. ✅ **Test-Pakete mit eigener Version** (2026-10-29): Ein neues `.deb` mit derselben Version `0.1.0-1` wurde von
   `apt install ./….deb` stillschweigend übersprungen (nur „openpacketmon wird an Stelle von … gewählt“); erst `--reinstall`
   half. Lösung nach Wunsch des Nutzers **wie bei opennetmon**: Versionen `0.x.y` ohne Datum, vor jedem neuen Test-Paket in
   `Cargo.toml` erhöhen (`y` Kleinigkeiten, `x` größere Schritte; nächster größerer Sprung `0.2.0`). `dev/build-deb.sh` baut
   den Workspace und das Paket und hält an, wenn die Version nicht neuer als die installierte ist (`--force` baut trotzdem).
   Ein erster Entwurf mit Datum (`0.1.0+JJJJMMTT.HHMM`) wurde verworfen, das Paket davon gelöscht und nie installiert.

**Entscheidungen des Nutzers (2026-10-29) zu den Punkten 9–11:** „à la Wireshark“ ist die **durchgehende Paketliste**
(Variante B); die Verbindungen bekommen **ein einzelnes Filterfeld** wie der Anzeigefilter; Blacklist-Benachrichtigung
und Weiterleitung an eine URL (Webhook) entfallen; Favoriten sind **Hosts, IP-Adressen (neu) und Programme**, Favoriten
für Dienste entfallen; die **Mini-Ansicht entfällt**. Umsetzung am 2026-10-29:

9. ✅ **Verbindungen: Suchbereich zu hoch** (erledigt 2026-10-29): Die elf Einzelfelder und „Nur Favoriten/gesperrte“ sind
   durch **ein Filterfeld** wie bei den Paketen ersetzt (eine Zeile mit Sortierung, Anwenden, Zurücksetzen, aufklappbare
   Hilfe mit Beispielen, Protokollen und Feldern; Live-Prüfung beim Tippen über `/api/filter?kind=connections`, jetzt in
   `static/app.js` für beide Seiten). `ListQuery { f, sort, page, size }` in `src/web/connections.rs`. Grammatik aus
   `src/filter/` mit eigenen Feldern (`Filter::parse_connections`, `CONNECTION_FIELDS`, `CONNECTION_PROTOCOLS`):
   `ip.addr/src/dst`, `remote`, `proto`, `port`, `srcport`, `dstport`, `tcp.port` …, `eth.*`, `vlan.id`, `direction`
   (`incoming`/`outgoing`), `service`, `domain`, `as`, `as.name`, `as.number`, `country`, `program`, `program.name`,
   `program.path`, `exporter`, `packets`, `bytes`, `favorite`, `blacklisted`. Links von Hosts/Programmen/Benachrichtigungen
   führen auf `/connections?f=…` (`FavoriteItem::connection_filter`, `entries::search_query`). Alte
   `SearchParameters` entfernt. Verbindungsseite: Stern an der Adresse der Gegenstelle, Link „Alle Verbindungen dieser Adresse“.
10. ✅ **Durchgehende Paketliste à la Wireshark** (erledigt 2026-10-29): Seiten und „Zeilen pro Seite“ entfallen auf der
    Paketseite. `templates/packets.html` hat einen Scroller (`.vlist`) mit fester Kopfzeile, nur die sichtbaren Zeilen stehen
    im DOM; `static/packets.js` lädt Blöcke (`/api/packets?f=&from=&count=`, höchstens 500 Zeilen, Antwort mit `total`,
    `rows`, `running`), skaliert die Höhe ab 8 Mio. px, folgt beim Mitschnitt am Ende (Abfrage alle 0,3 s), hält die
    Position beim Nachladen, „Gehe zu Paket“ (Strg+G) markiert die Zeile, Strg+Pos1/Ende, Bild-/Pfeiltasten. Der Rückweg aus
    den Paketdetails führt mit `goto=N` an dieselbe Stelle. Die Seitenwahl bleibt für Verbindungen.
11. ✅ **Benachrichtigungen abgespeckt** (erledigt 2026-10-29): Grenzwert (`DataNotification`), Blacklist-Benachrichtigung
    und Weiterleitung an eine URL (`RemoteNotifications`) entfernt; `Notifications { volume, favorite_notification }`,
    `LoggedNotification { favorite, timestamp }`. Favoriten (`src/config/favorite.rs`): Hosts, Programme und neu
    **IP-Adressen** (`addresses`, höchstens 1000), einzutragen in den Einstellungen (Textfeld, ungültige Einträge werden
    gemeldet) oder per Stern auf der Verbindungsseite; Dienste haben keinen Stern mehr (Spalte entfällt). Eine Adresse
    meldet sich, wenn sie Quelle oder Ziel neuen Verkehrs ist. Alte `conf.toml` mit den entfernten Feldern laden weiter.
    Die Blacklist selbst (Import, Filterfeld `blacklisted`) bleibt.
12. ✅ **Mini-Ansicht entfernt** (2026-10-29): Route `/mini`, Template, Knopf, CSS und Texte; im Fenster-Programm der
    Menüpunkt und das zweite Fenster. Außerdem toter Code aus dem Original entfernt (`ByteMultiple`,
    `formatted_string`, `supports_exporters`, `to_string_with_equal_prefix`). 287 + 3 + 2 Tests, im Headless-Chrome hell
    und dunkel geprüft.

13. ✅ **Leisten stimmiger und flacher** (Rückmeldung 2026-10-29 zu 0.2.0, erledigt 2026-10-29): In den Leisten haben Knöpfe,
    Felder und Auswahlboxen verschiedene Höhen. Ursache in `static/app.css`: Eingabefelder `padding .6rem`, Schrift
    `.93rem` (≈ 40 px); Knöpfe `padding .55rem`, `.9rem` (≈ 36 px); `button.small` ≈ 28 px; `select` ohne
    `appearance: none`, daher zeichnet WebKitGTK eine eigene graue Box mit eigener Höhe; Bytes/Bits/Pakete
    (`.segmented`) und „Zeilen pro Seite“ (`label.rows-select`) haben wieder andere Maße; der Kartenkopf hat
    `padding .95rem`. Ansatz (nur CSS und wenige Klassen in den Templates):
    - Zwei feste Höhen als Variablen: `--control-h: 2.25rem` (36 px, Formulare, Filterleiste, Seitenkopf) und
      `--control-h-sm: 1.9rem` (≈ 30 px, alle Leisten in Kartenköpfen). Felder, Auswahlboxen, Knöpfe, Links als Knopf,
      `.segmented` und „Gehe zu Paket“ bekommen `height` statt Innenabstand oben/unten, gleiche Schriftgröße, gleichen
      Radius und Rahmen; `.card-head .card-tools` stellt alles darin auf die kleine Höhe.
    - `select` mit `appearance: none`, Hintergrund und Rahmen wie die Felder und eigenem Pfeil (kleines SVG als
      Hintergrundbild, Farbe passend für hell und dunkel), damit es im Fenster-Programm und im Browser gleich aussieht.
    - Knopf „Anwenden“ in Kartenköpfen gleich hoch wie die Felder daneben (nicht mehr `small` mit eigener Höhe);
      Kontrollkästchen „Nur Favoriten“ mittig dazu.
    - Kartenkopf mit Leiste flacher (`padding .55rem 1.35rem`), Überschrift dort etwas kleiner; Filterleiste über den
      Paketen/Verbindungen auf `--control-h`.
    - Betroffene Seiten: Hosts/Dienste/Programme, Verbindungen, Pakete (Filterleiste, Live, Farben, „Gehe zu Paket“),
      Statistik, Aufzeichnung, Einstellungen, Übersicht (Einheiten-Umschalter).
    - Prüfen im Headless-Chrome **und im Fenster-Programm** (WebKitGTK, dort zeigt sich das graue `select`), hell und
      dunkel: alle Bedienelemente einer Leiste gleich hoch (per Skript `getBoundingClientRect().height` vergleichen),
      Kartenköpfe vorher/nachher messen.
    **Umgesetzt:** Variablen `--control-h` (36 px) und `--control-h-sm` (≈ 30 px) in `static/app.css`; Felder, Auswahlboxen,
    Knöpfe, Links als Knopf und `.segmented` haben feste Höhen statt Innenabstand oben/unten; `select` mit
    `appearance: none` und gezeichnetem Pfeil (zwei Verläufe in `var(--muted)`, kein Bild, also hell/dunkel passend und
    CSP-sicher); alles in `.card-tools`, Seitenfeld der Seitenwahl und „Zeilen pro Seite“ auf der kleinen Höhe; im
    Seitenkopf steht der Einheiten-Umschalter auf der normalen Höhe neben den Knöpfen; Kartenkopf `padding .6rem`
    (Mindesthöhe 3,2rem), `.card-tools` `.55rem`. Gemessen im Headless-Chrome per Skript: in jeder Leiste aller Seiten
    nur noch eine Höhe (Kartenköpfe 30 px, Filterleisten und Seitenköpfe 36 px), Kartenkopf mit Leiste 51 px statt ≈ 70 px.
14. ✅ **IP-Blacklist entfernt** (Frage 2026-10-29, erledigt 2026-10-29). Stand nach 0.2.0: Die Datei aus
    Einstellungen → Daten wird beim Start und beim Speichern im Hintergrund geladen (`IpBlacklist`, IPs und CIDR-Netze);
    bei jedem Paket wird die **Gegenstelle** geprüft (`manage_packets.rs`), das Ergebnis steht an der Verbindung
    (`is_blacklisted`). Sichtbar ist das nur noch (a) als rot hinterlegte Zeile mit Etikett „Blacklist“ in der
    Verbindungsliste, (b) als Hinweis auf der Verbindungsseite und (c) im Verbindungsfilter `blacklisted`. Keine
    Benachrichtigung mehr, nichts in Hosts, Paketliste, Übersicht oder Statistik.
    Gefundene Mängel:
    - **Fehler:** Ist kein Pfad eingetragen, steht trotzdem „Die Datei konnte nicht gelesen werden“ (Bildschirmfoto vom
      2026-10-29), weil `IpBlacklist::is_invalid()` nur prüft, ob die Liste leer ist. Eine lesbare Datei ohne gültige
      Zeile meldet denselben Text.
    - Die Liste wird beim **Start der Aufzeichnung** an den Mitschnitt übergeben (`state.rs`, `spawn_capture_thread`);
      wird sie während einer Aufzeichnung geändert, wirkt das erst bei der nächsten.
    **Entscheidung des Nutzers (2026-10-29): Variante C – Blacklist ganz entfernen** (bewusste Abweichung vom
    Funktionsumfang des Originals; zum Beobachten bestimmter Adressen dienen die Adress-Favoriten). Verworfen: A
    (behalten und ausbauen), B (nur den Fehler beheben). Umsetzung nach Freigabe:
    - Engine/Mitschnitt: `src/networking/types/ip_blacklist.rs` samt Tests und `resources/test/ip_blacklist_*.txt`
      löschen; Parameter `ip_blacklist` aus `capture.rs`, `parse_packets.rs`, `manage_packets.rs`, `ipfix/collect.rs`
      (auch Tests) und `spawn_capture_thread` entfernen; Feld `is_blacklisted` aus `InfoAddressPortPair`;
      `EngineState::ip_blacklist`, `Engine::load_ip_blacklist` und das Laden beim Start (`src/engine/mod.rs`).
    - Einstellungen: Feld `ip_blacklist` aus `Settings` (alte `conf.toml` mit dem Feld laden weiter, serde ignoriert es;
      Test dafür), Formularfeld, `blacklist_info`/`blacklist_invalid` in `src/web/settings.rs`, Template, Texte
      `settings-ip-blacklist*`, `blacklist-loading`, `blacklist-invalid`. Der Fehler „Datei konnte nicht gelesen werden“
      bei leerem Pfad verschwindet damit von selbst.
    - Verbindungen: Filterfeld `blacklisted` (`CONNECTION_FIELDS`, Werte in `connection_values`), Beispiel
      `favorite or blacklisted` → `favorite`, Hilfetext („favorite und blacklisted allein …“), Zeilenklasse
      `blacklisted` und Etikett in `connections.html`, Hinweis `connection-blacklisted` auf der Verbindungsseite, CSS
      `tr.blacklisted`, Texte `tag-blacklisted`, `connection-blacklisted` (de/en).
    - Crate `prefix-trie` aus `Cargo.toml` entfernen, wenn nichts anderes ihn braucht (heute nur die Blacklist);
      `Cargo.lock` nachziehen, `cargo audit`.
    - Doku: CHANGELOG (Eintrag „the blacklist itself and its filter field stay“ ändern: Blacklist entfernt),
      README (falls erwähnt), PLAN.md 2.1 Punkt 9/10 kennzeichnen, Abschnitt 0 (Abweichung vom Funktionsumfang).
    - Prüfen: `cargo test --workspace`, Clippy, alte `conf.toml` mit `ip_blacklist = "…"` lädt, Einstellungen und
      Verbindungen im Headless-Chrome, `grep -ri blacklist src templates locales static` ohne Treffer.
    **Umgesetzt** wie oben beschrieben: `ip_blacklist.rs`, Testdateien, Parameter im Mitschnitt und IPFIX-Sammler,
    `is_blacklisted`, Laden in der Engine, Einstellung, Filterfeld `blacklisted`, Markierung, Texte und Crate
    `prefix-trie` entfernt; Test `settings_of_older_versions_still_load` (alte `conf.toml` mit `ip_blacklist` und alten
    Benachrichtigungen lädt). Beispiel im Verbindungsfilter jetzt `favorite and bytes > 100000`. 283 + 3 + 2 Tests.
15. ✅ **Verbindungsseite: Knöpfe der Gegenstelle „gewürfelt“** (Rückmeldung 2026-10-29, erledigt 2026-10-29):
    In der Karte „Gegenstelle“ stehen „Latenz messen“, „Alle Verbindungen dieser Adresse“ und
    „Alle Verbindungen dieses Hosts“ als verschieden breite Knöpfe in einer `.form-actions`-Zeile; die Karte ist eine
    halbe Seite breit, daher bricht der dritte Knopf allein in die zweite Zeile um. Links in der Karte „Verbindung“
    steht „Pakete dieser Verbindung“ allein, auf anderer Höhe. Außerdem ist „Latenz messen“ eine Aktion, die anderen
    beiden sind Sprünge zu einer Liste.
    Ansatz (Vorschlag, Entscheidung nach der Untersuchung des Nutzers):
    - **Latenz in die Faktenliste:** Zeile „Latenz“ immer zeigen, Wert oder „–“, daneben ein kleiner Knopf „messen“
      (wie der Stern neben der Adresse). Damit bleiben unten nur Sprünge.
    - **Sprünge einheitlich:** „Pakete dieser Verbindung“, „Verbindungen dieser Adresse“ und „Verbindungen dieses
      Hosts“ als eine gemeinsame Leiste unter beiden Karten (eine Zeile, gleiche Höhe nach Punkt 13), oder je Karte
      untereinander mit gleicher Breite (`display: grid`, volle Breite der Spalte). Bevorzugt: gemeinsame Leiste, da
      alle drei Ziele Listen sind und so nichts umbricht.
    - Kürzere Texte („Verbindungen dieser Adresse“ statt „Alle Verbindungen dieser Adresse“).
    - **Neuer Sprung „Statistik dieser Verbindung“** (Wunsch des Nutzers 2026-10-29): bisher nur über „Pakete dieser
      Verbindung“ → Statistik erreichbar. Link `/statistics?f=…` mit demselben Filter wie der Paket-Link
      (`packets::conversation_filter(key)` in `src/web/connections.rs`, `packets`-Feld der Seite), nur wenn der
      Paketspeicher an ist; neuer Text `connection-statistics` (de/en). Damit vier Sprünge in der gemeinsamen Leiste:
      Pakete, Statistik, Verbindungen dieser Adresse, Verbindungen dieses Hosts (die ersten beiden zur Verbindung, die
      anderen zur Gegenstelle, ggf. mit kleinem Abstand gruppiert).
    - Prüfen bei 1000/1400/1900 px Fensterbreite, mit und ohne Host bzw. Latenz-Unterstützung (ICMP-Rechte), hell/dunkel.

    **Umgesetzt (2026-10-29), mit einer Abweichung:** Die Sprünge stehen nicht unter, sondern **über** den Karten als
    Knopfleiste des Seitenkopfs, wie auf den Seiten Pakete und Statistik: „Pakete“, „Statistik“ (neu, `/statistics?f=…`
    mit dem Filter der Verbindung, nur mit Paketspeicher), Trennstrich, „Verbindungen der Adresse“, „Verbindungen des
    Hosts“. Kurze Texte, damit die Leiste auch bei 1000 px Fensterbreite in eine Zeile passt. Die Latenz ist eine Zeile
    der Gegenstelle („Latenz: – [messen]“, kleiner Knopf), nur bei Live-Mitschnitt (mit PCAP-Import im Test nicht
    sichtbar, also noch im echten Betrieb prüfen).

16. ✅ **Bedienung der Sprünge zu Paketen und Statistik** (Rückmeldung 2026-10-30 zu 0.2.5, erledigt 2026-10-30,
    Freigabe „Setze A bis F um“; im Vorschlag gab es nur A bis E): „Bedienung schlechter geworden, seit die Knöpfe geändert wurden; ich kann nicht mehr direkt zu den
    dargestellten Paketen gehen; die Statistik soll dazukommen.“ Geprüft mit einer Testinstanz (Live, 1000/1400 px):
    - **Verbindungsseite:** Die Leiste über den Karten heißt nur „Pakete“, „Statistik“, „Verbindungen der Adresse“,
      „Verbindungen des Hosts“ – „Pakete“/„Statistik“ lesen sich wie die Menüpunkte links, nicht wie „die 77 Pakete
      dieser Verbindung“. Der Bezug zur Zahl „Pakete 77“ in der Karte fehlt (vor Punkt 15 stand der Knopf in der Karte).
      Die vier Knöpfe sehen gleich aus, beziehen sich aber auf zwei Dinge (Verbindung bzw. Gegenstelle).
    - **Paketdetails:** fünf gleichwertige Knöpfe, Blättern (Vorheriges/Nächstes) und Sprünge gemischt; bei 1000 px
      bricht „Zur Verbindung“ allein in die zweite Zeile (dasselbe „gewürfelt“ wie vorher). Statistik fehlt.
    - **Pakete/Statistik einer Verbindung:** zeigen nur den langen Filterausdruck, keinen Hinweis „Verbindung …“ und
      keinen Weg zurück zur Verbindung.
    - **Stream folgen:** der Richtungsumschalter bricht unsauber in zwei Zeilen um (Darstellungsfehler).
    - **Nebenbefund TCP-Analyse:** Keep-Alives einer schon laufenden Verbindung (Client `Seq` = bestätigte Nummer − 1,
      `Len=0`, jede Sekunde) und ihre Antworten werden als „TCP Dup ACK“ rot markiert. Richtig wäre „Keep-Alive“ bzw.
      „Keep-Alive ACK“ (RFC 9293 3.8.4), ohne Fehlerfarbe.
    Vorschlag:
    - A) Verbindungsseite: Leiste oben entfällt, Sprünge in die Zeilen, auf die sie sich beziehen: „Pakete 77 ·
      anzeigen · Statistik“ (Karte Verbindung), „Adresse … ☆ · Verbindungen“ und „Host … · Verbindungen“ (Karte
      Gegenstelle); kleine Textlinks wie „messen“ bei der Latenz.
    - B) Paketdetails: links „‹ Vorheriges | Nächstes ›“ als Knopfgruppe, rechts mit Trennstrich die Sprünge zur
      Verbindung: Pakete, Statistik (neu), Stream, Verbindung; in einer Zeile bis 1000 px.
    - C) Pakete und Statistik mit dem Filter einer Verbindung: Hinweis „Verbindung 1.2.3.4:443 ⇄ … · zur Verbindung“
      über der Liste (Parameter `conn=` neben `f=`).
    - D) Richtungsumschalter von „Stream folgen“ reparieren. E) Keep-Alive-Erkennung in `src/engine/analysis.rs`
      auch ohne gesehenen Verbindungsaufbau (Sequenznummer = letzte Bestätigung der Gegenseite − 1), mit Test.

    **Umgesetzt (2026-10-30), mit diesen Abweichungen vom Vorschlag:**
    - A wie vorgeschlagen (`Fact.links`). Der Host-Link steht in der Zeile „Reverse DNS“, weil es keine Zeile „Host“ gibt.
      Die Zahl „Pakete“ zählt nur die Richtung der Verbindungszeile, „anzeigen“ zeigt beide Richtungen; das sagt der
      Tooltip des Links.
    - B: Die Sprünge stehen hinter dem Wort „Verbindung:“ (Pakete, Statistik, Stream, Details), damit „Pakete“ und
      „Statistik“ nicht wie die Menüpunkte wirken. Bei 1000 px bleibt alles in einer Zeile; am Anfang oder Ende der
      Liste ist „Vorheriges“ bzw. „Nächstes“ ausgegraut, die Leiste springt also nicht.
    - C ohne Parameter `conn=`: Der Server erkennt den Filter selbst (`packets::filtered_connection`, gleicher Text wie
      `conversation_filter`). So erscheint der Hinweis auf jedem Weg (Pakete ↔ Statistik, Browser-Zurück, Paketdetails)
      und verschwindet, sobald der Filter geändert wird.
    - D: Die Knöpfe heißen „Client → Server“/„Server → Client“ (Adressen im Tooltip), sonst passen lange IPv6-Adressen
      nicht; die Werkzeuge rutschen bei Platzmangel unter die Richtungszeilen.
    - E: dazu neuer Befund „TCP Keep-Alive ACK“ (Filterfeld `tcp.analysis.keep_alive_ack`, keine Fehlerfarbe). Die
      Antwort auf einen Keep-Alive war auch bei gesehenem Verbindungsaufbau als Dup ACK markiert; außerdem meldeten
      Daten nach einem Keep-Alive am Mitschnittbeginn eine falsche Lücke (behoben, weil `next` nachgezogen wird).
    Geprüft: 2 neue Tests, Headless-Chrome mit erzeugter PCAP bei 1000/1400 px, hell/dunkel (Latenz-Zeile nur bei
    Live-Mitschnitt, dort nicht erneut geprüft). Test-Paket **0.2.6** gebaut (2026-10-30).

17. ✅ **Zurück-Knopf** (Rückmeldung 2026-10-30 zu 0.2.7, umgesetzt 2026-10-31): „In vielen Situationen fehlt er eindeutig.“ Das
    Fenster-Programm hat keine Browser-Leiste, die Sprünge zwischen Verbindung, Paketen, Statistik und Stream führten nur vorwärts.
    - Pfeil ohne Rahmen im Rand links neben dem Titel aller Seiten (`templates/app.html`, `.page-back`). Der erste Entwurf, ein Knopf mit
      Text, wirkte im Test 0.2.8 zu groß und schob den Titel weg; so bleibt der Titel bündig mit den Karten.
    - Ein Schritt durch die Seiten des Tabs (`static/app.js`): Jeder Verlaufseintrag merkt sich in `history.state` seine Tiefe und
      wie viele Schritte zur vorigen anderen Seite führen. Formulare, die zur selben Seite zurückkehren (Favorit, Einheit, Pause),
      erzeugen sonst Einträge mit gleicher Adresse, und „Zurück“ täte scheinbar nichts. Die Tiefe wandert in `sessionStorage` von Seite zu Seite.
    - Ohne Verlauf (Seite direkt geöffnet): Verbindung → Verbindungen, Paket → Liste, Stream → Paket (`{% block back %}`); die Seiten der
      Navigation haben kein Ziel, der Pfeil fehlt dann.
    - Blättern (nächstes Paket, Text/Hex, Richtung im Stream) legt mit `data-replace` keinen eigenen Schritt an: Zurück führt aus den
      Paketdetails direkt zur Liste. Beim Öffnen eines Pakets schreibt `packets.js` `goto=` in die Adresse der Liste, Zurück landet also
      beim Paket statt am Ende der Liste; die Liste folgt dann nicht mehr den neuen Paketen (Knopf Live).
    - Alt+Links macht dasselbe wie der Knopf. Die Fehlerseite behält ihren eigenen `data-back` (ein Schritt, damit das Formular erhalten bleibt).
    - Geprüft in Headless-Chrome (Testinstanz, erzeugte PCAP, 152 Pakete) mit Klick, Alt+Links, Favoriten-Formular, Blättern und frischen
      Tabs. **Im Fenster-Programm (WebKitGTK) noch nicht geprüft** (Test-Paket 0.2.8).

18. ✅ **Statistik-Knopf auf der Seite Verbindungen** (Rückmeldung 2026-10-30 zu 0.2.7, umgesetzt 2026-10-31): Der Knopf „Statistik“
    übernimmt den Inhalt der Suchleiste, auch wenn er noch nicht angewendet wurde, und die Statistik zeigt die Pakete aller passenden
    Verbindungen. Die Filter sind verschieden: `country`, `program`, `service`, `direction`, `favorite`, `bytes`, `packets` kennt nur der
    Verbindungsfilter, daher ein eigener Parameter.
    - `/statistics`, `/packets`, `/api/packets` und `/packets.pcap` verstehen `c=` (Verbindungsfilter) neben `f=` (Anzeigefilter), beide
      lassen sich kombinieren (`packets::Selection`). Jedes `StoredPacket` kennt seine Verbindung; `EngineState::selected_connections`
      nennt die passenden. Wie in der Verbindungsliste zählt jede Richtung als eigene Verbindung: `direction == outgoing` nimmt nur diese Pakete.
    - Abweichung vom Plan: Das Filterfeld wechselt nicht seine Bedeutung. Ist `c` gesetzt, steht links daneben ein zweites Feld mit dem
      Verbindungsfilter (`data-filter-kind="connections"`, `templates/connections_field.html`), sonst ginge die Kombination aus Statistik-Links
      (`c=…&f=tcp`) nicht. Darüber ein Hinweis mit Link zurück zu den Verbindungen (`connections_note.html`), Zurücksetzen entfernt beide.
    - `FilterIndex::with_connections`: eigener Eintrag je Filtertext, merkt sich die Menge der Verbindungen. Ändert sie sich (neue Verbindung,
      `bytes` wächst), wird neu ausgewertet, ohne Pakete zu zerlegen; sonst kommen nur neue Pakete dazu. Einträge von 4 auf 6 erhöht.
    - Ohne Paketspeicher oder bei IPFIX führt der Knopf zur Statistik mit deren Hinweis. Es zählen nur Pakete, die noch im Speicher sind;
      die Summen können kleiner sein als in der Verbindungsliste, das sagt der Hinweis.

19. ✅ **Vorschläge in den Filterfeldern** (Wunsch des Nutzers, 2026-10-31, noch vor 0.3.0: „im Moment muss man die Befehle eher raten“).
    Alle Filterfelder (Verbindungen, Pakete, Statistik) zeigen beim Tippen eine Liste (`static/suggest.js`):
    - Der Text vor dem Cursor sagt, was als Nächstes kommen kann (kleiner Zustandsautomat, dieselben Wörter wie `tokenize`): Feld oder
      Protokoll, passende Operatoren (`filter::operators`, ein Test prüft jeden gegen den Parser), Wert, `and`/`or`, `{`, `}`, `)`.
    - `GET /api/filter/names` (`src/web/suggest.rs`) liefert Namen, Wertart und Operatoren; Erklärungen für die wichtigsten rund 65 Namen
      in Deutsch und Englisch (`filter-hint-*`, ein Test prüft, dass jede zu einem Namen gehört). Bei Werten steht ein Beispiel (`filter-value-*`).
    - Im Verbindungsfilter echte Werte: `GET /api/filter/values?field=…&q=…` (`EngineState::connection_field_values`), die häufigsten
      zuerst, nicht für `packets`, `bytes`, `favorite`. Sie kommen aus dem Netzverkehr und werden nur als Text gesetzt; `filter::literal`
      setzt Werte mit Leerzeichen in Anführungszeichen. Im Paketfilter bewusst keine Werte, dafür müsste man alle Pakete auswerten.
    - Bedienung: Pfeile wählen, Tab oder Enter übernimmt die gewählte Zeile, Tab ohne Auswahl die erste, sobald etwas getippt ist.
      **Enter ohne Auswahl wendet weiter den Filter an.** Esc schließt, Maus geht auch. Keine neue Abhängigkeit.
    - Geprüft mit echten Tastendrücken in Headless-Chrome (`ip.`-Felder, `in { }`, Klammern, zwei Felder auf einer Seite, 800 px Breite).
      **Im Fenster-Programm noch nicht geprüft.**

#### Fenster-Programm `openpacketmon-app` (Variante B, erledigt am 2026-10-28)

Wunsch des Nutzers: eigenständige App statt Browser, mit Symbol im Infobereich. Umsetzung:
- Eigenes Programm im Workspace (`app/`), **ohne Mitschnittrechte**: `/usr/bin/openpacketmon` hat `cap_net_raw`/`cap_net_admin`,
  die Browser-Engine (WebKitGTK) soll nicht mit diesen Rechten laufen. Crates `wry` 0.57 (MIT/Apache), `tao` 0.37 (Apache-2.0),
  `tray-icon` 0.26 (MIT/Apache, AppIndicator per dlopen), `png`.
- Startet den Server (`openpacketmon --no-browser --port …`, eigene Prozessgruppe), wenn keiner antwortet, oder nutzt den
  systemd-Dienst; meldet sich mit dem Startlink an (Datei im Laufzeitverzeichnis, nur für den Benutzer lesbar).
- Fenster schließen versteckt es (Aufzeichnung läuft weiter); Menü im Infobereich: öffnen, im Browser öffnen (die
  Mini-Ansicht ist seit 2026-10-29 entfernt), beenden (`POST /app/quit`, beendet auch den Server). Antwortet der Server nicht
  mehr (beendet über die Einstellungen), endet das Programm. Zweiter Start holt das Fenster nach vorn (Unix-Socket).
- Server: `GET /app/texts` (Menütexte in der Sprache der UI) und `POST /app/quit`, nur mit dem Zugangsschlüssel als
  `Authorization: Bearer` (nicht mit einer Browser-Sitzung). Fremde Links öffnen im Browser, Downloads landen im
  Download-Ordner mit Hinweis im Fenster. Einstellung „Beim Anmelden im Infobereich starten“
  (`~/.config/autostart/openpacketmon-app.desktop`, `--hidden`).
- Paket: Startmenü startet `openpacketmon-app`; Abhängigkeiten `libwebkit2gtk-4.1-0`, `libgtk-3-0`, Empfehlung
  `libayatana-appindicator3-1`. Bauen: `cargo build --release --workspace && cargo deb`.
- Vom Nutzer im Fenster geprüft (Wayland, Zorin): Anmeldung, Sprache, Darstellung wie im Browser.
- ✅ **Nutzertest am 2026-10-29 erfolgreich** (installiertes `.deb`): Fenster schließen und über das Symbol zurückholen,
  Mini-Ansicht, PCAP-Download mit Hinweis, fremde Links im Browser, „openpacketmon beenden“ im Menü. Nächster Schritt: Stufe 2
  nach Freigabe.

### Phase 4 – Vor dem Öffentlich-Schalten (Plan 2026-10-29, Umsetzung je Schritt erst nach Freigabe)

Ziel: Das Repo darf öffentlich werden, ohne fremde Daten mit unpassender Lizenz zu verbreiten, mit allen Pflichten aus
`MIT OR Apache-2.0` und den Lizenzen der Abhängigkeiten, und mit einer Historie, die nur den bereinigten Stand enthält.
Bestandsaufnahme vom 2026-10-29 (Stand `f4ca4ee`): Name/Logo/Attribution, Entfernen der Sniffnet-Dienste und -Links,
Sounds (Töne per WebAudio, keine Dateien) und `cargo audit` in der CI sind erledigt; keine Heimnetz-Adressen und keine
`/home/…`-Pfade in der Historie.

#### 4.0 Entscheidungen

| Thema | Entscheidung |
|---|---|
| Git-Historie | **Neu schreiben** (Nutzer, 2026-10-29): Vor dem Umschalten ein **neuer Anfangs-Commit** mit dem bereinigten Stand; die alte Historie (mit nmap-Datei, GeoLite-DBs, Sounds, Thesis, Audits des Originals im ersten Commit `upstream-base`) wird nicht veröffentlicht. Umsetzung 4.9. |
| E-Mail in den Commits | **Bleibt** `m.tomcs@gmail.com`, wie bei opennetmon (Nutzer, 2026-10-29). |
| Marke „openpacketmon“ | **Vom Nutzer geprüft, keine Konflikte** (2026-10-29). |
| GeoIP-Quelle | **Variante A: DB-IP Lite** per Download in den Einstellungen (Nutzer, 2026-10-29), siehe 4.2. |
| `CLAUDE.md`/`PLAN.md` öffentlich | **Ja, wie bei opennetmon** (Nutzer, 2026-10-29); vorher Durchsicht in 4.7. |

#### 4.1 ✅ Dienstliste aus der IANA-Registry statt nmap – erledigt 2026-10-29

**Umgesetzt:** Lizenz der IANA-Registry: gemeinfrei (CC0 1.0, gemeinsame Erklärung IANA/IETF vom 2021-11-10,
https://www.iana.org/help/licensing-terms). Neues Skript **`dev/update-services.py`** (ersetzt `services.sh`): lädt
die CSV (oder nimmt eine Datei), erster Name je Port/Protokoll, Bereiche aufgelöst, Vorrang für `submissions` (465),
`l2tp` (1701), `nfs` (2049); schreibt `services.txt` mit Kopfzeilen (Quelle, Abrufdatum, CC0, nicht von Hand ändern) und
gleicht `categories.txt` ab (ungenutzte Namen raus, neue als `other`, Kopfzeile „Modified for openpacketmon“).
Ergebnis: **11 526 Ports, 6167 Namen** (vorher 12 093 Zeilen aus nmap); 602 Kategorien entfernt, 303 neu, davon 170
einmalig mit der Kategorie des alten nmap-Namens am selben Port (z. B. `bootps`/`bootpc` network, `ssdp`/`mdns`
discovery, `ldaps` identity, `http-alt` web, `submissions` email, `nfs` files), `eldim` (31337, früher die Hintertür
„Elite“) von Hand auf files, 133 seltene bleiben `other`. Zweiter Lauf ändert nichts. `build.rs`: Plausibilitätsgrenzen
(> 10 000 Ports, > 5000 Kategorien) statt fester Zahl, zwei Safe-Words ergänzt. Tests auf IANA-Namen umgestellt
(`manage_packets.rs`, `service.rs`). `NOTICE`: `categories.txt` als geändert, `services.txt` als ersetzt gelistet;
`dev/check-notices.sh` ohne Befund. Ende-zu-Ende mit PCAP: Seite Dienste zeigt `https`, `domain`, `ntp`, `ssdp`,
`mdns`, `bootps`, `submissions`, `openvpn`, `imaps`, `http-alt` mit passenden Kategorien. `grep -ri nmap` nur noch in
Doku, Skript-Kommentar und den IANA-Diensten `nmap`/`rnmap`. Code außerhalb der Tests nannte keine geänderten Namen.
283 + 3 + 2 Tests grün. Ursprünglicher Plan:

Vorher: `resources/services/services.txt` (12 093 Zeilen `name<TAB>port/proto`) stammt aus `nmap-services` (NPSL,
Copyleft) und wird von `build.rs` per `phf` fest eingebaut; `services.sh` lädt sie von nmap. `categories.txt`
(6466 Namen → Kategorie) kommt aus dem Original (MIT/Apache) und bleibt.
- Neues Skript `dev/update-services.py` (ersetzt `resources/services/services.sh`): lädt die **IANA Service Name and
  Transport Protocol Port Number Registry** als CSV
  (`https://www.iana.org/assignments/service-names-port-numbers/service-names-port-numbers.csv`), nimmt nur `tcp`/`udp`
  mit Port und Dienstnamen, zerlegt Bereiche (`6000-6063`) in einzelne Ports, schreibt dasselbe Format mit Kopfzeile
  (Quelle, Abrufdatum, „generated, do not edit“). **Mehrere Namen je Port/Protokoll** (bei IANA häufig): ersten
  Eintrag der Registry nehmen, ausgenommen eine kleine Vorrangliste im Skript für bekannte Fälle (z. B. 1900/udp
  `ssdp`, 5353/udp `mdns`, 443 `https`). Lizenz/Nutzungsbedingungen der IANA-Daten vor dem Einbau nachlesen und im
  Kopf der Datei und in `THIRD-PARTY-LICENSES.txt` nennen.
- `build.rs`: festen Zähler `assert_eq!(num_entries, 12093)` durch eine Plausibilitätsgrenze ersetzen (z. B. > 5000)
  oder auf die neue Zahl setzen; doppelte Schlüssel weiter als Fehler.
- Namensunterschiede nmap → IANA prüfen und `categories.txt` ergänzen (Skript meldet IANA-Namen ohne Kategorie);
  bekannte Fälle: `upnp` → `ssdp` (1900), `dhcps`/`dhcpc` → `bootps`/`bootpc`, `zeroconf`/`mdns`, `http-proxy`,
  `imap3`, `ldapssl`. Code und Tests, die Dienstnamen nennen, anpassen (`src/networking/types/service.rs` Tests,
  `get_service`, Farbregeln und Anzeigefilter der Pakete, Beispiele/Hilfetexte in `locales/`, Dienste-Favoriten gibt es
  nicht mehr).
- Folge (bekannt, akzeptiert): nmap-Namen für Trojaner/Würmer und inoffizielle Ports fehlen; solche Verbindungen zeigen
  den Dienst „?“.
- Prüfen: `cargo test --workspace`, Ende-zu-Ende-Test mit PCAP, Seite Dienste mit Live-Verkehr (https, domain, ntp,
  ssdp, mdns erkannt), `grep -ri nmap` ohne Treffer außer CHANGELOG/PLAN.

#### 4.2 ✅ GeoIP-Datenbanken nicht mehr einbauen – erledigt 2026-10-29 (Variante A + B)

**Umgesetzt:** `resources/DB/` (GeoLite2, 21 MB) und `include_bytes!` entfernt. `MmdbReader` kennt nur noch
`File(Pfad, Reader)` und `Empty`; `MmdbReaders` hält je Datenbank einen `SharedReader` (`Arc<RwLock<Arc<…>>>`), damit
neue Dateien ohne Neustart gelten (für Adressen, die danach erkannt werden; bereits aufgelöste behalten ihr leeres Land).
Neues Modul **`src/mmdb/geo.rs`**: Reihenfolge eigener Pfad aus den Einstellungen → `~/.local/share/openpacketmon/geo/
dbip-{country,asn}-lite.mmdb` (`$XDG_DATA_HOME`) → `/var/lib/GeoIP/GeoLite2-{Country,ASN}.mmdb` (geoipupdate). Download
nur auf Klick (`POST /settings/geo`, CSRF): `https://download.db-ip.com/free/dbip-{country,asn}-lite-JJJJ-MM.mmdb.gz`,
bei 404 der Vormonat; Grenzen 64 MB gepackt/256 MB entpackt, Prüfung (MMDB lesbar, 8.8.8.8 hat Land bzw. AS), erst
`.part`, dann umbenennen – eine kaputte Datei ersetzt nie eine gute; nur ein Download gleichzeitig. Feldnamen geprüft:
DB-IP-Dateien sind GeoLite2-kompatibel (`DBIP-Country-Lite`, `DBIP-ASN-Lite (compat=GeoLite2-ASN)`), kein Umbau der
Einträge nötig. Einstellungen „Geodaten“: Datei, Typ, Stand und Alter je Datenbank, Warnung ab 60 Tagen, Knopf
„Geodatenbanken von DB-IP herunterladen“ (zeigt „Wird heruntergeladen …“), Hinweis auf db-ip.com/CC BY 4.0, eigene
Pfade weiter möglich. Hinweis auf Übersicht und Hosts ohne Länderdatenbank; Seitenleiste verlinkt „IP Geolocation by
DB-IP“, solange DB-IP-Daten genutzt werden (Namensnennung nach CC BY). Neue Abhängigkeit `flate2` (stand schon im
Lockfile). Tests: mmdb-Tests auf `resources/test/ipinfo_*` und JSON-Einträge im GeoLite2/DB-IP-Format umgestellt,
neue Tests in `geo.rs` (Reihenfolge, kaputte Downloads werden nicht installiert). `NOTICE`: `src/mmdb/{asn,country,mod}.rs`,
`src/mmdb/types/mmdb_reader.rs`. Im Browser geprüft (Testinstanz mit eigenem `XDG_DATA_HOME`): Hinweis ohne
Datenbank, Download echt von db-ip.com (Stand 2026-10-01), danach Länder/AS in den Hosts, Fehlermeldungen, CSRF.
Offen für 4.5: DB-IP in `THIRD-PARTY-LICENSES.txt` nennen. Automatischer monatlicher Download nicht umgesetzt
(nur auf Klick). Ursprünglicher Plan:

Vorher: `src/mmdb/asn.rs`/`country.rs` binden `resources/DB/GeoLite2-ASN.mmdb` und `GeoLite2-Country.mmdb` per
`include_bytes!` ein (MaxMind GeoLite-EULA + CC BY-SA 4.0: Namensnennung, Weitergabe der Bedingungen, Aktualisierung
binnen 30 Tagen – mit einem eingebauten Stand nicht einzuhalten). Eigene Pfade sind in den Einstellungen schon möglich.
- Gemeinsam für alle Varianten: `include_bytes!` und `resources/DB/` entfernen; `MmdbReader::from` ohne eingebaute
  Daten (kein Pfad oder Datei fehlt → keine Geolokalisierung, Land/AS leer, Filterfelder `country`/`as` treffen nichts);
  Hinweis auf Übersicht/Hosts „Keine Geodatenbank – in den Einstellungen einrichten“; Tests der mmdb-Module auf
  Testdateien umstellen (4.3); Quellenhinweis der gewählten Daten in Einstellungen, README und
  `THIRD-PARTY-LICENSES.txt`.
- **Variante A (empfohlen): DB-IP „IP to Country Lite“ und „IP to ASN Lite“** (CC BY 4.0, nur Namensnennung, kein
  Konto, monatlich als mmdb): Knopf „Geodatenbanken herunterladen“ in den Einstellungen lädt beide nach
  `~/.local/share/openpacketmon/geo/` (nur auf Klick, Hinweis, dass dabei db-ip.com abgefragt wird), Prüfung des
  Formats, Anzeige von Stand und Alter, Hinweis ab 60 Tagen. Optional: monatlich automatisch, nur wenn eingeschaltet.
  Vorab prüfen: Feldnamen der DB-IP-Dateien passen zu `MmdbCountryEntry`/`MmdbAsnEntry` (sonst Anpassung).
- Variante B: MaxMind GeoLite2 mit eigenem kostenlosem Konto per `geoipupdate` (Debian-Paket `geoipupdate`,
  Standardpfad `/var/lib/GeoIP/`); openpacketmon findet die Dateien dort automatisch. Mehr Aufwand für Nutzer,
  Bedingungen der EULA beim Nutzer.
- Variante C: IPinfo Lite (CC BY-SA 4.0, Konto und Token nötig, Format wird von `src/mmdb` schon gelesen).
- A und B lassen sich kombinieren (automatische Erkennung von `/var/lib/GeoIP/GeoLite2-*.mmdb` zusätzlich).

#### 4.3 ✅ Testdaten prüfen – erledigt 2026-10-29

**Ergebnis:** Einzige fremde Testdaten im Repo sind `resources/test/ipinfo_{asn,lite,location}_sample.mmdb`. Herkunft
belegt: Metadaten der Dateien nennen `ipinfo … sample.mmdb`; Quelle ist das IPinfo-Repo `ipinfo/sample-database`
(Pfade `ASN Database/…`, `IPinfo Lite/…`, `IP Geolocation/…`), von Sniffnet im Commit `2a0103c05c` (2025-08-13)
übernommen. Das IPinfo-Repo steht seit 2024-02-02 (`LICENSE.md`) unter **CC BY 4.0** – Weitergabe erlaubt mit
Namensnennung. Die exakten Stände sind dort nicht mehr aktuell (IPinfo ersetzt die Beispiele wöchentlich), daher
behalten statt neu laden. **Behalten**, neue Datei `resources/test/README.md` mit Quelle, Lizenz und „unverändert“;
in 4.5 in `THIRD-PARTY-LICENSES.txt` aufnehmen. Nicht im `.deb` (keine Assets). Alle übrigen Testdaten sind eigen:
PCAP-Dateien der Ende-zu-Ende-Tests erzeugt der Testcode (`src/engine/mod.rs`, `write_pcap`), Pakete der Zerleger- und
Filtertests sind von Hand geschrieben; keine Mitschnitte aus fremden Quellen (Suche nach Wireshark-Beispielen ohne
Treffer, nur ein Doku-Link in der unveränderten Datei `lib/sniffnet-packet-parser/src/headers.rs`). Ursprünglicher Plan:

`resources/test/ipinfo_{asn,lite,location}_sample.mmdb` (aus dem Original) – Herkunft und Lizenz klären (IPinfo-Beispieldaten,
vermutlich CC BY-SA 4.0). Wenn die Lizenz die Weitergabe erlaubt: behalten und in `THIRD-PARTY-LICENSES.txt` nennen;
sonst durch selbst erzeugte Mini-Datenbanken ersetzen (Skript `dev/make-test-mmdb.py` mit einem mmdb-Writer, nur
Dokumentationsadressen wie 192.0.2.0/24, 2001:db8::/32). Ebenso `resources/test/*.pcap*` und weitere Testdateien auf
eigene Herkunft prüfen.

#### 4.4 ✅ Geänderte Dateien des Originals kennzeichnen (Apache-2.0 §4b) – erledigt 2026-10-29

**Umgesetzt:** 47 Dateien tragen oben zwei Kommentarzeilen („Modified for openpacketmon (2026), see NOTICE. Based on
`<originalpfad>` of Sniffnet (…), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.“): die 41 geänderten und 7
umbenannten Dateien, soweit sie Kommentare erlauben (Rust, TOML, `.gitignore`, `.gitattributes`, `postinst` nach der
Shebang-Zeile, `.desktop`), dazu die neuen Dateien mit Code des Originals – per Zeilenvergleich gefunden:
`src/engine/state.rs` (aus `src/gui/sniffer.rs`), `src/config/favorite.rs`, `src/config/settings.rs`
(aus `src/gui/types/…`), `src/engine/traffic_history.rs` („contains code adapted from“
`src/chart/types/traffic_chart.rs`, Lückenfüllung bei Offline-Mitschnitten). Nur in `NOTICE` gelistet (ohne
Kommentarmöglichkeit): `CHANGELOG.md`, `Cargo.lock`, `LICENSE-MIT`, `README.md`, `resources/packaging/LICENSE`,
`resources/packaging/linux/description.txt`. Neue Datei **`NOTICE`** (Englisch: Fork-Hinweis, Ausgangscommit, Liste);
README verweist darauf; Regel in `CLAUDE.md`. Prüfskript **`dev/check-notices.sh`** (klont das Original bei
`ab4bf739d464` oder nimmt ein Verzeichnis; Gegenprobe mit absichtlich geänderter Datei schlägt an). fmt, Clippy,
283 + 3 + 2 Tests grün. Ursprünglicher Plan:


Stand: 41 Dateien des Originals geändert, 7 umbenannt und geändert, 502 entfernt, 114 neu (`git diff --name-status
upstream-base HEAD`). **Muss vor 4.9 passieren**, weil danach der Tag `upstream-base` fehlt.
- Liste der geänderten/umbenannten Upstream-Dateien erzeugen und in **`NOTICE`** (neu, Englisch) ablegen: „This
  product is a fork of Sniffnet (https://github.com/GyulyVGC/sniffnet, commit ab4bf739d464), Copyright (c) 2022
  Giuliano Bellini …; the following files were modified …“, mit Verweis auf den CHANGELOG.
- In jeder geänderten Rust-Datei eine Kopfzeile: `// Modified for openpacketmon, 2026. Originally part of Sniffnet
  (MIT OR Apache-2.0).` In unveränderten Dateien nichts.
- Regel in `CLAUDE.md`: wer später eine bisher unveränderte Upstream-Datei ändert, setzt die Kopfzeile und ergänzt
  `NOTICE`. Optional ein Prüfskript `dev/check-notices.sh` gegen einen temporären Upstream-Klon.
- Copyright-Hinweise des Originals in `LICENSE-MIT`, README und Dateien bleiben unverändert.

#### 4.5 ✅ Fremdlizenzen – erledigt 2026-10-29

**Umgesetzt:** `dev/third-party-licenses.py` aus opennetmon übernommen und angepasst: Ziel `x86_64-unknown-linux-gnu`,
Wurzeln `openpacketmon` und `openpacketmon-app` (nur normale Abhängigkeiten, Workspace-Mitglieder selbst ausgenommen),
uPlot 1.6.32 als gebündelte Datei, Hinweise zu IANA (CC0), DB-IP (CC BY 4.0, nur per Download), IPinfo-Testdaten
(CC BY 4.0, nur im Repo) und den Systembibliotheken (libpcap, GTK, WebKitGTK, ayatana-appindicator, nicht
mitgeliefert); `--check` für Prüfungen. Ergebnis **`THIRD-PARTY-LICENSES.txt`** (313 Crates, 13 107 Zeilen, 670 KB):
alle Lizenzen verträglich (überwiegend MIT/Apache-2.0, dazu Unicode-3.0, ISC, Zlib, BSD, `option-ext` MPL-2.0
unverändert genutzt, `self_cell` „Apache-2.0 OR GPL-2.0-only“ → Apache); sechs Crates ohne Lizenzdatei im Quellpaket
sind mit ihrer SPDX-Angabe gelistet. `.deb`: `NOTICE` und `THIRD-PARTY-LICENSES.txt` nach `/usr/share/doc/openpacketmon/`.
Seite **`/licenses`** (mit Anmeldung, `text/plain`) zeigt `NOTICE` und die Datei, beide per `include_str!` eingebaut;
Link „Lizenzen“ in der Fußzeile der Seitenleiste. `dev/build-deb.sh` erzeugt die Datei vor dem Bau neu und hält an,
wenn sie sich geändert hat oder nicht committet ist. Test `the_licenses_are_shown`, 289 + 3 + 2 Tests grün.
Offen für 4.6: `--check` in die CI. Ursprünglicher Plan:

- `dev/third-party-licenses.py` aus opennetmon übernehmen (eigene Datei des Nutzers, für den Fork freigegeben) und
  anpassen: alle Crates des Release-Binarys beider Programme (`openpacketmon`, `openpacketmon-app`), dazu uPlot
  (`static/vendor/uplot/LICENSE`), Dienstliste (IANA), Geodaten-Quelle (4.2), Testdaten (4.3).
- Ergebnis `THIRD-PARTY-LICENSES.txt` im Repo; im `.deb` nach `/usr/share/doc/openpacketmon/` (Asset in
  `Cargo.toml`); Link in der Fußzeile der Oberfläche („Lizenzen“) auf eine Seite `/licenses`, die die Datei zeigt.
- `dev/build-deb.sh` erzeugt die Datei vor dem Bauen neu und hält an, wenn sie sich geändert hat und nicht committet ist.

#### 4.6 ✅ `cargo deny` und CI – erledigt 2026-10-29

**Umgesetzt:** `deny.toml` (cargo-deny 0.20.2, lokal installiert): Ziel `x86_64-unknown-linux-gnu`; Lizenzen erlaubt
0BSD, Apache-2.0 (auch WITH LLVM-exception), BSD-2/3-Clause, BSL-1.0, CC0-1.0, ISC, MIT, MIT-0, MPL-2.0 (nur
`option-ext`, unverändert), Unicode-3.0, Unicode-DFS-2016 (`finl_unicode`, nur Build über `rustrict`), Unlicense,
Zlib; Workspace-Crates als privat ignoriert. `sources`: nur crates.io, keine Git-Quellen. `bans`: doppelte Versionen
nur Warnung (18, aus Abhängigkeiten: GTK 3, TLS, rand …), Wildcards verboten. `advisories`: `unmaintained`/`unsound`
für den ganzen Baum (wie `cargo audit`); dieselben drei bekannten Meldungen mit Begründung ausgenommen
(RUSTSEC-2026-0374 `basic-toml` über askama, RUSTSEC-2024-0370 `proc-macro-error` und RUSTSEC-2024-0429 `glib` über
GTK 3 des Fenster-Programms). Wichtig: **`cargo deny --workspace check`** – ohne `--workspace` prüft cargo-deny nur
`openpacketmon`, nicht `openpacketmon-app`. Gegenprobe: ohne Ausnahme für `glib` → „advisories FAILED“, ohne MIT →
„licenses FAILED“. CI: Job `audit` durch Job `deny` ersetzt (cargo-deny per `cargo install` mit Cache, wöchentlich
und bei jedem Push, Actions weiter auf Commit-Hash, `contents: read`); Job `check` prüft zusätzlich
`dev/third-party-licenses.py --check`. Die Actions des Repos sind abgeschaltet (Phase 1) – im Betrieb erst nach dem
Umschalten (4.9/4.10). Ursprünglicher Plan:

- `deny.toml`: erlaubte Lizenzen (MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2/3-Clause, ISC, Zlib,
  Unicode-3.0, BSL-1.0, CC0-1.0, CDLA-Permissive-2.0, MPL-2.0 nur unverändert genutzt), `bans` (keine doppelten großen
  Crates als Warnung), `sources` (nur crates.io), `advisories` (ersetzt oder ergänzt `cargo audit`).
- CI-Job `deny` (`cargo deny check`), Actions wie bei opennetmon auf Commit-Hash gepinnt, Rechte `contents: read`.
- Prüfen: `cargo deny check` lokal und in der CI grün.

#### 4.7 ✅ Repo-Dateien und Durchsicht – erledigt 2026-10-29

**Umgesetzt:** Nach Vorlage opennetmon (Englisch): `SECURITY.md` (Meldeweg „Report a vulnerability“, Umfang:
Weboberfläche und Anmeldung, Zerleger/Filter/TCP-Analyse, Import von PCAP/MMDB, Geodaten-Download, Fenster-Programm,
Paket; bekannte Grenzen: nur `127.0.0.1` per HTTP, `setcap` erlaubt jedem lokalen Benutzer den Mitschnitt, Pakete im
Speicher; Meldungen zu Code des Originals gehen an Sniffnet weiter), `CONTRIBUTING.md` (Prüfbefehle inkl. `cargo deny`,
Fluent-Texte DE/EN, keine Inline-Skripte, **kein Wireshark-Code**, Kennzeichnung geänderter Sniffnet-Dateien,
Abhängigkeiten/Lizenzliste, keine Fremd-Assets, KI-Hilfe offenlegen, DCO), `.github/pull_request_template.md`,
`.github/ISSUE_TEMPLATE/` (Fehlerbericht, Wunsch, Sicherheitsmeldungen privat) und CI-Job `dco` (nur bei Pull
Requests). README: Abschnitte „Contributing“ (mit Hinweis auf KI-Hilfe) und „License and data sources“, Verweis auf
`SECURITY.md`, `build-deb.sh` prüft die Lizenzliste; Status „work in progress, no releases yet“ stimmt, kein Hinweis
auf „privat“. Suchläufe über alle getrackten Dateien: keine Heimnetz-Adressen, Zugangsdaten oder Schlüssel; Treffer
nur als Beispiele/Platzhalter (`/home/user/…`, `192.168.1.x`), Copyright-Namen, Plan-Texte; `nmap`/`GeoLite` nur in
Doku, Skript-Kommentar, Einstellungs-Hinweisen und den IANA-Diensten `nmap`/`rnmap`; `sniffnet.app` nur im Plan
(entfernte Dateien). `PLAN.md`: konkrete Heimgeräte in 5.1 verallgemeinert; E-Mail-Adresse bleibt wie entschieden.
Größte Datei 810 KB (Icon 1024²), nichts Unerwartetes. `lib/sniffnet-packet-parser`: Name bleibt, `publish = false`,
gegenüber dem Original nur `Cargo.toml` geändert. Ursprünglicher Plan:

- Neu (Englisch, nach Vorlage opennetmon): `SECURITY.md` (Meldeweg: GitHub „Private vulnerability reporting“),
  `CONTRIBUTING.md` (kleine PRs, KI-Hilfe offenlegen, Wireshark-Code/-Zerleger/-Grammatik nie übernehmen),
  `.github/pull_request_template.md`.
- README durchsehen: Englisch, „Based on Sniffnet“, Abschnitt „Data sources and licenses“ (IANA, Geodaten, uPlot,
  Link auf `THIRD-PARTY-LICENSES.txt` und `NOTICE`), Installation/Bau, keine Hinweise auf „privat“.
- `CLAUDE.md` und `PLAN.md` durchsehen (öffentlich wie bei opennetmon, sofern der Nutzer zustimmt): keine Heimnetz-
  Details (gehören in `CLAUDE.local.md`), keine Zugangsdaten, keine Pfade zu privaten Dateien außer den Logo-Originalen
  (`~/Bilder/…`, unkritisch) – sonst kürzen.
- Suchläufe über alle getrackten Dateien: `192.168.178`, `/home/`, `token`, `password`, `secret`, `BEGIN .* KEY`,
  `nmap`, `GeoLite`, `sniffnet.app`; Dateigrößen (`git ls-files | xargs du` – nichts Unerwartetes > 1 MB).
- `lib/sniffnet-packet-parser` behält seinen Namen (Code des Originals, unverändert, `publish = false` prüfen).

#### 4.8 ✅ Sicherheitstest – erledigt 2026-10-29

**Durchgeführt:** Code-Review (Anmeldung, Router, Header, alle Handler mit Parametern, Filter-Parser, Zerleger,
IPFIX, Fenster-Programm), neue Zufallstests und 190 Angriffe auf eine Testinstanz (Release-Build, Port 7390,
eigene `XDG_*`-Ordner, Import einer präparierten PCAP mit 6009 Paketen: XSS-Namen in DNS/TLS-SNI/ALPN/HTTP/DHCP/SSDP,
DNS-Zeigerschleife, kaputte IPv6-Erweiterung, 3-MiB-TCP-Strom, 3000 Verbindungen).

**In Ordnung:** Startlink-Datei `0600` in `0700`-Ordner, Server nur auf `127.0.0.1`; alle 19 GET- und 11 POST-Routen
ohne/mit gefälschter Sitzung abgelehnt; CSRF fehlt/falsch/von anderer Sitzung, `Origin` fremd oder `null` → 403;
11 fremde `Host`-Werte (DNS-Rebinding, `127.1`, `0.0.0.0`, `[::ffff:127.0.0.1]`, ohne Port …) → 421; Cookie
`HttpOnly; SameSite=Strict`, Abmelden macht die Sitzung ungültig, höchstens 16 Sitzungen; 3000 parallele falsche
Anmeldungen ohne Folgen (Token 256 Bit; der Token bleibt bis zum Neustart gültig, gewollt für Fenster-Programm und
Zweitstart); `/app/*` nur mit Bearer-Token; Sicherheits-Header und CSP ohne `unsafe-*` auf Seiten, Fehlern, `/static`
und JSON; keine Pfade außerhalb von `/static`; feindliche Namen auf allen Seiten, Paketdetails und Strömen
maskiert, `|safe` nur für Zahlen/Zeiten, Skripte setzen Daten per `textContent`; JSON als `application/json`.
Grenzen halten: `/api/packets` mit `count`/`from`/`goto`/`anchor` = `u64::MAX` oder negativ, Filter > 4096 Zeichen
oder tiefer als 64 abgelehnt, Kette aus 1024 `||` in 0,1 s, 1-MB-Parameter → 431, Body > 2 MB → 413, 200 parallele
verschiedene Filter 6,3 s, Stream folgen bei 1 MiB gekappt (0,07 s), PCAP-Download mit Filter 0,1 s, Seitenzahlen
ungültig → letzte Seite/400, Einstellungen mit Riesenzahlen → 422, Latenz nur für Adressen der Aufzeichnung.
Speicher der Testinstanz dabei 50 → 90 MB. Rechte: nur `cap_net_admin,cap_net_raw` auf `/usr/bin/openpacketmon`
(jeder lokale Benutzer darf mitschneiden, in `SECURITY.md` genannt), `openpacketmon-app` ohne. `cargo deny` ohne Befund.

**Gefunden und behoben:**
1. **Absturz im DNS-Zerleger** (DNS über TCP mit 1 Byte Nutzlast und Bytes hinter dem IP-Paket: Länge aus dem
   Trailer gelesen, `data[56..55]` → Panic). Von außen mit einem Paket auslösbar; die Paketliste, Filter und
   Details mit diesem Paket wären danach jedes Mal abgebrochen. Länge jetzt nur aus der Nutzlast (`dns.rs`).
2. **IPFIX-Collector** (lauscht standardmäßig auf `0.0.0.0:4739`): präparierte Vorlagen kosteten viel CPU und Speicher.
   Jetzt mit Grenzen für Vorlagen (`collect.rs`); Test `test_process_datagram_hostile_templates`. Einzelheiten und
   Messwerte nur lokal (`CLAUDE.local.md`), bis das Original den Befund behoben hat.
3. **Halbe HTTP-Anfragen** blieben unbegrenzt offen (900 Stück: +67 MB, nur lokal erreichbar). `web::serve` nimmt
   Verbindungen jetzt selbst an (hyper `http1` mit `header_read_timeout` 10 s, `hyper`/`hyper-util` als direkte
   Abhängigkeiten, keine neuen Crates); danach nach 15 s keine offen.
4. IPv6-Erweiterung länger als das Paket ergab verdrehte Bereiche (`118..58`) für die Transportschicht – ohne
   Absturz, jetzt als abgeschnitten markiert (`net.rs`, Prüfung auch in `dissect_with`).
5. `back`-Pfade mit Tab/Zeilenumbruch (`/\t/evil.example`, Browser machen daraus `//evil.example`) wurden
   übernommen, u. a. im Link „Zurück“ von `/packet?back=…` – jetzt abgelehnt (`back_path`).
6. Favoriten-Adressen: Doppelte per `Vec::contains` (quadratisch) und beliebig viele ungültige Wörter in der
   Weiterleitung – jetzt `HashSet`, höchstens 1000 Adressen und 20 genannte Fehler (150 000 Adressen: 0,02 s).
7. Fenster-Programm übergab Links jedes Schemas an `xdg-open` – jetzt nur `http(s)://`.

**Neue Tests:** `src/fuzz.rs` (mutierte Pakete aller Protokolle auf allen Link-Typen inkl. Folgesegmenten, mit
Prüfung der Bereiche und Filtern; zufällige und maximal verschachtelte Filtertexte; Länge per `FUZZ_ITERATIONS`,
`FUZZ_SEED`, lang: `FUZZ_ITERATIONS=1000000 cargo test --release fuzz` – 3 × 1 Mio. ohne Befund), IPFIX mit
verstümmelten Datagrammen. Angriffsskripte lagen nur im Scratchpad (nicht im Repo).
Nebenbei: `dev/check-notices.sh` meldete seit 4.7 `SECURITY.md`, `CONTRIBUTING.md` und die drei Issue-Vorlagen, weil
das Original Dateien gleichen Namens hat. Geprüft: neu geschrieben, mit dem Original nur YAML-Schlüssel des
GitHub-Formats gemeinsam – das Skript nimmt sie jetzt als „ersetzt“ aus (Liste `replaced`).

Ursprünglicher Plan:
- Anmeldung: Startlink nur für den Benutzer lesbar, Token nur einmal gültig?, Sitzungscookie (`HttpOnly`,
  `SameSite`), Abmelden, viele gleichzeitige Anmeldeversuche.
- CSRF auf allen POST-Routen, `Host`-/`Origin`-Prüfung (DNS-Rebinding), Sicherheits-Header und CSP, `/app/*` nur mit
  Bearer-Token.
- DoS/Ressourcen: `/api/packets` mit großen `count`/`from`, sehr lange oder tief verschachtelte Anzeige- und
  Verbindungsfilter (Rekursion), `/api/filter`, Stream folgen bei großen Strömen, PCAP-Download mit Filter, viele
  gleichzeitige Anfragen, Paketspeicher-Grenzen, Favoriten-Adressen (Grenze 1000), Einstellungsformular mit
  Riesenwerten.
- Zerleger mit kaputten Paketen (Fuzzing kurz mit `cargo fuzz` oder zufälligen Bytes über die Testschnittstelle):
  DNS-Kompression mit Schleifen, TLS-Längenfelder, DHCP-Optionen, IPFIX-Vorlagen.
- Rechte: nur `cap_net_raw`/`cap_net_admin` auf `/usr/bin/openpacketmon`, das Fenster-Programm ohne; Server nur auf
  127.0.0.1; Dateirechte von Konfiguration und Startlink.
- `cargo audit`/`cargo deny` ohne Befund; Funde beheben, danach erneut prüfen.

Test-Paket **0.2.5** (2026-10-29) mit allen Korrekturen aus 4.8 gebaut (`dev/build-deb.sh`).

#### 4.9 ✅ Historie neu schreiben (neuer Anfangs-Commit) – erledigt 2026-10-29

Freigabe „zuerst 4.9 danach commiten und pushen“. Umgesetzt:
- Sicherung `~/Dokumente/openpacketmon-history-bis-2026-10-29.bundle` (82 MB, `git bundle verify` ok; `main` =
  `d05cbf5`, `design-muster`, Tag `upstream-base`).
- Altes Repo umbenannt in `tomCSLK/openpacketmon-old` (privat), neues privates Repo `tomCSLK/openpacketmon` ohne
  Vorlagen, Actions erlaubt; Beschreibung übernommen.
- Ein Anfangs-Commit „openpacketmon 0.2.5 – fork of Sniffnet …“ mit dem Stand von `d05cbf5` plus Version 0.2.5 und
  dieser Doku; Tag `upstream-base` lokal gelöscht, `design-muster` bleibt lokal (nie pushen).
- Doku: `CLAUDE.md`, `CHANGELOG.md` (kein Tag mehr, Bezug Commit `ab4bf739d464`), Übersicht.

Ursprünglicher Plan:


Voraussetzung: 4.1–4.8 erledigt, committet, CI grün, Arbeitsbaum sauber.
1. Sicherung der alten Historie **lokal**, außerhalb des Repos: `git bundle create
   ~/Dokumente/openpacketmon-history-bis-<datum>.bundle --all` (enthält auch `upstream-base` und `design-muster`).
2. **Neues GitHub-Repo statt Force-Push** (empfohlen): Bei einem Force-Push auf das bestehende Repo bleiben alte
   Commits über ihre Hash-URL, Actions-Läufe und Caches noch eine Weile abrufbar. Deshalb:
   - altes Repo umbenennen in `tomCSLK/openpacketmon-old` und **privat** lassen (Archiv; später löschen, wenn der
     Nutzer möchte);
   - neues privates Repo `tomCSLK/openpacketmon` anlegen (ohne README/Lizenz-Vorlage), Actions erlauben.
3. Lokal: `git checkout --orphan clean main`, alles übernehmen, ein Commit „openpacketmon <version> – fork of
   Sniffnet (commit ab4bf739d464, MIT OR Apache-2.0)“ mit kurzer Beschreibung und Verweis auf `NOTICE`/CHANGELOG,
   Autor wie bisher; Zweig `main` darauf setzen, `origin` auf das neue Repo, `git push -u origin main`.
   Tag `upstream-base` lokal löschen (bleibt im Bundle); `design-muster` bleibt lokal und wird nie gepusht.
4. Prüfen: `git log` hat einen Commit, `git rev-list --all --objects` enthält weder `services.txt` aus nmap noch
   `GeoLite2-*.mmdb` noch Dateien des Originals, die entfernt wurden; CI im neuen Repo grün; Update-Prüfung
   (`releases` von `tomCSLK/openpacketmon`) funktioniert weiter.
5. Doku anpassen: `CLAUDE.md` (kein Tag `upstream-base` mehr; Upstream-Fixes per `git format-patch` aus einem
   temporären Upstream-Klon, Bezug ist Commit `ab4bf739d464`), Übersicht `~/Claude/Projekte/CLAUDE.md`.

#### 4.10 Umschalten und erstes öffentliches Release

**Haltepunkt (2026-10-30 bis 2026-11-02):** Ein Befund aus 4.8 wurde am 2026-10-30 an die Entwickler von Sniffnet gemeldet;
das Repo blieb deshalb zunächst privat. **Aufgehoben (Nutzer, 2026-11-02):** Ohne Antwort von Sniffnet hat der Nutzer
entschieden, das Repo sofort öffentlich zu schalten, weil die Tester unter Windows einen signierten Installer brauchen und die
kostenlosen Signaturwege ein öffentliches Projekt verlangen (5.3, `docs/windows.md` 6.1). Sniffnet wurde vorher benachrichtigt.
Einzelheiten zum Befund stehen nur in `CLAUDE.local.md` und kommen erst nach der Behebung im Original in öffentliche Dateien.
`dev/release.sh --publish` hält bei einem privaten Repo an.

Vorbereitet (2026-10-30):
- `.deb`: `depends = "$auto, libcap2-bin"`. Vorher fehlte `libc6`, das Paket ließ sich auf zu alten Systemen fehlerfrei
  installieren und startete dann nicht (gebaut unter Zorin 18.1/Ubuntu-24.04-Basis: glibc ≥ 2.39). Prüflauf: `libc6
  (>= 2.39)`, `libcap2-bin`, `libgtk-3-0t64`, `libwebkit2gtk-4.1-0 (>= 2.35.1)` u. a.
- `dev/release.sh` nach dem Vorbild von opennetmon, nur `.deb`, `THIRD-PARTY-LICENSES.txt`, `SHA256SUMS` (kein tar.gz,
  das Fenster-Programm braucht ohnehin WebKitGTK). **Der Release-Titel ist `vX.Y.Z`**, weil die Update-Prüfung den Namen als
  Version liest. Noch nicht ausgeführt (nur Syntax geprüft); der Trockenlauf folgt.
- README (Status, Installation, Netzzugriffe des Programms), CHANGELOG zu einer Liste „was anders ist als bei Sniffnet“
  verdichtet. Vor dem Release wird `[Unreleased]` zu `[X.Y.Z]`, das Skript prüft das.
- Geklärt: Die Töne erzeugt der Browser per Oszillator (`static/live.js`), keine Tondateien im Repo.

Stand 2026-10-30: Commit `55f5213` gepusht, CI grün. Test-Paket **0.2.7** gebaut (enthält gegenüber 0.2.6 nur die
korrigierten Paket-Abhängigkeiten). Von Sniffnet noch keine Antwort.

**Geändert (Nutzer, 2026-10-31):** Das erste Release wird 0.4.0 oder 0.5.0 statt 0.3.0, siehe 5.5. Der Haltepunkt gilt unverändert.

**Entschieden (Nutzer, 2026-10-30):** Antwort von Sniffnet abwarten, dann umschalten und **0.3.0** als erstes Release
veröffentlichen. Bleibt die Antwort aus, entscheidet der Nutzer über das weitere Vorgehen.

**Neu gefasst (2026-11-02):** Die Liste darunter galt für die Browser-Oberfläche (0.3.0, `app/`, WebKitGTK). Seit dem
Umstieg auf egui (5.5) gibt es kein Fenster-Programm `app/` mehr, das Paket hängt nicht mehr an WebKitGTK, und das erste
Release wird 0.4.0 oder 0.5.0. Vor dem Release erledigt (2026-11-02, auf Ansage, ohne den Haltepunkt zu berühren):
- **`deny.toml` neu bewertet:** Die drei Ausnahmen RUSTSEC-2026-0370/0371/0372 bleiben. Sie sind „unmaintained“ (zvariant,
  zbus_names, zvariant_derive gingen in zbus 6 auf), keine Schwachstelle; sie kommen nur über `accesskit_unix` (Screenreader
  von egui), und accesskit 0.24.1/`accesskit_atspi_common` 0.21.1 sind die neuesten Fassungen, noch mit zbus 5. Prüfdatum
  steht in `deny.toml`. Schriften: Die Ausnahme für `epaint_default_fonts` bleibt, aber deren Lizenztexte fehlten in
  `THIRD-PARTY-LICENSES.txt`, weil sie im Unterordner `fonts/` liegen: Ubuntu Font Licence, OFL (Noto Emoji), Hack mit
  Bitstream-Vera-Lizenz, MIT (Emoji Icon Font). `dev/third-party-licenses.py` nimmt sie jetzt über `EXTRA_LICENSE_FILES` mit
  und hält an, wenn eine dieser Dateien fehlt.
- **Unzuverlässiger Test `test_increments_ping_sequence` behoben:** Er erwartete 0, 1, 2 vom gemeinsamen Zähler
  `PING_SEQUENCE`, den die gleichzeitig laufenden Ping-Tests mit hochzählen. Jetzt zählt er einen eigenen Zähler über
  `next_from` und prüft auch den Überlauf nach `u16::MAX`. `latency.rs` ist damit eine geänderte Datei des Originals
  (Kopfzeilen, `NOTICE`); das fand der Trockenlauf.
- **Sicherheitsdurchsicht** (Teile seit 4.8: egui-Oberfläche, Mitschnitt-Helfer, MCP, Geo-Download, Update-Prüfung):
  - Helfer: übersetzt den BPF-Filter jetzt erst **nach** dem Abgeben der Rechte; ein Fehler im BPF-Übersetzer von libpcap
    bekommt so keine Rechte mehr. Neuer Test `refuses_a_broken_filter_after_giving_up_the_rights` im Netz-Namensraum;
    `inside_the_namespace` prüft weiter, dass der Filter wirkt und die Rechte weg sind.
  - Update-Prüfung: Zeitlimit 30 s (vorher keins, eine hängende Verbindung hätte die Prüfung für immer offen gelassen).
  - Ohne Befund: Links im Fenster nur zu festen Adressen (Quellcode, Releases, DB-IP); Paketinhalte werden überall nur als
    druckbares ASCII gezeigt, alles andere als `·` (`dissect::text`, Stream, Hex-Dump), Steuerzeichen beim Kopieren
    ersetzt; fremde Unicode-Zeichen kommen nur aus Geo-Datenbanken, Programm- und Hostnamen, fehlende Zeichen zeichnet
    `MissingGlyphPolicy::Tofu` als Kästchen; lange Texte in Listen werden abgeschnitten. Geo-Download mit Grenzen (64 MiB
    gepackt, 256 MiB entpackt, 120 s). Protokoll Helfer → Programm mit Längengrenzen (256 KiB je Paket, 4 KiB Text). MCP: Stand
    der Prüfung aus 5.4 Stufe 4 (24 von 24 Angriffen abgewehrt), seitdem unverändert.
  - Langer Zufallstest im Release-Bau mit 300 000 Runden je Test (Zerleger, Filter, Masker, MCP-Werkzeuge) ohne Befund.
  - Rechte des Helfers: Das Paket setzt sie auf `/usr/bin/openpacketmon-capture` für **alle** lokalen Benutzer, wie
    Sniffnet bei seinem Programm; Wireshark erlaubt es nur der Gruppe `wireshark`. Entschieden (Nutzer, 2026-11-02): so
    lassen, die README (Security) beschreibt, wie man es auf eine Gruppe `openpacketmon` beschränkt (`groupadd`, `usermod`,
    `dpkg-statoverride root openpacketmon 0750`, danach erneut `setcap`, weil der Gruppenwechsel die Rechte löscht).
    Im Container (Debian 13, `--cap-add NET_ADMIN,NET_RAW`) geprüft: Gruppenmitglied startet den Helfer (Antwort
    „bereit“), ein anderer Benutzer bekommt „Permission denied“, die Einstellung übersteht `apt install --reinstall`;
    nach `purge` bleibt der Eintrag von `dpkg-statoverride` stehen und wird mit `--remove` gelöscht (steht in der README).
- **Trockenlauf `dev/release.sh`** (erstmals, mit vorübergehendem Abschnitt `## [0.3.9]` im CHANGELOG, danach
  zurückgesetzt): Der erste Lauf hielt an der fehlenden Kennzeichnung von `latency.rs` an (behoben), der zweite lief durch:
  Prüfungen, Release-Bau, Paket mit allen Pflichtdateien, Version und Abhängigkeiten, `SHA256SUMS`. Abhängigkeiten des
  Pakets: `libasound2t64`, `libc6 (>= 2.39)`, `libcap2-bin`, `libpcap0.8t64`, `libvulkan1 | libegl1`, Wayland-, X11- und
  xkbcommon-Bibliotheken. Damit läuft es erst ab Ubuntu 24.04 und Debian 13 (die `t64`-Pakete und glibc 2.39 fehlen
  Debian 12 und Ubuntu 22.04). Der Teil mit `--publish` ist weiter ungetestet.
- **Installationstest im Container** (podman, frische Images `ubuntu:24.04` und `debian:trixie`, Paket aus dem
  Trockenlauf): Installation mit `apt` ohne Empfehlungen, `openpacketmon --version`, keine fehlenden Bibliotheken
  (`ldd`), `postinst` setzt `cap_net_admin,cap_net_raw=eip` auf den Helfer, `--list-adapters`, Desktop-Datei, Entfernen mit
  `purge` sauber. Im Ubuntu-Image fehlen `NOTICE` und `THIRD-PARTY-LICENSES.txt` unter `/usr/share/doc/openpacketmon/`, weil
  das minimierte Image `/usr/share/doc/*` ausschließt; im Paket sind sie drin, und das Programm zeigt beide Texte selbst
  (Seite Lizenzen). Fenster und Mitschnitt gehen im Container nicht, siehe Schritt 4.

Noch offen, in dieser Reihenfolge:
1. ~~Antwort von Sniffnet~~ Haltepunkt am 2026-11-02 aufgehoben, Sniffnet vorher benachrichtigt (Einzelheiten
   `CLAUDE.local.md`). Sniffnet hat am 2026-11-08 geantwortet und einen Pull Request erlaubt; der Fix liegt als
   GyulyVGC/sniffnet#1357 vor (2026-11-09), die Rückmeldung dazu steht aus. Herkunftsangabe im Advisory prüfen, sobald
   der Pull Request übernommen oder abgelehnt ist.
2. ~~Nutzertest der egui-Fassung abschließen~~ erledigt: Test-Paket 0.3.10 ohne Befund (2026-11-02, 5.5).
3. ~~Version setzen~~ erledigt 2026-11-02: **0.4.0** (Nutzer), Commit `4e94702`, CI grün. Ursprünglich: Version **0.4.0 oder 0.5.0** setzen: `Cargo.toml`, `capture/Cargo.toml`, `cargo update --workspace --offline`; in
   `CHANGELOG.md` `## [Unreleased]` zu `## [X.Y.Z] - Datum`; Release-Hinweise (Englisch) als Datei; committen, pushen, CI grün.
4. Installationstest mit Fenster und Mitschnitt auf einem sauberen Ubuntu 24.04 und Debian 13 (VM oder anderer Rechner);
   der Test im Container (siehe oben) prüft nur Abhängigkeiten, Installation, Rechte und Entfernen.
5. ~~Repo-Einstellungen~~ erledigt 2026-11-02: Beschreibung „Network traffic monitor for Linux and Windows, based on
   Sniffnet“, Themen `linux`, `network-monitor`, `packet-capture`, `rust`, `windows`, Wiki/Projects aus, Ruleset
   „protect main“ (kein Force-Push, kein Löschen von `main`).
6. ~~Sichtbarkeit auf öffentlich~~ erledigt 2026-11-02 (Nutzer): per `gh api -X PATCH repos/tomCSLK/openpacketmon -f
   visibility=public` (das installierte `gh` kennt `--accept-visibility-change-consequences` nicht), danach „Private
   vulnerability reporting“ an (`enabled: true`). `tomCSLK/openpacketmon-old` bleibt privat.
7. ~~Release~~ erledigt 2026-11-02: **v0.4.0** mit `dev/release.sh --publish --notes <datei>` (nur Linux,
   https://github.com/tomCSLK/openpacketmon/releases/tag/v0.4.0; `.deb`, `THIRD-PARTY-LICENSES.txt`, `SHA256SUMS`,
   Download-Prüfung im Skript ok); `test_fetch_latest_release_from_github` danach grün. Vorher Installationstest des 0.4.0-Pakets
   in Containern (Ubuntu 24.04, Debian 13) bestanden. Punkt 4 (Test mit Fenster auf sauberem System) steht weiter aus.
   Nutzer (2026-11-02): `.deb` aus dem Release heruntergeladen, als Update über 0.3.10 installiert, läuft wie erwartet. Vorher meldete
   0.3.10 nach dem Start die neue Version 0.4.0, der Link führte zum Release (Update-Prüfung im echten Einsatz bestätigt).
8. ~~Doku nachziehen~~ erledigt 2026-11-02: `CLAUDE.md`, `PLAN.md`, `docs/windows.md`, Übersicht `~/Claude/Projekte/CLAUDE.md`; die
   Notiz zum Haltepunkt gilt jetzt nur noch dafür, Einzelheiten zum Befund nicht zu veröffentlichen.

Geklärt (2026-10-30): Herkunft der Vorlage `../design-system`, siehe Phase 3 Punkt 7.

#### Reihenfolge und Aufwand

4.4 zuerst (braucht den Tag) → 4.1 → 4.2 → 4.3 → 4.5 → 4.6 → 4.7 → 4.8 → 4.9 → 4.10. Größte Brocken: 4.1 (Namen und
Kategorien abgleichen), 4.2 (Download und Hinweise), 4.8 (eigener Prüfdurchgang). Commit, Push und `.deb` nur auf
Anweisung des Nutzers.

### Phase 5 – Erweiterungen (gemeinsam festlegen)

#### 5.1 „Kleines Wireshark“ (beschlossen 2026-10-28, Reihenfolge festgelegt; Umsetzung je Stufe erst nach Freigabe)

Ziel: openpacketmon kann neben den Zusammenfassungen einzelne Pakete zeigen, zerlegen, filtern und analysieren – im
Funktionsumfang eine kleinere Version von Wireshark, zugeschnitten auf das Heimnetz. Reihenfolge:

1. ✅ **Stufe 1 – Paketspeicher, Paketliste, Paketdetails** (enthält Punkt 2 der Testliste „Rohdaten auf der Verbindungsseite“)
   **Erledigt am 2026-10-28**, so umgesetzt:
   - `src/engine/packets.rs`: Ringpuffer über alle Pakete einer Aufzeichnung (Live, Live mit PCAP-Export, Datei), Grenze
     10 000/100 000 (Standard)/250 000/500 000 Pakete und immer höchstens 512 MiB, älteste fallen heraus; Pakete ohne IP
     (LLDP, STP …) werden mit gespeichert (ohne Verbindung, nur ohne Filter sichtbar). Nur im Arbeitsspeicher, nicht auf Platte.
     IPFIX: Hinweis „keine einzelnen Pakete“. Beim Beenden der Aufzeichnung wird der Speicher wie die übrigen Daten verworfen.
   - `src/dissect/` (eigene Zerlegung nach RFCs/IEEE, kein Wireshark-Code, kein neues Crate): Ethernet II/802.3, VLAN/QinQ,
     Linux SLL v1/v2, BSD-Loopback, Raw IP, ARP, IPv4 (Prüfsumme geprüft, Fragmente), IPv6 mit Erweiterungs-Headern,
     TCP (Flags, Optionen MSS/WS/SACK/Timestamps), UDP, ICMP, ICMPv6 (ND-Ziel), IGMP; jedes Feld kennt seine Bytes;
     Feldnamen englisch. Test mit allen Präfixen und Zufallsbytes (keine Panik).
   - Seite **Pakete** (`/packets`): Nr., Zeit (s seit dem ersten gespeicherten Paket), Quelle, Ziel, Protokoll, Länge, Info;
     Filter nach Quelle/Ziel/Ports/Protokoll mit „auch Gegenrichtung“, Seiten, ohne Seitenangabe folgt sie live den neuesten
     Paketen; Zeile anklicken öffnet das Paket; Fußzeile mit Anzahl, Speicher, Link-Typ, verworfenen Paketen.
   - **Paketdetails** (`/packet?n=`): Frame-Angaben, Schichtenbaum, Hex-Dump mit Text (bis 16 KiB angezeigt); Feld anklicken
     oder überfahren markiert die Bytes, Byte anklicken wählt das kleinste Feld; Vorheriges/Nächstes, „Zur Verbindung“.
   - **Als PCAP herunterladen** (`/packets.pcap`, alle bzw. das Filterergebnis); Verbindungsseite verlinkt ihre Pakete.
   - Einstellungen „Paketspeicher“: an/aus (Standard an), Anzahl, „Nur Kopfdaten“ (erste 128 Bytes, Standard aus); gilt ab
     der nächsten Aufzeichnung.
   Ursprünglicher Plan:
   - Paketspeicher: Ringpuffer über alle Pakete in `parse_packets.rs` (Zeitstempel, Original-/gespeicherte Länge, Bytes,
     Verbindungsschlüssel), Grenze einstellbar (Vorschlag 100 000 Pakete bzw. 200 MB, älteste fallen heraus); optional
     zusätzlich als pcapng auf die Platte. Bei Capture-Dateien gleiche Pufferung beim Einlesen; IPFIX hat keine Pakete (Hinweis).
   - Neue Seite **Pakete** (wie das Hauptfenster von Wireshark): Nr., Zeit (relativ/absolut), Quelle, Ziel, Protokoll, Länge,
     Info; live, Filterung und Seitenbildung auf dem Server (kein Laden aller Pakete in den Browser).
   - **Paketdetails**: Schichtenbaum (Ethernet, VLAN, ARP, IPv4/IPv6, TCP/UDP/ICMP mit Feldern) und Hex-Dump mit ASCII-Spalte;
     Verbindungsseite zeigt die Pakete ihrer Verbindung.
   - **Als PCAP herunterladen** (aktuelles Filterergebnis bzw. eine Verbindung).
   - Einstellungen: Grenze des Speichers; ob Nutzdaten gespeichert werden oder nur Kopfdaten (Datenschutz, klären).
2. ✅ **Stufe 2 – Protokolle der Anwendungsschicht** (füllt die Info-Spalte und den Schichtenbaum)
   **Erledigt am 2026-10-29** (Freigabe „jetzt mit stufe 2 weitermachen“), so umgesetzt – alles eigener Code nach RFCs,
   kein neues Crate, Feldnamen englisch, jedes Feld mit seinen Bytes:
   - `src/dissect/app.rs` wählt nach Port und ersten Bytes: TCP – TLS-Record, HTTP/1, DNS über TCP (Port 53), sonst auf
     TLS-Ports (443, 465, 636, 853, 993, 995, 5223, 8443, 8883) „Continuation Data“; UDP – DNS 53, mDNS 5353, LLMNR 5355,
     DHCP 67/68, DHCPv6 546/547, SSDP 1900, NTP 123, QUIC 443.
   - `dns.rs`: Kopf mit Flags, Fragen (mDNS: Unicast-Bit), Antworten/Autorität/Zusätzliches (mDNS: Cache-Flush), Namen mit
     Kompression und Schleifenschutz; RDATA A, AAAA, CNAME, PTR, NS, MX, TXT, SRV, SOA, SVCB/HTTPS (Priorität, Ziel, ALPN,
     Adresshinweise, ECH), EDNS. Info z. B. „Response 0x1a2b A github.com: 140.82.121.4“, Fehler wie „No such name“.
   - `dhcp.rs`: BOOTP-Kopf, Optionen (Nachrichtentyp, gewünschte IP, Server, Lease, Maske, Router, DNS, Hostname, Domain,
     Vendor-Class, Client-ID, Parameterliste, FQDN …); Info „DHCP Request … - laptop (wants 192.168.1.20)“. DHCPv6 nur Typ und ID.
   - `tls.rs`: mehrere Records je Segment, Handshake-Typen; Client Hello mit SNI, ALPN, supported_versions, Gruppen,
     Key-Share-Gruppen, ECH, Cipher Suites (IANA-Namen, GREASE); Server Hello mit Version und Suite; Alerts,
     Change Cipher Spec, Application Data, verschlüsselte Handshake-Nachrichten. Protokollspalte „TLSv1.2“/„TLSv1.3“.
   - **Zusammensetzen über TCP-Segmente**: geht ein Handshake-Record über das Segment hinaus (großes Client Hello mit
     ML-KEM-Schlüssel), setzt `Dissection::followup` die nächste Sequenznummer; `src/web/packets.rs` sucht in den nächsten
     50 gespeicherten Paketen derselben Verbindung und Richtung die passenden Segmente (höchstens 16 KiB, Wiederholungen und
     Pakete außer der Reihe werden übergangen) und zerlegt erneut mit `dissect_with`. Liste und Details zeigen dann die SNI;
     die Detailseite nennt die verwendeten Pakete, der Hex-Dump zeigt die angehängten Bytes mit.
   - `text.rs`: HTTP/1 (Startzeile, Header, Body-Länge, Content-Type; Info mit Host), SSDP (M-SEARCH/NOTIFY/Antwort mit
     NT/ST, NTS und SERVER/USER-AGENT). `misc.rs`: NTP (LI/Version/Modus, Stratum, Referenz, Zeitstempel in UTC), QUIC
     (Long Header: Version 1/2/Draft, Typ, Verbindungs-IDs, Versionsverhandlung; Short Header nur „Protected Payload“).
   - Detailseite zeigt drei Ebenen (z. B. Handshake → Cipher suites → einzelne Suite).
   - Tests: 13 neue (DNS mit Kompression, NXDOMAIN, Zeigerschleife, DNS über TCP, mDNS, DHCP, TLS-SNI auch über zwei
     Segmente und im Paketspeicher, mehrere Records, HTTP, SSDP, NTP, QUIC, verfälschte Nutzlasten auf allen Ports ohne
     Panik). Mit einer erzeugten PCAP (14 Pakete, alle Protokolle) Liste und Details geprüft.
   Ursprünglicher Plan:
   - DNS (Abfragen/Antworten), TLS-ClientHello mit SNI (Ziel-Domain trotz HTTPS, ohne Entschlüsselung), DHCP, mDNS, SSDP
     (Gerätenamen, Hersteller), HTTP/1 (Anfragezeile, Status), ARP und ICMP im Klartext.
   - Nutzen fürs Heimnetz: z. B. sehen, mit welchen Zielen Smart-TV oder Sprachassistent sprechen.
3. ✅ **Stufe 3 – Anzeigefilter und „Stream folgen“** – **erledigt am 2026-10-29** (Freigabe „ja, mache damit weiter“),
   zusammen mit der **Live-Paketliste** (Rückmeldung des Nutzers: Aktualisierung zu langsam, „fast wie bei Wireshark“):
   - **Live-Liste**: vorher wurde die Liste alle 5 s ganz neu geladen und gar nicht, solange die Maus darüber stand. Jetzt
     fragt `static/packets.js` alle 0,3 s `GET /api/packets?after=N&f=…&size=…` ab und bekommt nur die neuen Zeilen als HTML
     (bei mehr als einer Seite neu: ganze Liste); die Tabelle ist ein eigener Scrollbereich mit festem Kopf und folgt den
     neuesten Paketen, solange sie unten steht. Knopf „Live“ hält an/setzt fort. Seitenwahl (`page=`) bleibt ohne Live.
     Zeilen je Seite 50–1000 (Standard 100). Die übrigen Live-Bereiche (Verbindungen, Hosts, Toplisten) laden jetzt alle 2 s
     statt 5 s nach; Kacheln und Diagramm bleiben im Sekundentakt des Kerns.
   - **Anzeigefilter** (`src/filter/`, eigene Grammatik und eigener Code): Protokolle (`dns`, `tls` …) und rund 90 Felder
     (`ip.addr`, `tcp.port`, `port`, `tcp.flags.syn`, `dns.qry.name`, `dns.a`, `tls.sni`, `http.host`, `dhcp.hostname`,
     `frame.len`, `domain` = DNS-Name/SNI/HTTP-Host …); `== != < <= > >= contains in {…}`, `and or not`, Klammern; Text ohne
     Groß-/Kleinschreibung, Adressen auch als Netz; Namen für Werte (`dns.qry.type == AAAA`, `dhcp.type == request`,
     `tls.version == 1.3`). Die Zerleger liefern die Werte (`Dissection::values`/`protocols`). Fehler mit Stelle und
     Erklärung (DE/EN); das Eingabefeld wird beim Tippen grün/rot (`GET /api/filter`). Grenzen: 4096 Zeichen,
     Verschachtelung 64. Hilfe mit Beispielen, Protokoll- und Feldliste auf der Seite. Die alten Felder (Quelle/Ziel/Ports)
     sind ersetzt; „Pakete dieser Verbindung“ setzt einen Filter für beide Richtungen.
   - **Treffer-Zwischenspeicher** (`src/web/packet_index.rs`): je Filter (bis 4) die passenden Paketnummern, nur neue
     Pakete werden zerlegt; der Speicher wird nur kurz für Kopien von 2048 Paketen gesperrt (Bytes jetzt `Arc<[u8]>`), damit
     der Mitschnitt nicht wartet. Ein über TCP-Segmente geteiltes Client Hello wartet bis 1 s auf sein Folgesegment.
     PCAP-Download nutzt denselben Filter.
   - **Farbregeln**: Fehler/Reset (RST, ICMP-Fehler, DNS-Fehler, TLS-Alert, HTTP ≥ 400) rot, TCP-Auf-/Abbau grau, DNS blau,
     ARP/DHCP/SSDP/IGMP/NTP gelb, TLS/QUIC violett, HTTP grün, ICMP rosa, sonstiges UDP türkis; abschaltbar (Kästchen
     „Farben“, im Browser gemerkt), Legende in der Hilfe; hell und dunkel. Regeln als Filterausdrücke (`filter::COLOR_RULES`).
   - **Stream folgen** (`/stream?n=`, `src/web/stream.rs`): Nutzdaten einer TCP-/UDP-Verbindung aus den gespeicherten
     Paketen, TCP nach Sequenznummern (Wiederholungen und Überlappungen entfernt, Lücken angezeigt; Pakete außer der
     Reihe werden nicht umsortiert), Text oder Hex, beide Richtungen oder eine, farbig; Bytes und Pakete je Richtung; bis
     1 MiB angezeigt. Knöpfe auf der Paketdetailseite: „Stream folgen“, „Filter: diese Verbindung“.
   - Tests: Filter (Felder, Vergleiche, Netze, Namen, Fehler mit Stelle, Grenzen, Müll), Farbregeln, Treffer-Zwischenspeicher
     mit wartendem Client Hello, Live-Auswahl, Verbindungsfilter, Stream-Zusammensetzung; im Headless-Chrome geprüft
     (Anhängen und Kürzen, Mitscrollen, Pause, Filterprüfung beim Tippen, Seitenmodus, Stream, hell/dunkel, keine JS-Fehler).
   - **Nachtrag 2026-10-29 – Fehler behoben: Live-Mitschnitt nur 200 Bytes je Paket.** Der Upstream-Mitschnitt
     (`CaptureType::from_device`) schnitt Pakete ohne PCAP-Export auf 200 Bytes ab (Snaplen); damit waren SNI, Stream
     folgen und der PCAP-Download beim Live-Mitschnitt unvollständig, obwohl „Nur Kopfdaten“ aus war. Jetzt nimmt der
     Mitschnitt ganze Pakete (65535 Bytes, Kernel-Puffer 16 MB statt 2 MB), wenn der Paketspeicher vollständige Pakete
     behält oder exportiert wird (`PacketStoreConf::full_packets`, `live_limits`), sonst weiter 200 Bytes.
     Auf der Seite **Aufzeichnung** neuer Abschnitt **Paketspeicher** (Wunsch des Nutzers): vollständige Pakete / nur
     Kopfdaten / aus und höchstens Pakete, für Adapter und Datei; die laufende Aufzeichnung zeigt den Modus. Die
     Einstellungsseite behält den Abschnitt (Hinweis ergänzt). Tests für die Grenzen und das Startformular.
   Ursprünglicher Plan:
   - Anzeigefilter mit eigener, einfacher Syntax (Teilmenge im Wireshark-Stil): `ip.addr == …`, `tcp.port == 443`, `dns`,
     `tls.sni contains amazon`, `and`/`or`/`not`, Klammern; Fehlermeldung bei falscher Eingabe. BPF-Mitschnittfilter bleibt.
   - TCP/UDP-Stream folgen: Inhalt einer Verbindung als Text/Hex, Richtungen farbig.
   - Farbregeln in der Paketliste (z. B. Fehler/RST rot, DNS blau).
4. ✅ **Stufe 4 – TCP-Analyse und Statistik** – **erledigt am 2026-10-29** (Freigabe „ja, fang mit Stufe 4 an“):
   - **Analyse** (`src/engine/analysis.rs`, eigene Regeln nach RFC 9293): je Aufzeichnung neben dem Paketspeicher
     (`SharedPacketStore::analysis`, Sperrreihenfolge Analyse → Speicher). Sie läuft nicht im Mitschnitt, sondern beim
     Ansehen (Liste, Filter, Details, Statistik) in Paketreihenfolge; der Treffer-Zwischenspeicher füttert sie mit seinen
     Zerlegungen, sonst zerlegt `analysis::advance` selbst. Je Paket ein kleiner Eintrag (Befunde als Bits, Protokollpfad,
     Länge, Zeit, ACK-Bezug mit RTT, Handshake-Dauer, DNS-Antwortzeit). Grenzen: 100 000 Verbindungen, 10 000 offene
     DNS-Anfragen, 16 Lücken und 64 unbestätigte Segmente je Richtung.
   - **Befunde:** Retransmission (Bytes schon gesendet; erneuter SYN), Lücke (vorheriges Segment fehlt), außer der Reihe
     (füllt eine Lücke), doppelter ACK (reiner ACK, gleiche Nummer und gleiches Fenster; SYN-ACK zählt nicht),
     Fenster-Aktualisierung, Nullfenster, Keep-Alive, RST. Zeiten: ACK-RTT (ohne wiederholte Segmente, Karn),
     Handshake SYN → ACK des Clients, DNS Anfrage → Antwort (Adressen, Ports, ID).
   - **Kopfdaten:** `tcp.len` zählt jetzt die Länge auf der Leitung (aus der IP-Länge), damit „Nur Kopfdaten“ keine
     falschen Lücken meldet.
   - **Anzeige:** Info mit Vorsatz `[TCP Retransmission]` usw., Farbregel „Fehler“ (`tcp.analysis.flags`), in den Details
     Feld „[SEQ/ACK analysis]“ im TCP-Teil und Antwortzeit im DNS-Teil, darüber Links „ACK für Paket N“, „bestätigt in
     Paket M“, „Antwort auf Paket N“, „beantwortet in Paket M“ (Suche bis 20 000 Pakete weiter).
   - **Filterfelder:** `tcp.analysis.flags` (alle Probleme), `.retransmission`, `.out_of_order`, `.lost_segment`,
     `.duplicate_ack`, `.zero_window`, `.keep_alive`, `.window_update`, `.acks_frame`, Zeiten `tcp.analysis.ack_rtt`,
     `tcp.analysis.handshake`, `dns.time`, dazu `dns.response_to`. Neue Feldart Zeit: Sekunden (`0.1`) oder mit Einheit
     (`100ms`, `800us`).
   - **Seite „Statistik“** (`/statistics?f=`, `src/web/statistics.rs`, Menü unter „Pakete“, Knopf auf der Paketseite):
     für alle Pakete oder die des Anzeigefilters Kacheln (Pakete, Bytes, Zeitraum, Ø Pakete/s und Datenrate),
     Verkehrsdiagramm (uPlot, Bit/s oder Pakete/s, Schritte so, dass höchstens 300 Punkte entstehen),
     Protokollhierarchie mit Balken, TCP-Analyse (Anzahl je Befund) und Antwortzeiten (Anzahl, Mittel, Höchstwert), jede
     Zeile verlinkt die passenden Pakete. Während einer laufenden Aufzeichnung lädt sie sich alle 2 s nach.
   - Tests: 8 für die Analyse (Handshake/RTT, Retransmission/Lücke/außer der Reihe/Keep-Alive, Dup-ACK/Fenster/Nullfenster/RST,
     wiederholter SYN und neue Verbindung, Kopfdaten, DNS, Filter/Info/Schichten, Einträge folgen dem Speicher), Zeitwerte
     im Filter, Diagrammschritte, Formate; im Headless-Chrome mit einer erzeugten PCAP geprüft (hell/dunkel).
   **Rückmeldungen aus dem Nutzertest (ab 2026-10-29; Punkte 1 und 2 erledigt am 2026-10-29, Freigabe „setz alle drei
   Punkte um, bei der Paketliste Variante A“):**
   1. ✅ **Statistik springt beim Nachladen nach oben** (2026-10-29; erledigt: fünf kleine Fragmente statt einem, das
      Diagramm bleibt stehen und bekommt per `setData` neue Werte, ein Zoom bleibt erhalten, Mindesthöhe für
      `#stats-chart`, `live.js` stellt die Scrollposition wieder her. Geprüft im Headless-Chrome mit einem langsam
      gefüllten FIFO als laufender Aufzeichnung: `scrollY` blieb über 8 Aktualisierungen gleich, Zoom blieb stehen): Während einer laufenden Aufzeichnung wird man beim
      Herunterscrollen bei jeder Aktualisierung (alle 2 s) wieder nach oben geschoben; der untere Teil (Hierarchie,
      Befunde, Antwortzeiten) ist kaum nutzbar. Ursache: Die ganze Seite unter dem Filter ist *ein* Fragment
      (`data-fragment="stats"` in `templates/statistics.html`). `live.js` ersetzt es komplett; im neuen Teil ist
      `#stats-chart` leer, bis `statistics.js` uPlot neu zeichnet. Die Seite ist dadurch kurz etwa 280 px kürzer, der
      Browser setzt die Scrollposition zurück. Die Schutzregel „nicht nachladen, solange die Maus darüber ist“ greift
      beim Scrollen mit dem Mausrad nicht zuverlässig (`pointerover` kommt erst bei Mausbewegung; WebKitGTK im
      Fenster-Programm).
      Ansatz:
      - Fragmente aufteilen: Kacheln, Hierarchie, TCP-Befunde und Antwortzeiten je ein eigenes Fragment; die
        Diagramm-Karte ist kein Fragment mehr. Die Diagrammdaten (`#stats-data`) werden aus der neuen Seite übernommen
        und per `plot.setData()` aktualisiert, statt uPlot abzubauen (bleibt auch beim Zoomen stehen).
      - `#stats-chart` bekommt eine feste Mindesthöhe, damit die Seite nie zusammenfällt.
      - Allgemein in `live.js`: `scrollY` vor dem Ersetzen merken und danach wiederherstellen, wenn sich die
        Seitenhöhe geändert hat (schützt auch Übersicht, Verbindungen, Hosts usw.).
      - Prüfen im Headless-Chrome: herunterscrollen, mehrere Aktualisierungen abwarten, `scrollY` bleibt gleich;
        auch Zoom im Diagramm und Umschalten Bit/s ↔ Pakete/s bleiben erhalten.
   2. ✅ **Paketliste: nur „Zurück“/„Weiter“ zwischen den Seiten** (2026-10-29; **Variante A erledigt**: gemeinsames
      Modul `src/web/pager.rs` + `templates/pager.html` für Pakete *und* Verbindungen – „Erste“, „Zurück“, Seitenzahlen
      (1 … 5 6 7 8 9 … 34), „Weiter“, „Letzte“, Feld „Seite [7] von 34“; in der Paketliste „Gehe zu Paket“
      (`?goto=N`, Strg+G) öffnet die Seite mit dem Paket bzw. dem nächsten passenden, markiert und zentriert die Zeile,
      Hinweis, wenn das Paket nicht in der Liste ist (gefiltert oder schon verworfen). Pos1/Ende bewusst nicht belegt,
      weil sie sonst das Scrollen der Seite stören. Tests für Seitenzahlen und Sprungziel; im Headless-Chrome mit
      100 000 Paketen geprüft, hell/dunkel. Variante B bleibt Option): Bei vielen Seiten (z. B. „Seite 10 von 10“,
      33 238 Pakete) kommt man nur schrittweise vor und zurück; eine Seite oder ein Paket direkt anspringen geht nicht.
      **So macht es Wireshark:** keine Seiten, sondern *eine* durchgehende Liste aller (gefilterten) Pakete mit
      Scrollbalken; es werden nur die sichtbaren Zeilen gezeichnet. Dazu „Gehe zu Paket“ (Strg+G, Paketnummer eingeben),
      erstes/letztes Paket (Strg+Pos1/Ende), vorheriges/nächstes Paket, beim Mitschnitt automatisch zum Ende scrollen
      („Auto-Scroll“); nach dem Ändern des Filters bleibt das gewählte Paket sichtbar.
      Ansatz in zwei Schritten:
      - **A – Seitenleiste ausbauen (klein, zuerst):** `templates/packets.html` → „Erste“, „Zurück“, Eingabefeld
        „Seite [ 7 ] von 10“ (Enter springt), „Weiter“, „Letzte“; bei vielen Seiten zusätzlich Seitenzahlen in der Nähe
        (1 … 6 7 8 … 34). Dazu Feld **„Gehe zu Paket“** (Nummer): öffnet die Seite, die das Paket enthält (mit Filter: das
        nächste passende Paket ab der Nummer), und hebt die Zeile hervor (`#p-<nr>`, Sprung per Anker). Server:
        `PacketQuery.goto`, Position über den Treffer-Zwischenspeicher (`packet_index.rs`, Binärsuche in den Nummern).
        Tastenkürzel: Strg+G fokussiert „Gehe zu Paket“, Pos1/Ende erste/letzte Seite (nur außerhalb von Eingabefeldern).
      - **B – durchgehende Liste wie Wireshark (größer, optional danach):** eine Tabelle mit Scrollbalken über alle
        Treffer; Zeilen werden blockweise (z. B. 200) über `/api/packets?from=<index>&count=` nachgeladen, nur der
        sichtbare Bereich steht im DOM (feste Zeilenhöhe, Platzhalter darüber/darunter). Browser begrenzen die
        Elementhöhe (Chrome etwa 33 Mio. px, WebKit/Firefox weniger): bei 500 000 Paketen × 28 px = 14 Mio. px knapp →
        Höhe ab einer Grenze skalieren. Live: am Ende bleiben = Auto-Scroll. Seiten und Zeilen-pro-Seite entfallen dann.
      Empfehlung: **A jetzt** (behebt das Problem mit wenig Aufwand und Risiko), B später bei Bedarf.
      Prüfen: Tests für Sprungziel und Seitenzahlen, im Headless-Chrome mit `big.pcap` (Seite eingeben, Paket anspringen,
      mit und ohne Filter, live).
   Ursprünglicher Plan:
   - Experteninfos: Retransmissions, doppelte ACKs, Zero Window, RST, Handshake-Dauer, Antwortzeiten (RTT).
   - Protokollhierarchie (Anteile je Protokoll), Verkehrsdiagramm mit Filter.

Bewusst nicht: Hunderte Protokoll-Zerleger, Lua, WLAN-Monitor-Modus, USB/Bluetooth, TLS-Entschlüsselung (höchstens viel
später über `SSLKEYLOGFILE`).

**Lizenz:** Wireshark ist GPL – **kein Code, keine Zerleger und keine Filtergrammatik daraus übernehmen**, sonst wäre
`MIT OR Apache-2.0` nicht haltbar. Eigene Umsetzung oder Crates unter MIT/Apache (`etherparse` schon vorhanden; geprüft
werden u. a. `hickory-proto` für DNS, `tls-parser` für TLS) – Lizenz jedes neuen Crates vor dem Einbau prüfen.
Hinweis: Die Feldnamen (`ip.addr`, `tcp.port`) als Bedienkonvention sind kein Code; trotzdem eigene Grammatik schreiben.

#### 5.2 Weitere Ideen (noch nicht beschlossen)

Langzeitspeicherung (SQLite) und Berichte; Gerätenamen aus AdGuard/Omada; Warnung bei auffälligem Upload. (Fenster-Variante B ist umgesetzt, siehe „Fenster-Programm“.)

#### 5.3 Windows-Version (begonnen 2026-11-02 auf dem Zweig `windows`, am selben Tag nach `main` übernommen)

Alles, was für die Fortsetzung auf einem Windows-Rechner nötig ist, steht in **`docs/windows.md`** (2026-10-30): Software,
Zugänge, Einrichten, die Linux-Bindungen im Code mit ihrer Lösung, Tests, Installer, CI und Reihenfolge. Npcap wird nie mitgeliefert.
**Stand 2026-11-02 (abends):** Debug- und Release-Fassung bauen, Tests grün, Fenster startet. Release-Fassung ohne Konsolenfenster (auch
für die Helfer), mit Icon und Versionsangaben; `openpacketmon.exe mcp` antwortet über Pipes. Installer (Inno Setup) und
Tester-Anleitung da (Absatz „Stufe A und Installer“ unten); Mitschnitt im Fenster vom Nutzer getestet, Tester-Fassung 0.3.11 gebaut,
noch nichts weitergegeben (Smart App Control der Tester blockiert den unsignierten Installer, Signatur angehalten, siehe unten).

**Vorgezogen (Nutzer, 2026-11-02):** Die Windows-Version kommt **vor** dem ersten Release, weil andere Tester eine
Windows-Fassung zum Testen brauchen. Sie enthält MCP (5.4). Der Haltepunkt aus 4.10 gilt weiter: Repo privat, nichts
veröffentlichen. Ebenfalls entschieden (2026-11-02): Gebaut und geprüft wird auf einem **eigenen Windows-Rechner** mit
Claude Code (Einrichtung `docs/windows.md` 1 bis 3, nicht per GitHub Actions vom Linux-Laptop aus); die Tester bekommen **gleich
einen Installer** (zunächst unsigniert), nur als Datei, ohne Zugang zum Repo, solange der Haltepunkt gilt. `docs/windows.md`
ist um MCP, Helfer, `export_pcap`, Adapter ohne IP-Verkehr und den Installer ergänzt.
**Windows-Rechner eingerichtet (2026-11-02, `docs/windows.md` 1):** Rust 1.99.1, cargo-deny 0.20.2, Npcap 1.90 im WinPcap-Kompatibilitätsmodus
und SDK 1.17 nachinstalliert; Git, GitHub CLI, Python und Visual Studio Community 2026 waren schon da. Npcap gibt es nicht über winget
und es lässt sich nicht still installieren; das steht jetzt in `docs/windows.md`. Erster `cargo check --workspace` (Zweig `windows`):
nur ein Fehler (`libc::timeval` in `capture_helper.rs`, behoben mit `as _`), danach grün mit vier Warnungen über Linux-Code
(Einzelheiten `docs/windows.md` oben). `cargo build --workspace` linkt, beide Programme starten. `cargo test --workspace` zuerst mit 2 Fehlschlägen (`geo_dir` ohne
`HOME`; ein Test löscht eine noch offene PCAP-Datei), danach ein zeitabhängiger MCP-Test (Verbindung auf geschlossenen Port hängt unter
Windows 2 s); alle drei behoben, jetzt **grün** (386 bestanden). `geo_dir` nimmt unter Windows `%LOCALAPPDATA%\openpacketmon\geo`.
`cargo clippy -- -D warnings` unter Windows grün (Warnungen über Linux-Code behoben). Offen: Testordner bleiben in `%TEMP%` liegen. Ursache ist kein Testfehler: Die Mitschnitt-Helfer der Adapter-Vorschau erben unter
Windows das Handle der von libpcap geöffneten PCAP-Datei, nach dem Stoppen eines Imports bleibt sie gesperrt (`docs/windows.md` 4
„Helfer erbt offene Dateien“; inzwischen gelöst mit „Stop wartet“, siehe unten). Außerdem: Smart App Control blockiert ab und zu frisch gebaute Dateien (`docs/windows.md` oben).
Das Fenster startet (kurzer Start der Debug-Fassung: Titel stimmt, 9 Helfer für die Vorschau, nach dem Beenden keine übrig);
Bedienung und Mitschnitt noch nicht ausprobiert. **Release-Fassung (2026-11-02, vom Nutzer freigegeben):** ungenutzte
`redirect_stdout_stderr_to_file` entfernt (brauchte die fehlende Crate `gag`); `windows_subsystem = "windows"` nur im Release, Helfer
mit `CREATE_NO_WINDOW`; Icon `resources/packaging/windows/openpacketmon.ico` aus `dev/make-ico.py`, eingebettet von `build.rs` mit
`winresource` (nur Build-Abhängigkeit unter Windows). Folge: Kommandozeilen-Ausgaben der Release-Fassung nur mit umgeleiteter
Ausgabe sichtbar (`| Out-Host`). `dev/third-party-licenses.py` schreibt unter Windows jetzt dieselbe Datei wie unter Linux.
**Stufe A und Installer (2026-11-02, vom Nutzer freigegeben; auf dem Zweig `windows` committet):** Entschieden: Inno Setup, Installation standardmäßig
nur für den Benutzer (wahlweise für alle), gesperrte Datei per „Stop wartet“ ohne `unsafe`, eine gemeinsame Lizenzliste für beide
Plattformen. Umgesetzt (Einzelheiten `docs/windows.md` 4, 6, 7): Adapter mit ihrer Beschreibung statt `\Device\NPF_{…}`
(`MyDevice::display_name`/`is_called`, `--adapter` nimmt auch die Beschreibung); `Engine::stop` wartet, bis der Mitschnitt seine Dateien
geschlossen hat (`CaptureEnded`), eine importierte PCAP-Datei ist danach sofort frei, die Tests räumen ihre Ordner auf und starten keine
echten Helfer mehr; `wpcap.dll` wird verzögert geladen, ohne Npcap erscheint ein Dialog mit Link auf npcap.com (`src/npcap.rs`, neue
Abhängigkeit `webbrowser` nur unter Windows); C-Laufzeit statisch (`.cargo/config.toml`); Live-Test des Helfers auf dem Npcap-Loopback
(Timeout kommt auch ohne Pakete); `resources/packaging/windows/openpacketmon.iss` und `dev/build-installer.ps1` (lokal installiert und
deinstalliert, mit 0.3.10 nur zum Testen); `deny.toml` mit Windows-Ziel und `BSL-1.0`; `THIRD-PARTY-LICENSES.txt` für beide Plattformen
(482 Crates); Tester-Anleitung `docs/windows-tester.md`. Geprüft: `cargo test --workspace` (388 bestanden), Clippy, fmt, `cargo deny`,
`dev/check-notices.sh`. **Offen** (Stand dieses Absatzes; erledigt seither: Mitschnitt im Fenster, Version 0.3.11, Systemsprache,
Rechte von `mcp-access.json`, Linux-Prüfung, siehe unten): Test auf sauberem System (Windows-Sandbox), CI-Job.
**Blinkendes Fenster (2026-11-02, vom Nutzer freigegeben):** Unter Windows nahm wgpu Vulkan, und das Fenster blinkte bei
Mausbewegungen (Intel Arc); mit DirectX 12 und OpenGL nicht. `src/gui/mod.rs` gibt unter Windows jetzt DirectX 12 vor, sonst OpenGL;
`WGPU_BACKEND` gilt weiter (`docs/windows.md` 4 „Fenster“).
**MCP unter Windows geprüft (2026-11-02):** installierte Fassung, Mitschnitt auf USB-Ethernet; alle Lesewerkzeuge antworten über
`openpacketmon.exe mcp`, Programme und Geodaten sind dabei, Paketinhalte bleiben ohne den Schalter gesperrt. Mit dem Schalter liefern
`follow_stream` und `packet_bytes` die Inhalte, eine eigene HTTP-Anfrage mit erfundenen Geheimnissen (Cookie, Bearer, URL-Query) kommt
nur mit `x` an, auch in Hex. `mcp-access.json` ist nur
für SYSTEM, Administratoren und den Benutzer lesbar, keine Änderung nötig (`docs/windows.md` 5 Punkt 9).
Nutzer (2026-11-02): Mitschnitt im Fenster getestet, bisher ohne Probleme. **Systemsprache (vom Nutzer gewünscht, 2026-11-02):** Ohne
Sprache in den Einstellungen und ohne `LANG` nimmt das Programm unter Windows die Anzeigesprachen des Systems (`sys-locale`, nur Windows);
Npcap-Dialog jetzt auf Deutsch. Gefunden: Die MCP-Integrationstests sind unter Windows nicht von den echten Einstellungen getrennt
(`confy` beachtet `XDG_CONFIG_HOME` dort nicht), Lösung offen (`docs/windows.md` 4).
**Erste Tester-Fassung 0.3.11 (2026-11-02, Version vom Nutzer):** Installer `openpacketmon-0.3.11-setup.exe` mit `dev\build-installer.ps1`
gebaut, SHA-256 in `docs/windows.md` 6; unsigniert, Weitergabe nur als Datei.
**Nach `main` übernommen (2026-11-02, Nutzer):** Der Zweig `windows` (`d5e38ce` bis `3061214`) wurde ohne Merge-Commit nach `main`
übernommen. Vorher unter Linux auf dem Zweig geprüft (die CI läuft nur auf `main` und bei PRs, für den Zweig also nie): fmt, Clippy,
`cargo test --locked --workspace` (370 Tests im Hauptprogramm, Live-Test des Helfers im Netz-Namensraum), `cargo deny --workspace check`
mit beiden Zielen, `dev/third-party-licenses.py --check`, `dev/check-notices.sh`, alles ohne Befund. Ungeprüft übernommen sind die
Angaben zu Windows-Updates und Preisen in `docs/windows.md` 6.1 und `docs/windows-tester.md` (KB5127648, April-Update, Certum).
Weiter offen: sauberes System (Windows-Sandbox), CI-Job für Windows, MCP-Tests unter Windows von den echten Einstellungen trennen
(`confy` ohne `XDG_CONFIG_HOME`), Signatur am Haltepunkt (die Tester haben Smart App Control an und können 0.3.11 nicht installieren).
Früher entschieden (2026-10-30/31): Beginn erst nach dem ersten Release und nach MCP; der Installer soll, **wenn möglich, eine Code-Signatur**
bekommen. Weg (`docs/windows.md` 6.1): zuerst kostenlos über die SignPath Foundation, das setzt ein unsigniertes Windows-Release und
einen Build in GitHub Actions voraus, und die Zusage ist nicht sicher, weil deren Regel „No hacking tools“ ein
Mitschnitt-Programm treffen könnte. Sonst ein OV-Zertifikat; Azure Artifact Signing gibt es für Einzelentwickler in Deutschland nicht.
**Signatur angehalten (Nutzer, 2026-11-02):** Die Tester haben Smart App Control an, das den unsignierten Installer 0.3.11 ganz
blockiert. Die kostenlosen Wege (SignPath, Certum Open Source ab 49 €) brauchen ein öffentlich sichtbares Projekt und warten damit am
Haltepunkt 4.10, den der Nutzer abklärt. Ein OV-Zertifikat ist dem Nutzer zu teuer und fällt weg; Smart App Control auszuschalten
ist für Tester kein Ausweg (`docs/windows.md` 6.1, `docs/windows-tester.md`).
**Weiter (2026-11-02):** Haltepunkt aufgehoben, Repo öffentlich, Linux-Release v0.4.0 veröffentlicht. Nächster Schritt: Antrag für die
Signatur. Empfehlung: Certum Open Source mit Cloud-Signatur, Antrag im Browser (geht auch unter Linux), signiert wird beim Bau des
Installers auf dem Windows-Rechner; SignPath bleibt möglich, braucht aber erst ein unsigniertes Windows-Release und einen
Windows-Build in GitHub Actions.
**Certum (Nutzer, 2026-11-02):** Konto im Certum-Shop angelegt. Noch offen: Produkt kaufen, aktivieren, Identität prüfen lassen,
Versorgerrechnung und Projektadresse hochladen (`docs/windows.md` 6.1).
**Signieren im Skript vorbereitet (2026-11-02, vom Nutzer freigegeben, ungetestet):** `dev\build-installer.ps1 -CertSha1 <Fingerabdruck>`
signiert beide Programme, Installer und Deinstaller mit Zeitstempel von Certum und prüft die Signaturen; ohne Fingerabdruck wie bisher
unsigniert. Einrichtung von SimplySign Desktop und Ablauf in `docs/windows.md` 6.1. Testen erst mit dem Zertifikat auf dem Windows-Rechner.
**Certum-Stand (2026-11-02 abends):** „Open Source Code Signing in the Cloud“ (365 Tage) gekauft, Daten eingegeben, automatische
Identitätsprüfung (IDnow) abgeschlossen, ein Dokument hochgeladen; Vorprüfung „Waiting for verification“, Zertifikat „Under
verification“. Warten auf Certum. Die Projektadresse war auf der Vorprüfungsseite nicht zu sehen; fragt Certum danach, nachreichen.
**Zertifikat da (2026-11-03):** ausgestellt, über die SimplySign-App aktiviert, SimplySign Desktop angemeldet, gültig bis 2027-11-03.
Erster signierter Installer `openpacketmon-0.4.0-setup.exe` (Version unverändert aus `Cargo.toml`) gebaut; Programme und Installer
von `signtool verify` bestätigt. Offen: Deinstaller nach einer Installation prüfen, Test bei einem Tester mit Smart App Control,
Tester-Anleitung auf den signierten Installer umstellen, Weitergabe. Details `docs/windows.md` 6.1.
**Veröffentlicht (Nutzer, 2026-11-03):** Test mit Smart App Control bestanden; Installer und neue `SHA256SUMS` zum Release `v0.4.0`
hinzugefügt, Release-Text, README, CHANGELOG und Tester-Anleitung nennen Windows. **Deinstaller geprüft (2026-11-03,
Windows-Sandbox):** veröffentlichter Installer still für den Sandbox-Benutzer installiert (Exitcode 0, ohne Neustart);
Installer, `openpacketmon.exe`, `openpacketmon-capture.exe` und `unins000.exe` mit gültiger Signatur, Zeitstempel von Certum
(`docs/windows.md` 6.1). Offen: CI-Job Windows, sauberes System (Deinstallation, Update, Start ohne Npcap). `dev/release.sh` nimmt den Installer seit 2026-11-03 mit (`--add-windows` für ein bestehendes
Release, auch in Git Bash unter Windows; `--windows` für ein neues Release unter Linux; `docs/windows.md` 6.1).

#### 5.4 MCP-Anbindung (Idee des Nutzers, 2026-10-31; neu gefasst und begonnen 2026-11-02, vom Nutzer freigegeben; Stufen 1 bis 3 erledigt)

Beim Untersuchen von Geräten im Heimnetz mit Claude in einer anderen Instanz (`../Abfrage_Geraete_im_Heimnetzwerk`) soll Claude den Mitschnitt
von openpacketmon direkt abfragen können, statt dass der Nutzer Werte abliest oder PCAPs weitergibt. Der erste Entwurf stammte aus der Browser-Zeit
(Server, Startlink); beides gibt es seit der Umstellung auf egui nicht mehr.

**Zeitpunkt (Nutzer, 2026-11-02):** MCP kommt jetzt **vor** dem ersten Release, weil das Release am Haltepunkt 4.10 wartet. Es bleibt alles im
privaten Repo, und MCP wird mit dem ersten Release veröffentlicht; dessen Sicherheitsdurchsicht muss es also einschließen. Windows bleibt danach.

Entschieden (Nutzer, 2026-11-02):

```
Claude Code ──stdio──> openpacketmon mcp ──127.0.0.1 + Token──> laufendes Fenster
                       (MCP-Server, ohne Daten)                 (Mitschnitt, Engine)
```

- **`openpacketmon mcp`** (`src/mcp/server.rs`): startet der Assistent selbst. Spricht MCP auf stdio (`rmcp`, offizielles SDK, Apache-2.0), kennt die
  Werkzeuge, öffnet weder Fenster noch Mitschnitt und liest die Einstellungen nicht (Abzweig in `main` vor `CONF`, weil das Laden sie bei einem Fehler
  zurücksetzen würde). stdout gehört allein MCP (Test `tests/mcp.rs`).
- **Zugang im Fenster** (`src/mcp/access.rs`): lauscht nur auf `127.0.0.1`, zufälliger Port, neuer Token je Einschalten, Datei `mcp-access.json` neben den
  Einstellungen (Rechte `0600`). Je Frage eine Verbindung mit JSON-Zeilen (`src/mcp/protocol.rs`), kein MCP im lauschenden Teil.
  Seit Stufe 4 weisen sich beide Seiten aus, ohne den Token zu senden: `Hello { nonce }` → `Challenge { nonce, proof = HMAC(token, "window\n"+nonce des Clients) }`
  → `Request { proof = HMAC(token, "client\n"+nonce des Fensters), tool, arguments }` → Antwort. HMAC-SHA256 von Hand mit `sha2` (war schon im Baum).
  Höchstens 4 gleichzeitige Clients, 64 KiB je Frage, 5 s je Schritt, Vergleich in fester Zeit.
- **Standardmäßig aus**, eingeschaltet in den Einstellungen (Karte „KI-Assistenten (MCP)“); dort stehen Zustand, Zahl der Abfragen und der Befehl
  `claude mcp add openpacketmon -- openpacketmon mcp`.
- **Reihenfolge egal:** Ist das Fenster zu, sagt das Werkzeug verständlich, was zu tun ist; `openpacketmon mcp` scheitert daran nicht. Sobald das Fenster
  offen ist, geht es ohne neue Verbindung des Assistenten weiter.
- **Abgelehnt:** MCP über HTTP direkt im Fenster (kein Teilprozess, aber wieder ein Webserver, den wir gerade entfernt haben).
- **Nutzdaten** (Stream, Bytes) nur mit eigenem Schalter, standardmäßig aus.

Stufen, jede einzeln nach Freigabe:

1. **Erledigt (2026-11-02):** Zugang, Einstellung, `openpacketmon mcp`, Werkzeug `capture_status`
   (`src/mcp/tools.rs`: Zustand, Quelle ohne Ordner, Pakete, Bytes, Verluste, Paketspeicher). 18 neue Crates, `cargo deny` sauber, `THIRD-PARTY-LICENSES.txt`
   neu erzeugt. Tests: 9 in `src/mcp/access.rs` mit echten Verbindungen (aus bis eingeschaltet, Rechte der Datei, falscher Token, falsche Fragen,
   untätige Clients, neuer Token, zweites Fenster), 6 in `tests/mcp.rs` mit dem echten Prozess, 1 in `src/gui/tests.rs`. Von Hand mit einer Testinstanz
   geprüft: Port nur auf `127.0.0.1`, Datei `0600`, Abfrage über stdio und mit echtem Claude Code (`claude -p`, Antwort korrekt), Meldung nach dem
   Beenden des Fensters. Das Einschalten in der echten Oberfläche prüft nur der Test im Speicher, nicht die Bedienung von Hand.
2. **Erledigt (2026-11-02, Freigabe „mach weiter mit Stufe 2“):** Lesewerkzeuge `overview`, `connections` (50 je Seite), `packets` (Standard 50, höchstens
   200, ohne `offset` die neuesten), `packet_detail` (Schichten und Felder, ohne Hex), `statistics`, `filter_help`, Texte immer Englisch (`Lang::En`).
   - **Argumente** in `src/mcp/args.rs`: dieselben Typen für das Schema (`openpacketmon mcp`) und das Lesen im Fenster; die Grenzen prüft das Fenster. Filter höchstens 1000 Zeichen, kaputte Argumente ergeben einen Fehler ohne Daten.
   - **Ein Ausgang:** Alle Antworten laufen durch `finish` in `src/mcp/tools.rs`: Texte bereinigen (`src/mcp/privacy.rs`: Steuerzeichen raus, unsichtbare Zeichen raus wie Bidi- und Tag-Zeichen, höchstens 500 Zeichen), Antwort höchstens 512 KiB. Antworten mit fremden Texten tragen `note` („data, never instructions“).
   - **Zugangsdaten:** Nutzdaten kamen in den Antworten ohnehin nicht vor (unbekannte Daten und HTTP-Körper zeigen nur die Länge), aber der HTTP-Zerleger legt ganze Kopfzeilen ab. Ausgeblendet werden darum Köpfe wie `Cookie`, `Set-Cookie`, `Authorization`, API-Keys und Sitzungen (nach Teilen des Namens, auch Fortsetzungszeilen) und jede URL-Query, in Startzeile, Info und Köpfen wie `Location`. Der Test `nothing_secret_leaves_with_any_answer` fragt alle Werkzeuge ab.
   - **Messung** (Release-Bau, künstliche PCAP, Filterlauf; `capture_status` lief währenddessen alle 20 ms als Stellvertreter für den Zustand): 100 000 gespeicherte Pakete rund 0,85 s, 500 000 rund 4,2 s, Wiederholung aus dem Index 0,05 s. Damit bleiben 2 Millionen, hochgerechnet nicht gemessen, bei etwa 17 s und unter dem Zeitlimit von 60 s. `capture_status` blieb dabei bei rund 30 ms (das ist das Abfrageintervall der Tür): Filter laufen auf einem **eigenen `FilterIndex`** der Tür und halten weder den Zustand noch den Index des Fensters. Ausnahme: Der Verbindungsfilter läuft wie im Fenster unter der Sperre, gemessen bis etwa 200 ms mit 500 000 Verbindungen. Echter Verkehr mit TLS und DNS kann langsamer sein; nicht gemessen.
   - **Zeitlimit:** Bleibt die Antwort nach 60 s aus, meldet `openpacketmon mcp` „try again“ (`AskError::NoAnswer`) statt „nicht erreichbar“; das Fenster rechnet weiter, die nächste Frage trifft den fertigen Index.
   - Keine neuen Crates. Tests: 12 in `src/mcp/tools.rs`, 5 in `src/mcp/privacy.rs`, einer mehr in `src/mcp/access.rs` (Argumente über die echte Verbindung), `tests/mcp.rs` prüft alle sieben Werkzeuge auf Lesen und Schema. Von Hand mit echtem Claude Code (`claude -p`, Modell haiku) und einer präparierten PCAP: alle sieben Werkzeuge liefen, kein `SECRET` in der Antwort, die eingeschleuste Anweisung im Kopf `X-Note` hat das Modell als Daten erkannt und nicht befolgt (ein Lauf, kein Beweis).
3. **Erledigt (2026-11-02, Freigabe „jetzt machen wir mit Stufe 3 weiter“):** Schalter für Paketinhalte mit `follow_stream` und `packet_bytes`, Anzeige in der Seitenleiste.
   - **Entschieden (Nutzer, 2026-11-02):** Geheimnisse bleiben auch im Inhalt ausgeblendet, im Text wie im Hex-Dump; der Schalter wird **nicht gespeichert** und geht mit dem Fenster aus.
   - **Schalter** „Paketinhalte erlauben“ in der Karte „KI-Assistenten (MCP)“, nur wählbar, wenn der Zugang an ist (`Access::allow_content`, kein Feld in `McpConf`). Jedes Öffnen und Schließen der Tür setzt ihn zurück. Ist er aus, antworten beide Werkzeuge mit einem Hinweis und ohne Daten; die Tür liest ihn bei jeder Frage.
   - **`follow_stream`** (`number` eines Pakets der Verbindung, `format` `text`/`hex`, `direction` `both`/`client`/`server`, `offset`): Inhalt beider Richtungen wie im Fenster (`view::stream::assemble`, dafür jetzt `stream::conversation`), höchstens 64 KiB je Antwort mit `next_offset`, lange Zeilen umbrochen (400 Zeichen, damit die Bereinigung nichts abschneidet), Lücken als `missing_bytes_before`; über 1 MiB je Verbindung nur der Anfang wie im Fenster.
   - **`packet_bytes`** (`number`): Hex-Dump des ganzen Pakets mit Köpfen, höchstens 16 KiB wie im Fenster, `payload_starts_at`. Auch für gekappte Pakete (neues `dissect::captured_payload`).
   - **Ausblenden in Bytes** (`privacy::Masker`): überschreibt mit `x` bei gleicher Länge die Werte von Köpfen mit Cookie/Auth/Token/Secret/Password/Session/API-Key im Namen (auch Fortsetzungszeilen), Queries in Zeilen von HTTP/SSDP/GENA und in Köpfen, den Rest der Zeile nach `PASS` und `AUTH` (FTP, POP3, SMTP). Er geht eine Richtung Stück für Stück durch und merkt sich, wo die Zeile begann; ein über zwei Segmente geteiltes Geheimnis bleibt verborgen. Nach einer Lücke, und für eine Wiederholung ohne bekannten Anfang, wird der Rest der ersten Zeile ganz verborgen. UDP-Datagramme stehen jedes für sich.
   - **Seitenleiste:** über „Design“ ein Punkt mit Text, solange der Zugang offen ist: grün „KI-Assistenten haben Zugriff“, orange „KI-Assistenten lesen Paketinhalte“; Klick öffnet die Einstellungen.
   - Keine neuen Crates. Tests: 4 neue in `src/mcp/privacy.rs`, 5 in `src/mcp/tools.rs` (Freigabe, Stream ohne Geheimnisse in Text und Hex, Bytes, geteiltes Geheimnis mit Wiederholung und Lücke, lange Streams in Teilen), 1 in `src/mcp/access.rs` (Schalter über die echte Verbindung, endet mit der Tür), 1 in `src/gui/tests.rs` (Einschalten, Hinweis in der Seitenleiste, Ausschalten); `tests/mcp.rs` kennt neun Werkzeuge. Bilder `settings-mcp-*` im Bildtest, hell und dunkel angesehen.
   - **Geprüft mit Test-Paket 0.3.9** (2026-11-02, installiert, Live-Mitschnitt auf WLAN): `claude mcp list` verbunden, Abfragen über `openpacketmon mcp` per stdio (eigener MCP-Client im Scratchpad, derselbe Server, den Claude Code startet). Schalter aus: `packet_bytes` abgelehnt. Schalter an: eine eigene HTTP-Anfrage an `example.com` mit erfundenen Geheimnissen (Query, `Cookie`, `Authorization: Bearer`, `X-Api-Key`) erschien in `follow_stream` (Text und Hex), `packet_bytes` und `packet_detail` nur mit `x` an deren Stelle, auch in den zurückgewandelten Bytes; `http.request.uri contains "…"` auf die Query fand nichts. SSDP und TLS über TCP liefen ohne Fehler, ein Versatz hinter dem Ende ergibt eine leere Antwort. Vom Nutzer im Fenster bestätigt: Hinweis in der Seitenleiste orange, solange Inhalte erlaubt sind, nach Schließen und erneutem Öffnen wieder grün (Schalter aus). Leerzeichen in ausgeblendeten Werten bleiben stehen (`Authorization: xxxxxx xxxxxxxxxxxx`), die Länge war ohnehin sichtbar.
4. **Erledigt (2026-11-02, Freigabe „mach Stufe 4, Punkte 3 und 4 so umsetzen“):** Filter-Orakel gesperrt, gegenseitiger Nachweis, Zufallstests, Angriffe, Messung.
   - **Filter-Orakel gesperrt:** Die Filter der Tür sehen dieselben Werte wie die Antworten. `FilterIndex::with_values` nimmt einen Haken, der die Werte eines Pakets vor dem Filtern ändert; die Tür nutzt `privacy::hide_in_values` (Queries in `http.request.uri` und `ssdp.location` ausgeblendet, andere Geheimnisse sind nicht filterbar). `http.request.uri contains "…"` findet einen Pfad, aber keinen Teil einer Query mehr; das Fenster filtert weiter auf den vollen Werten.
   - **Gegenseitiger Nachweis** (Ablauf oben bei „Zugang im Fenster“): Der Token geht nie mehr über die Leitung. Ein Programm, das den Port eines geschlossenen oder abgestürzten Fensters übernimmt, bekommt nur das `Hello` mit einer Zufallszahl, weder Token noch Frage; `openpacketmon mcp` meldet dann `AskError::Stranger` mit eigenem Text („Nothing was sent to it …“). Die Beweise beider Seiten haben verschiedene Präfixe, das Fenster lässt sich also nicht als Orakel für den Beweis des Clients benutzen. Nonces sind 32 Zufallsbytes als Hex; ein `Hello` mit anderer Nonce wird abgelehnt. Fragen von 0.3.8 und früher (mit Token) lehnt das Fenster ab.
   - **Zufallstests:** `fuzz_masker_with_random_pieces` (`src/mcp/privacy.rs`: zufällige Kopfzeilen, URLs, Befehle und Bytes, ganz und in zufälligen Stücken: gleiches Ergebnis, gleiche Länge, nur `x` ersetzt, nie Zeilenenden) und `fuzz_tools_with_random_arguments` (`src/mcp/tools.rs`: alle Werkzeuge mit zufälligen Argumenten aller JSON-Arten über 80 zerstückelten Paketen, mit und ohne Inhalt; immer Antwort oder Fehler unter 1 MiB, ein Geheimnis in eigener Verbindung erscheint nie). Helfer aus `src/fuzz.rs`. Langer Lauf mit `FUZZ_ITERATIONS=200000` im Release-Bau ohne Befund.
   - **Angriffe als Tests** in `src/mcp/access.rs`: falsche Beweise (anderer Token, alte Nonce, der Beweis des Fensters, Token selbst, leer), altes Protokoll, Betrüger am Port (vier Arten, bekommt nur das `Hello`), Flut von 800 Verbindungen, langsamer Client (nach 5 s abgeschnitten), übergroße Frage nach dem `Hello`, Ein- und Ausschalten während dreier fragender Threads.
   - **Angriffe auf eine Testinstanz** (Release-Bau, eigener Konfigurationsordner, PCAP mit Geheimnissen, Skript mit eigenem HMAC in Python): 24 von 24 bestanden. Flut von 2000 Verbindungen aus 40 Threads, danach sofort wieder Antworten; 10 MB als `Hello` und 65 KiB als Frage ohne Antwort; der fünfte Client wird abgewiesen, nach 5,1 s ist die Tür wieder frei; 500 Fragen hintereinander (im Mittel 50 ms, das ist das Abfrageintervall der Tür); Threads 21 vorher und nachher, Speicher unverändert. Über `openpacketmon mcp`: Filter-Orakel 0 Treffer, alter Client von 0.3.8 abgewiesen, fremder Listener bekommt nur das `Hello`.
   - **Messung** (Release-Bau, 1 Million Pakete, davon 500 000 in einer TCP-Verbindung, Paketspeicher 2 Mio./2 GiB): `conversation()` hält die Sperre des Paketspeichers rund 40–50 ms (die Pakete sind `Arc<[u8]>`, kopiert werden nur Zeiger), bei 2 Millionen also rund 0,1 s. `follow_stream` brauchte 3,5 s, `packet_bytes` für ein spätes Paket 4,9 s, fast alles in `transport_payload`, das jedes Paket ganz zerlegt. Jetzt hält die Zerlegung dafür nach der Transportschicht an (`dissect::layers` ohne Anwendungsschicht, die weder `truncated` noch die Transportangaben setzt) und `masker_before` merkt sich Zustände nur für Segmente mit der gesuchten Sequenznummer: 3,0 s und 3,4 s. Der Rest steckt in den Feldern der unteren Schichten (rund 6 µs je Paket); ein eigener schneller Kopfleser würde Logik doppeln, bewusst gelassen. Die Sperre ist dabei nicht gehalten; das Fenster nutzt für „Stream folgen“ denselben Weg und wird damit ebenfalls etwas schneller. Normale Verbindungen haben wenige tausend Pakete und antworten in Millisekunden.
   - Keine neuen Crates (`sha2` war schon im Baum, nur als direkte Abhängigkeit eingetragen). Tests: 385 statt 374.
5. Später vielleicht Start und Stopp einer Aufzeichnung, nur mit Bestätigung im Fenster, nicht beim Assistenten.

Risiken, die offen bleiben:

- **Prompt Injection:** DNS-Namen, TLS-Servernamen, HTTP-Köpfe und Paketinhalte stammen von fremden Geräten und können versteckte Anweisungen enthalten.
  Kennzeichnen hilft, ist aber kein Schutz. Die Werkzeuge lesen nur, doch der Assistent hat in derselben Sitzung andere Werkzeuge; die Freigaben im
  Assistenten bleiben die wichtigste Bremse. Seit Stufe 2 stehen fremde Namen und Köpfe in den Antworten: bereinigt, gekürzt und mit `note` gekennzeichnet; `INSTRUCTIONS` in `server.rs` sagt es dem Modell. Sichtbarer Text bleibt sichtbarer Text.
- **Datenschutz:** Jede Abfrage geht an den KI-Dienst, schon Metadaten zeigen Geräte und besuchte Seiten. Steht in README und Einstellungen.
- **Ausblenden ist eine Liste bekannter Stellen, keine Garantie:** Ein Token im Pfad einer URL sieht aus wie jeder Pfad und bleibt. Das Filter-Orakel ist seit Stufe 4 gesperrt (die Filter der Tür sehen die ausgeblendeten Werte). Rohbytes lassen sich nicht filtern.
- **Paketinhalte (seit Stufe 3):** Mit dem Schalter geht der Inhalt unverschlüsselter Verbindungen an den KI-Dienst. Ausgeblendet sind nur die bekannten Stellen; ein Passwort in einem Formular (`password=` im Körper), in JSON oder in einem unbekannten Protokoll bleibt sichtbar, ebenso Geheimnisse in Binärprotokollen. Beginnt die Aufzeichnung mitten in einer Kopfzeile, gilt das erste Segment als Zeilenanfang (selten, nicht abgefangen).
- **Andere Benutzer desselben Rechners** erreichen den Port, brauchen aber den Token. Nach einem harten Beenden (auch `kill`) bleibt die Datei stehen; der Token ist dann
  wertlos, und das Fenster legt beim nächsten Einschalten einen neuen an. Übernimmt ein anderes Programm den alten Port, bekommt es seit Stufe 4 weder
  Token noch Frage und kann keine Antwort unterschieben (gegenseitiger Nachweis). Es bleibt: Ein anderer Benutzer kann die vier Plätze der Tür mit halben
  Fragen je 5 s belegen und so die Antworten aufhalten (nur lokal, nur solange der Zugang an ist, keine Daten).
- **Zwei Fenster:** Es antwortet das zuletzt eingeschaltete; schließt es, ist das ältere bis zum erneuten Einschalten nicht erreichbar. Bewusst so gelassen.
- **Windows:** Der Code nutzt keine Unix-Sockets, und `0o600` gilt nur unter Unix (`cfg(unix)`). Unter Windows schützen die Rechte des Benutzerordners die
  Datei; das beim Windows-Start prüfen (`docs/windows.md`). Aufruf dort `openpacketmon.exe mcp`.

#### 5.5 Umstellung der Oberfläche auf egui (Wunsch des Nutzers, 2026-10-31, umgesetzt 2026-11-01, Bedienung von Hand steht aus)

Der Nutzer möchte für Client-Software eine native Oberfläche statt Browser-Technik (egui, bekannt aus den ArtCraft-Programmen).
Der Probebau lief im Scratchpad (Ergebnis unten). Umgesetzt sind Mitschnitt-Helfer, Seitendaten und die egui-Seiten (Abschnitte „Stand Helfer“, „Stand Seitendaten“, „Stand egui“ unten); die Browser-Oberfläche ist entfernt.

Entschieden (Nutzer, 2026-10-31):

- **Wireshark-Muster:** Fenster schließen beendet das Programm, mit Rückfrage bei laufender Aufzeichnung oder ungesicherten Paketen.
  Kein Dauerwächter, also kein Infobereich und kein Autostart; Benachrichtigungen nur, solange das Programm läuft.
- **Erstes Release** nicht 0.3.0, sondern **0.4.0 oder 0.5.0**. Der Haltepunkt aus 4.10 gilt unverändert.

Aufbau (Plan B, vom Nutzer am 2026-10-31 bestätigt):

- `openpacketmon-capture`: kleiner Helfer, nur er hat die Rechte; öffnet Adapter, setzt den BPF-Filter, schreibt Pakete in eine Pipe,
  schreibt keine Dateien. Unter Windows läuft er ohne Sonderrechte; lädt nur er `wpcap.dll`, startet das Programm auch ohne Npcap.
- `openpacketmon`: egui-Programm ohne Rechte mit Engine, Zerlegern und Oberfläche. Die Webschicht fällt weg (Server, Startlink, CSRF,
  Host-Prüfung, systemd-Dienst). MCP braucht dann einen kleinen, abschaltbaren Zugang auf `127.0.0.1` (umgesetzt, siehe 5.4).

Befunde aus dem Code (2026-10-31):

- Der Kern (18 916 Zeilen) hängt nicht an der Weboberfläche, nur `src/cli/mod.rs` nennt `web::DEFAULT_PORT`. Ersetzt werden Templates,
  JavaScript und CSS (2964 Zeilen), der HTML-Teil von `src/web` (7205 Zeilen, 58 von 311 Tests) und `app/`.
- Der Server behält `cap_net_admin,cap_net_raw` die ganze Laufzeit und zerlegt dabei fremde Pakete; der Code gibt die Rechte nirgends ab.
  Das gilt auch ohne egui und spricht für den Helfer. Mit dem Helfer behoben (2026-11-01).
- Nahtstellen für den Helfer: `packet_stream` in `parse_packets.rs` und `traffic_preview.rs`. Mitziehen müssen Adapter-Vorschau, Pause,
  Speichern als PCAP, Statistik verworfener Pakete und Link-Typ. Die Latenzmessung braucht den Helfer nicht (`ping_group_range`).
- `Engine::lock` hält den Mitschnitt auf: Die Oberfläche arbeitet mit Abzügen, Filter über 500 000 Pakete laufen im Hintergrund.
- Crates: Fenster-Programm heute 143, eframe mit `egui_extras` und `egui_plot` 197 (OpenGL) oder 221 (wgpu).
- Prüfen ohne Headless Chrome: `egui_kittest` 0.37; `egui_mcp` nur auf ausdrücklichen Wunsch.

Aufwand (Schätzung): 8 bis 11 Arbeitstage ohne Infobereich, dazu 1 bis 2 Tage für den MCP-Zugang.

Probebau (2026-10-31, nur im Scratchpad, erfundene Daten statt echter Pakete, UHD 620, Wayland, 2560×1080 ohne Skalierung): egui 0.37 mit
`egui_extras`-Tabelle, Seitenleiste mit Verlauf, Details mit Hex-Dump, `egui_plot`, Filter im Hintergrund, Rückfrage beim Schließen.

| Messung | OpenGL | wgpu |
|---|---|---|
| Scrollen, Bilder/s (CPU je Bild) | 56 (6,2 ms) | 57 (4,6 ms) |
| Springen an zufällige Stellen, Bilder/s | 59 | 58 |
| Live, 3000 Pakete/s, CPU eines Kerns | 8,6 % | 6,1 % |
| Leerlauf, CPU eines Kerns | 0,4 % | 0,4 % |
| RAM bei 524 000 Zeilen | 111 MiB | 126 MiB |

Filter über 524 000 Zeilen: rund 0,5 s, die Oberfläche läuft dabei mit 60 Bildern/s weiter. 2 Millionen Zeilen: 54 bis 56 Bilder/s,
157 MiB, Filter 2 s, Zeilen auch am Ende sauber ausgerichtet. Die Rückfrage hält das Schließen unter Wayland an. Daraus folgt:

- **wgpu** statt OpenGL: etwas weniger CPU, unter Windows DirectX 12 statt OpenGL, 15 MiB mehr RAM und 24 Crates mehr sind vertretbar.
- **Inter** als eingebaute Schrift (Lizenz SIL OFL, in `THIRD-PARTY-LICENSES.txt` aufnehmen): Die Standardschrift von egui hat keinen `→`.
- Noch zu erledigen: „Gehe zu Paket“ muss das Listenende begrenzen (ein Sprung darüber hinaus zeigt eine Leerfläche), Spaltenlücken in der
  Zeilenfarbe füllen, Gitter des Diagramms im dunklen Design dämpfen.
- Nicht geprüft: echte Pakete und Engine, Skalierung über 100 %, Markieren und Kopieren.
- Vom Nutzer von Hand getestet (2026-10-31): alles gut, bis auf **„Mitlaufen“**: Man musste erst selbst nach unten scrollen. In der
  echten Fassung springt die Liste beim Einschalten sofort ans Ende.

Reihenfolge: Helfer (erledigt, läuft mit der Browser-Oberfläche), dann Seitendaten aus den Handlern lösen (erledigt, siehe „Stand Seitendaten“), dann die Seiten.

Stand Helfer (2026-11-01, freigegeben vom Nutzer, Test-Paket 0.3.0, kein Release: das erste Release bleibt 0.4.0 oder 0.5.0):

- Neue Crate `capture/` im Workspace: Binary `openpacketmon-capture`, dazu eine Bibliothek mit dem Format auf der Pipe (`Ready`, `Packet`,
  `Stats`, `Timeout`, `Failed`, mit Grenzen beim Lesen). Abhängigkeiten nur `pcap` und `rustix`, `unsafe` bleibt verboten.
  In `src/networking/types/capture_helper.rs` steckt die Gegenseite: `CaptureHelper` ersetzt `Capture<Active>` in `capture_context.rs`,
  `parse_packets.rs` und `traffic_preview.rs` bleiben fast unverändert. PCAP-Export über `Capture::dead(…).savefile(…)`.
- Ablauf im Helfer: Adapter muss in `Device::list()` stehen, öffnen, BPF setzen, **alle Capabilities abgeben und `no_new_privs` setzen**
  (scheitert das, bricht er ab), `Ready` senden. Befehle `pause`/`resume` über stdin; schließt sich stdin, endet er, kann also das
  Programm nicht überleben. Er sieht nicht in die Pakete und schreibt keine Dateien.
- **Befund beim Live-Test:** libpcap überlässt bei TPACKET_V3 den Timeout dem Kernel, und der liefert leere Blöcke nicht aus. Ein
  blockierendes `next_packet` kommt ohne passende Pakete also nie zurück: Mit dem Pausenfilter `less 2` wäre die Aufzeichnung nie
  wieder angelaufen, und Pakete wären im Puffer des Helfers hängen geblieben. Daher unter Linux **nicht blockierend mit eigenem `poll`**.
  Nebenfolge: Pseudoquellen wie `nfqueue` melden „nichts da“ als Fehlertext, den wertet `nothing_yet` als Leerlauf.
- Getestet: `capture/tests/live.rs` läuft in einem privaten Netz-Namensraum (dort sind keine `setcap`-Rechte nötig; wo Namensräume
  fehlen, sagt der Test das und endet ohne Prüfung): Rechte weg, BPF wirkt, Pause, Fortsetzen samt Filter, Ende beim Schließen von stdin,
  unbekannter Adapter. Von Hand im Namensraum mit dem ganzen Programm auf `lo`: Stopp ohne Zombie, nach `kill -9` des Programms kein Helfer
  mehr. Last im Release-Bau: 200 000 Datagramme in 1,2 s auf `lo` (400 000 Pakete), keines verloren, wie beim Paket 0.2.9 ohne Helfer
  (je 3 Läufe); CPU-Zeit des Programms 1,5 s statt 2 s.
- Vom Nutzer getestet (2026-11-01): `.deb` 0.3.0 installiert, Aufzeichnung auf echten Adaptern mit `setcap` ohne Fehler „auf Anhieb“. Noch nicht getestet: Betrieb über Tage.
- Bekannte Grenze: Stirbt der Helfer mitten im Mitschnitt, steht die Meldung nur auf stderr, die Aufzeichnung steht still. Die Meldung in der
  Oberfläche kommt mit der egui-Fassung. `cap_net_raw,cap_net_admin` auf dem Helfer erlauben weiter jedem lokalen Benutzer den Mitschnitt,
  wie zuvor auf `openpacketmon` (siehe `SECURITY.md`).

Stand Seitendaten (2026-11-01): Alles, was die Seiten zeigen, liegt jetzt in **`src/view/`**, ohne Webserver, HTML und Adressen.
Die Oberfläche ist unverändert, `src/web/` rendert diese Daten nur noch.

- `src/view/`: `i18n`, `format`, `filter_index` (vorher `web/packet_index.rs`), `packets`, `statistics`, `stream`, `entries`, `connections`, `overview`,
  `notifications`, `capture`, `settings`, `suggest`, dazu in `mod.rs` `Jump` und `Status`. Einstiege sind einfache Funktionen, etwa `packet_list`, `row_block`,
  `packet_detail`, `pcap_file`, `statistics`, `stream`, `entry_list`, `connection_list`, `connection_detail`, `overview`, `capture_setup`,
  `settings`; Änderungen laufen über `apply_start(StartRequest)` und `apply_settings(SettingsChange)`.
- Links sind `Jump`-Werte (`Connection`, `Connections`, `Packets`, `Statistics`, `Stream`), Zeiten bleiben `Timestamp`. `src/web/jump.rs` macht daraus Adressen,
  `format::TimeHtml` das `<time>`-Element; Templates rufen `{{ r.jump.href() }}` und `{{ ts.html()|safe }}`. Links zu einem einzelnen Paket bleiben im Web,
  weil sie den Rückweg (`back=`) tragen; die Daten liefern dort nur die Nummer.
- Im Web bleiben Handler, Formulare, Live-Protokoll (`api.rs`), Pager, Anmeldung, Fenster-Programm und `sound`/`unit`-Codes der Formulare. `Lang::js_texts` steht noch in
  `view/i18n.rs` und fällt mit der Browser-Oberfläche weg.
- Der Test `the_view_knows_nothing_of_the_browser_pages` hält die Grenze: In `src/view/` dürfen weder `axum`, `askama`, `crate::web` noch Links vorkommen.
- Geprüft: 631 Seiten je Englisch und Deutsch (Aufzeichnung aus einer PCAP mit 177 Paketen, Pakete, Details, Stream, Verbindungen, Statistik, Listen, Einstellungen,
  JSON-Antworten) vor und nach dem Umbau Byte für Byte gleich, nach jedem Schritt. Normalisiert wurden nur CSRF-Token, temporäre Pfade, die Reihenfolge von JSON-Schlüsseln und
  gleich sortierte Zeilen bei `/connections?sort=`. POST-Wege decken die Tests im Web ab; ein Live-Mitschnitt lief für diesen Umbau nicht, er berührt die Aufzeichnung nicht.
- Vor dem Release zu beheben: Der Test `test_increments_ping_sequence` (`src/networking/types/latency.rs`) schlägt selten fehl, weil parallele Tests denselben Zähler `PING_SEQUENCE` erhöhen; beim Umbau einmal aufgetreten, beim nächsten Lauf nicht mehr.
- Eine gewollte Abweichung: Eine Benachrichtigung ohne Filter (Host ohne Name, AS und Land) führt nach `/connections` statt `/connections?f=`, die Seite ist dieselbe.
- Auf dem Weg ohne Änderung der Ausgabe zusammengelegt: doppeltes `sound_code` (`api.rs`, `settings.rs`), `endpoint` in `stream.rs` (jetzt `format::socket`), Seitengrößen als Konstante.

Stand egui (2026-11-01, Stufen 1 bis 4 freigegeben vom Nutzer, Test-Pakete **0.3.5** bis **0.3.7**, kein Release): Alle Seiten liegen in **`src/gui/`**, `openpacketmon` öffnet das Fenster. Stufen 1 bis 3 stehen im Commit „Native egui window next to the browser pages“, der Abbau (Stufe 4) im Commit danach; wer die Browser-Oberfläche nachsehen will, findet sie dort.

- Aufbau: `src/gui/mod.rs` (Fenster, Seitenleiste, Verlauf, Rückfrage beim Schließen), `theme.rs` (Farben wie `static/app.css`, Inter), `widgets.rs`, `load.rs` (liest im Hintergrund, die Oberfläche wartet nie auf die Engine), `chart.rs`, `filter_field.rs`, `sound.rs`, `icons.rs`, `pages/` je Seite. Die Seiten rufen nur `src/view/`.
- Paketliste: eine lange Liste, gelesen wird blockweise nur der sichtbare Teil; Mitlaufen endet beim Hochscrollen und kommt am Ende wieder; Strg+G springt zu einem Paket; Farblegende in der Filterhilfe.
- Speichergrenze (Stufe 3): `PacketStoreConf` hat das neue Feld `megabytes` (256 MiB bis 2 GiB), die Paketzahl geht bis 2 Millionen. 
- Fund beim Bau, behoben: Starb der Mitschnitt-Helfer mitten in der Aufzeichnung, stand das nur auf stderr und die Oberfläche zeigte weiter „läuft“. Jetzt meldet `parse_packets` das einmal als `BackendTrafficMessage::CaptureStopped`, `capture_problem` zeigt den Text `capture-stopped`.
- Neue Abhängigkeiten: `eframe`, `egui_extras`, `egui_plot`, `image`, `rfd` (Dateidialog), `rodio` (Ton); zum Bauen zusätzlich `libasound2-dev` (CI, README, `CONTRIBUTING.md` ergänzt). Inter liegt in `resources/fonts/`, die Lizenz steht in `THIRD-PARTY-LICENSES.txt`. `deny.toml`: drei Ausnahmen für Sicherheitsmeldungen (RUSTSEC-2026-0370/0371/0372) und für die Schriftlizenzen von `epaint_default_fonts`; **vor dem Release neu bewerten**.
- Tastatur der Paketliste wie in der Browser-Fassung: Pfeile, Bild auf/ab, Strg+Pos1/Ende, Strg+G.
- Stufe 4 (2026-11-01): `src/web/`, `app/`, `templates/`, `static/` (nur `logo-mark.png` zog nach `resources/icons/`), `openpacketmon.service`, `locales/*/js.ftl` entfernt; die im Fenster gebrauchten Texte stehen in `app.ftl`, die übrigen sind weg. Der Test `every_message_is_used` hält das sauber. Optionen `--port`, `--no-browser`, `--gui` entfallen. Crates raus: axum, askama, hyper, hyper-util, tower-http, subtle, wry, tao und die Testhilfen dazu, dazu die schon vorher ungenutzten `ctrlc` und `unic-langid`. Dadurch fielen auch drei Ausnahmen in `deny.toml` weg (`basic-toml`, `proc-macro-error`, `glib`), und die Lizenzen BSL-1.0 und MPL-2.0 werden nicht mehr gebraucht. Die 37 Tests aus `src/web/` prüften meist Anmeldung, CSRF und Adressen; was Verhalten prüfte, steht jetzt in `src/view/` und `src/gui/tests.rs` (Einstellungen übernehmen, Start-Anfrage, fehlende Datei/Adapter, falsche Filter, Lizenzen, Tastatur, zurück aus den Details).
- `.deb`: Das Fenster lädt die Bibliotheken für Anzeige und Grafik erst zur Laufzeit, `$auto` sieht sie nicht; deshalb stehen `libxkbcommon0`, `libwayland-client0`, `libx11-6`, `libxcursor1`, `libxi6`, `libxkbcommon-x11-0` und `libvulkan1 | libegl1` von Hand in `depends`, `xdg-desktop-portal` (Dateidialoge) in `recommends`. Der Eintrag im Startmenü ruft `openpacketmon` auf.
- Geprüft: `cargo test --workspace` (307 + Helfer und Unterpakete), `cargo clippy -- -D warnings`, `cargo fmt`, `cargo deny --workspace check`, `dev/check-notices.sh`, Lizenzliste. Bilder aller Seiten: `GUI_SHOTS=<ordner> cargo test --bin openpacketmon gui::tests::shots -- --include-ignored --test-threads=1`. Das Programm öffnete zweimal 8 Sekunden ein Fenster ohne Fehler; `--help`, `--version`, `--list-adapters` gehen, `--port` wird abgelehnt.
- **Weißes Aufblitzen der Paketliste behoben** (2026-11-01, aus dem Test von 0.3.5, seit 0.3.6): Beim Folgen schiebt egui die Liste erst nach dem Zeichnen ans Ende (`stick_to_bottom`). Das erste Bild nach einem neuen Block zeigte also die alte Position; kamen zwischen zwei Abfragen mehr Pakete, als Zeilen sichtbar sind, lagen deren Zeilen nicht mehr im Block und blieben leer, im hellen Design weiß, bis zum nächsten Neuzeichnen. Jetzt springt `receive` im selben Bild ans Ende. Tests `following_shows_a_new_block_in_the_same_frame` (schlug vorher fehl) und `a_new_block_leaves_a_list_that_does_not_follow_in_place`.
- **Filterleisten enden am Seitenrand** (2026-11-01, aus dem Test von 0.3.5, in 0.3.6): Pakete, Statistik und Verbindungen rechneten die Feldbreite mit festen Schätzwerten ohne die Abstände, „Zurücksetzen“ ragte bis zu 19 Punkte in den Rand. Jetzt werden die Knöpfe gemessen (`widgets::button_width`, `widgets::field_width`, `filter_field::button_widths`). Alle Knopfarten haben dafür einen Rand in eigener Farbe (`Primary`, `Danger`), weil egui ihn zur Größe rechnet; „Anwenden“ wird 2 Punkte breiter. Die Sortier-Auswahl hat feste 190 Punkte, damit ihr längster Text passt und die Leiste nicht springt. Test `the_rows_of_the_filters_end_at_the_edge_of_the_page` (beide Sprachen, alle Seiten, alle Sortierungen).
- **Zähler „Benachrichtigungen“ wieder rechteckig** (2026-11-01, aus dem Test von 0.3.6, seit 0.3.7): Beim Umbau auf egui war Phase 3 Punkt 1 verloren gegangen und der Zähler wieder rund geworden. Jetzt wieder 5 px Rundung, Mindestbreite 22, ab 100 „99+“ mit der genauen Zahl im Tooltip; der Text der Zeile wird gekürzt, wenn der Zähler ihn sonst überdecken würde, „Benachrichtigungen“ also nur bei „99+“ (`nav_item` in `src/gui/mod.rs`).
- **Aus dem Test von 0.3.6 umgesetzt** (2026-11-01, Freigabe „kannst jetzt alle änderungen umsetzen“, seit 0.3.7):
  1. **Kopieren in den Paketdetails** (`src/gui/pages/packet.rs`, Texte in `src/view/packets.rs`). Feldzeilen sind Knöpfe und der Hex-Bereich ein gezeichnetes Bild, deshalb wirkten weder Strg+C noch Rechtsklick. Strg+C kopiert die markierte Zeile als `Name: Wert`, bei einer markierten Schicht die Schicht mit allen Feldern, eingerückt wie im Baum. Rechtsklick auf eine Feldzeile: „Zeile kopieren“, „Wert kopieren“, „Bytes als Hex kopieren“. Auf eine Schicht: „Schicht kopieren“, „Bytes als Hex kopieren“. Im Hex-Bereich: „Auswahl als Hex kopieren“, „Auswahl als Text kopieren“ (nicht druckbare Zeichen als Punkt), „Ganzes Paket als Hex-Dump kopieren“. Überall „Alles kopieren“. Der Rechtsklick markiert die Zeile zugleich. Steuerzeichen und Zeilenumbrüche aus Paketinhalten werden beim Kopieren zu Punkten, sonst könnten sie beim Einfügen in ein Terminal wirken (`clean`). Kein freies Markieren einzelner Bytes.
  2. **Hinweistext `capture-packets-hint`**: statt „höchstens 512 MiB“ jetzt „Grenze in den Einstellungen, bis 2 GiB“.
  3. **Ladeanzeige** (`widgets::loading`, erst nach 150 ms, damit schnelle Antworten nicht flackern). Paketliste: Sie zeigte während des Ladens nichts und danach fälschlich „Noch keine Pakete.“ oder „Kein Paket passt zum Filter.“; jetzt merkt sich `State`, ob für die Filter schon ein Block kam (`received`), die Leer-Meldungen kommen erst dann. Statistik und Stream folgen zeigen dieselbe Anzeige. `Loader::running_for` nennt die Dauer eines laufenden Auftrags.
  4. **Kartenkopf „Benachrichtigungen“**: Die Notiz stand links oben neben dem Titel und zu klein, jetzt rechts und mittig (`widgets::card_note`, 13 Punkte). „Alle löschen“ setzt nun auch die Gesamtzahl zurück (`logged_notification.rs`), vorher stand dort „13 Benachrichtigungen“ mit „die neuesten 0“.
- **Letzte Rückmeldungen aus dem Test umgesetzt** (2026-11-01, Test-Paket 0.3.7; Nutzertest laut Nutzer ohne weitere Funde):
  1. **Top-Listen der Übersicht:** Klicks auf Top-Hosts und Top-Dienste taten nichts, weil jede Karte in `top_card` (`src/gui/pages/overview.rs`) `*action = …` schrieb, auch `None`, und so die Klicks der Karten davor löschte. Das traf auch den Favoriten-Stern. Jetzt setzt nur eine Karte mit Klick die Aktion. Test `every_top_list_of_the_overview_opens_its_entries` (schlug vorher fehl).
  2. **Griff des Lautstärkereglers:** egui leitet ihn aus der Höhe des Reglers ab, und `interact_size.y = 32` machte ihn fast 13 Punkte im Radius. In dieser Zeile gilt jetzt die Texthöhe als `interact_size.y` (`src/gui/pages/settings.rs`), kleiner als die Texthöhe rutscht der Regler aus der Mitte. Der Wert steht als Text daneben (`show_value(false)`), eintippen lässt er sich nicht mehr.
- **CI wieder ohne Grafikkarte lauffähig** (2026-11-01, kein neues Paket, nur Testcode): Seit `c30c68b` schlugen 9 Oberflächen-Tests in der CI mit `No adapter found` fehl, weil `window_of_size` (`src/gui/tests.rs`) jedes Testfenster mit dem wgpu-Renderer baute. Das Programm war nicht betroffen. Den Renderer braucht nur `shots`, er wird jetzt nur bei gesetztem `GUI_SHOTS` eingeschaltet.
- **Aus dem Test von 0.3.8 umgesetzt** (2026-11-02, Freigabe „führe die korrekturen durch“, Test-Paket **0.3.9**):
  - **Verkehrsbalken der Top-Listen** (Übersicht, Hosts, Dienste, Programme): Am Übergang Blau → Grün war eine Kerbe, die wie eine Lücke aussah, weil `widgets::share_bar` beide Teile einzeln mit runden Ecken zeichnete; kleine Teile wurden zu Punkten. Jetzt eine Form: beide Teile zusammen in der zweiten Farbe mit runden Enden, der erste darüber nur links rund. An der Naht scheint kein Hintergrund durch. Im Bildtest vergrößert geprüft, hell und dunkel.
  - **Schrift der Knöpfe auf gleicher Höhe:** „Beenden“ und „Pakete dieser Verbindung“ im Seitenkopf hatten ihre Schrift 2 px höher als die Knopfgruppe „Bytes · Bits · Pakete“ daneben (gemessen im Bildtest: Großbuchstaben bei y 27 statt 29, Kästen gleich, 32 px). Ursache: egui 0.37 setzt die Schrift eines Buttons dorthin, wohin das Layout außen herum ausrichtet (`ui.layout().align2()`): im Seitenkopf (`right_to_left(Align::Min)`) oben, in der Zeile der Gruppe (`ui.horizontal`, mittig) in die Mitte; der Innenraum ist höher als die Schrift. Am Button lässt sich das nicht einstellen, und ein passender Innenabstand ließ 1 px Rest (ganze Pixel). Jetzt zeichnet `widgets::add_button` die Schrift selbst in die Mitte des Kastens; der egui-Button behält Größe, Platz, Bedienung und Beschriftung (für Tests und Screenreader) und bekommt eine durchsichtige Schrift. Gilt für alle Knöpfe aus `button`, `small_button` und `segmented`. Gemessen in Übersicht, Stream (Seiten- und Kartenkopf) und Aufzeichnung: überall 11 px von der Oberkante bis zu den Großbuchstaben.
- **Adapter ohne IP-Verkehr ausgegraut** (aus dem Test 2026-11-01, umgesetzt 2026-11-02 auf Ansage, Test-Paket 0.3.10): Adapter wie `bluetooth0`, `bluetooth-monitor`, `nflog`, `nfqueue` zeigten in der Vorschau „0“ und eine leere Kurve, obwohl die Vorschau sie wegen des nicht unterstützten Link-Typs gar nicht öffnet; ein Start endete in `parse_packets` ohne Meldung. Bluetooth selbst bleibt laut 5.1 außerhalb des Umfangs. Jetzt meldet die Vorschau solche Adapter (`TrafficPreview::no_ip`, `DevicePreview::no_ip`), die Seite Aufzeichnung zeigt sie grau, nicht wählbar und am Ende der Liste, mit „Kein IP-Verkehr, wird nicht ausgewertet“ statt Kurve; sie werden nicht ausgeblendet. Eine zuvor gespeicherte Wahl eines solchen Adapters wird nicht übernommen. Ein Start auf so einer Quelle (auch Datei oder `--adapter`) bricht mit `CaptureError::NoIp` und Begründung samt Link-Typ ab, statt still nichts zu zeigen. Geprüft mit Tests (Vorschau, Ablehnung beim Start, leere PCAP mit Link-Typ 201), Bildtest hell und dunkel und `--adapter bluetooth0`/`nflog` über den installierten Mitschnitt-Helfer.
- **Von Hand geprüft (Nutzer, 2026-11-02, mit 0.3.10):** Bedienung über längere Zeit, Live-Mitschnitt (auch mit BPF-Filter), Ton, Dateidialoge, Skalierung des Bildschirms und Geo-Download funktionieren ohne Probleme; die Adapter ohne IP-Verkehr sind ausgegraut und lassen sich nicht wählen. Offen bleibt nur das `.deb` mit Fenster auf anderen Systemen (4.10 Schritt 4). Die alte Datei `~/.config/autostart/openpacketmon-app.desktop` war am 2026-11-02 schon nicht mehr da.
- Vor dem Release (erledigt 2026-11-02, Ergebnis in 4.10): `deny.toml`-Ausnahmen neu bewerten (RUSTSEC-2026-0370/0371/0372, Schriftlizenzen); `test_increments_ping_sequence` beheben; Sicherheitsdurchsicht der geänderten Teile: Webangriffe (4.8) fallen weg, neu ist, dass Paketinhalte als Text im Fenster stehen (fehlende Zeichen zeigt das Programm als Kästchen, `MissingGlyphPolicy::Tofu`). MCP (5.4) gehört in diese Durchsicht, es wird mit dem ersten Release veröffentlicht.

Entschieden (Nutzer, 2026-10-31): Der Paketspeicher bleibt im RAM, die Grenze wird wählbar (bis etwa 2 GiB mit passender Paketzahl).
Eine temporäre Datei wie bei Wireshark kommt nur, falls das später nicht reicht, weil sonst unverschlüsselter Verkehr auf der SSD läge.

#### 5.6 Neue Oberfläche im Stil von opensecmon (Wunsch des Nutzers, 2026-11-09; Entwurf abgenommen, Ergänzung zu den Updates abgenommen; umgesetzt 2026-11-10, Test-Paket 0.5.0)

Auftrag: klickbarer Entwurf wie bei opennetmon (`../opennetmon/docs/entwurf/`, dort lokal, am 2026-11-09 abgenommen), Farben und Gestaltung
von dort, Logo im selben Stil. Ergebnis in **`docs/entwurf/`** (statisches HTML, kein Anwendungscode; Start `index.html`, Einzelheiten
und offene Punkte in `docs/entwurf/README.md`). Inhalt und Funktionen wie 0.4.0, alle Daten erfunden (Dokumentationsbereiche).

- **Aufbau:** Kopfleiste statt violetter Seitenleiste (Übersicht, Verbindungen, Pakete, Statistik, Hosts, Dienste, Programme; Glocke,
  Thema, Hilfe, Zahnrad), Aufzeichnung immer rechts oben (Zustand, Adapter, Dauer, Pausieren, Beenden), Statusleiste unten (Pakete,
  Paketspeicher, verworfen, KI-Hinweis, neue Version, Links; „IP Geolocation by DB-IP“ bei jeder Breite sichtbar).
- **Gestaltung:** graue Flächen, Akzent `#1c5ea8`, Fira Sans/Fira Code (Ligaturen aus; in egui seit 2026-11-10 ebenfalls, siehe 5.7
  Stufe 3), dichte Zeilen (Tabellen 34 px, Paketliste 26 px),
  leiser getönte Farben der Paketliste für hell und dunkel, Symbole nach Lucide (ISC).
- **Seiten:** Aufzeichnung, Übersicht, Verbindungen, Verbindung, Pakete (virtuelle Liste mit 48 000 Zeilen, Live-Folgen, Anzeigefilter mit
  Prüfung und Vorschlägen, Strg+G, Leiste für das gewählte Paket), Paketdetails (Felder ↔ Bytes), Stream folgen, Statistik, Hosts, Dienste,
  Programme, Benachrichtigungen, Einstellungen mit Unternavigation, Logo (fünf Entwürfe, gewählt C „Lupe über Paketen“), Breiten
  (1440, 1280, 1024, 760 px).
- **Neu** (vom Nutzer alle übernommen): „Auffällig“ auf der Übersicht, Tastenkürzel-Hilfe „?“, „/“ ins Filterfeld, Leiste für das
  gewählte Paket, Rückfrage beim Beenden der Aufzeichnung.
- **Geprüft** (2026-11-09): alle Seiten ohne Skriptfehler; Bytes jeder Paketschicht ergeben die Paketlänge; Bildschirmfotos aus Headless
  Chrome bei 1280 × 820 hell, dazu dunkel, 1200, 1024 und 760 px und die Zustände „pausiert“/„keine“ sowie Paketinhalte für KI;
  von Hand im Browser: Filter (Vorschläge, Fehlermeldung mit Stelle, `?f=`), Verbindungsfilter, „Gehe zu“, ↑/↓/Pos1/Ende, Hilfe und
  Rückfrage beim Beenden, Thema, Felder ↔ Bytes, MCP-Schalter.
- **Abgenommen (Nutzer, 2026-11-09):** Farben und Aufbau passen; Logo **C „Lupe über Paketen“** (Such- und Filterfelder bekommen deshalb kein
  Lupensymbol); alle neuen Teile bleiben; maßgeblich ist **1280 × 820** (Desktop-Programm, heute schon die Standardgröße), kleinere Größen
  werden nicht eigens gestaltet. **Mindestgröße bleibt 760 × 480** wie heute (`src/gui/mod.rs`, `with_min_inner_size`; Vorschlag vom
  Nutzer bestätigt, 2026-11-09), nicht 1280 × 820, sonst passt das Fenster auf 1366 × 768 nicht mehr ganz. Zwischen 760 und 1280 px gelten
  nur die Regeln aus dem Entwurf (unter 1180 px „Mehr“, unter 900 px nur Symbole, Statusleiste kürzt in fester Reihenfolge, DB-IP bleibt;
  seit 2026-11-10 leicht verschobene Grenzen, siehe 5.7).
- **Vorerst nur dokumentiert und geplant** (Nutzer, 2026-11-09): keine Umsetzung; Entwurf und Plan auf Wunsch des Nutzers committet
  (2026-11-09).
- **Ergänzung zu den Updates** (vom Nutzer angekündigt, am 2026-11-09 eingebaut, **vom Nutzer abgenommen am 2026-11-09**): der Hinweis auf neue Versionen nach
  der Absprache im Entwurf des geplanten Launchers („Aufruf aus den Apps“, vereinbart 2026-11-09; Pfad in `CLAUDE.local.md`), wie bei opennetmon
  (dort Vorhaben P). openpacketmon sucht weiter selbst auf GitHub und lädt oder installiert nichts.
  - Statusleiste: „Version x.y verfügbar“ öffnet ein kleines Fenster („openpacketmon x.y ist erschienen“, installierte Version, Datum).
    **Mit Launcher** (nur Linux, erkannt an seiner Programmdatei): „Aktualisieren“ startet den Launcher mit der Seite von openpacketmon (läuft er
    schon, zeigt sein Fenster sie); er installiert, sobald openpacketmon geschlossen ist, und wartet darauf. Bei laufender Aufzeichnung der
    Hinweis, vorher als PCAP zu sichern. **Ohne Launcher und unter Windows:** „Release-Seite öffnen“. Immer dazu „Was ist neu?“.
  - Einstellungen → Aktualisierungen: Installiert, Neueste Version (Datum oder „aktuell“), Zuletzt gesucht, derselbe Knopf, ohne Launcher kurz
    die Schritte von Hand (`.deb` mit `SHA256SUMS` prüfen und mit apt installieren; Windows-Installer), darunter die **Release-Notes** von GitHub
    (englisch wie dort; ohne neue Version die der installierten Version). Heute gibt es dort nur einen Link zur Release-Seite.
  - Im Entwurf über die Entwurfsleiste „Update: mit Launcher / ohne Launcher / keins“; geprüft (2026-11-09) ohne Skriptfehler auf allen Seiten,
    Bildschirmfotos hell/dunkel bei 1280 × 820 und 760 × 480, Esc und Klick daneben schließen das Fenster, „Was ist neu?“ springt zu den Notes.
  - Für die Umsetzung: `check_updates.rs` liest bisher nur den Release-Titel (`name`); dazu `body` (Markdown) und `published_at` aus derselben
    Antwort. Die Release-Notes sind Fremdtext (nur `https://`-Links, kein HTML, Länge begrenzen) und brauchen einen kleinen Markdown-Renderer
    (eigener Code oder Crate mit passender Lizenz). Der Launcher wird ohne Shell gestartet. Name und Aufruf des Launchers erst ins Repo, wenn er
    öffentlich ist.
- **Umsetzung** in vier Stufen (Vorschlag in `docs/entwurf/README.md`):
  A Farben/Schriften (`src/gui/theme.rs`, Fira als TTF, Lizenzliste), B Kopf- und Statusleiste mit dem Hinweis auf neue Versionen
  (`src/gui/mod.rs`, `widgets.rs`, `locales/`; Fenster bleibt 1280 × 820 mit Mindestgröße 760 × 480), C Seite für Seite (`src/gui/pages/`,
  Seitendaten bleiben in `src/view/`; darin Einstellungen → Aktualisierungen mit Release-Notes), D Symbole, Logo C, Programmsymbole, README-Bild.
- **Umgesetzt (2026-11-10, „go“ des Nutzers: „orientiere dich penibel an den entwurf“; im Test-Paket 0.5.0):**
  - Stufen A bis D. Jede Seite mit Bildern aus `src/gui/tests.rs` (`GUI_SHOTS=… cargo test --bin openpacketmon gui::tests::shots -- --ignored`)
    gegen Bildschirmfotos des Entwurfs aus Headless Chrome bei 1280 × 820 verglichen, meist auf 1–3 px genau; Einstellungen auch bei 960
    und 760 px, Aufzeichnung auch mit Datei und IPFIX und bei 1150 px Höhe.
  - Einstellungen mit Unternavigation (Anzeige, Benachrichtigungen, Paketspeicher, Geodaten, KI-Assistenten, Aktualisierungen, Über);
    Thema und KI-Zugang wirken sofort (ohne „Speichern“), die Lizenzen stehen unter „Über“. Release-Notes mit eigenem kleinem
    Markdown-Leser (`src/view/release_notes.rs`: Überschriften, Absätze, Listen, fett, Code, nur `https://`-Links, höchstens 400 Blöcke).
  - Aufzeichnung: Quelle mit Symbolen in der Kopfzeile, Adaptertabelle mit Kurven der letzten 30 Sekunden (gemeinsamer Maßstab),
    Filter und Paketspeicher nebeneinander, PCAP-Datei; die laufende Aufzeichnung mit Zustand, Dauer, Fakten und Fuß wie im Entwurf.
  - Programmsymbol: Zeichen C weiß auf `#1c5ea8` (Ecken 22 %, Zeichen 75 %) in 8–1024 px, erzeugt mit `dev/make-icons.py` (Pillow),
    `.ico` mit `dev/make-ico.py`; README-Bild hell und dunkel (`docs/assets/openpacketmon-logo{,-dark}.png`), das alte Logo nur noch in der
    Geschichte. Ungenutzte Symbole entfernt (`phone`, `phone-xl`, `lock` bleiben für 5.7); die Symbole nach Lucide (ISC, teils MIT von
    Feather) stehen in `THIRD-PARTY-LICENSES.txt`.
  - Aufgeräumt: alte Bausteine der violetten Oberfläche, ungenutzte Texte, die Einstellung `plain_packets` (alte Konfigurationen laden
    weiter), die Crate `egui_plot`; `cargo clippy -- -D warnings` ohne Meldung. `rmcp` auf 3.9.0 (RUSTSEC-2026-0462 betrifft den
    HTTP-Client von rmcp, den openpacketmon nicht nutzt), `async-executor` 1.14.1 (1.14.0 zurückgezogen).
  - **Abweichungen vom Entwurf** (bewusst): Zeitentabelle der Statistik ohne Links; Dienste per Name; Hosts mit „+n“ weiteren Adressen;
    Benachrichtigungen höchstens 200; Sprache „Wie das System“ bleibt wählbar; der Hinweis auf neue Versionen nur ohne Launcher (der
    geplante Launcher ist nicht öffentlich, seine Erkennung kommt mit ihm); „Fremdlizenzen“ zeigt NOTICE und THIRD-PARTY-LICENSES.txt ganz;
    „Gespeichert“ als kurze Meldung; „Höchstens Pakete“ als freie Zahl von 1.000 bis 2.000.000; auf der Seite Aufzeichnung Filter und
    Paketspeicher nur für Adapter und Datei, PCAP-Datei nur für Adapter (der Entwurf zeigt sie bei jeder Quelle, sie wirken dort aber
    nicht); die Adapter bleiben nach Verkehr sortiert. Texte zu Anrufen (Paketspeicher, KI-Assistenten, Einstellungen → Anrufe) kommen mit 5.7.

#### 5.7 VoIP: SIP, RTP und Anrufe (Wunsch des Nutzers, 2026-11-09; Entwurf abgenommen 2026-11-10, Umsetzung nach „go“)

Auftrag: was Wireshark dafür bietet, mit eigenem Code: Anzeigefilter `sip` und `rtp`, eine Liste der erkannten Telefonate (dort
„Telephony → VoIP Calls“) und Anhören unverschlüsselter Gespräche. **Wireshark ist GPL:** nichts daraus übernehmen; Grundlage sind die
Standards (SIP RFC 3261, SDP RFC 8866, RTP/RTCP RFC 3550, Payload-Typen RFC 3551, G.711 nach ITU-T).

- **Entscheidungen (Nutzer, 2026-11-09):**
  - Anhören: ja, für **G.711** A-law und µ-law (PCMA/PCMU), beide Richtungen, dazu „Als WAV speichern“. G.722 und Opus später.
  - Vor dem ersten Anhören oder Speichern einmal ein Hinweis auf **§ 201 StGB** (Vertraulichkeit des Wortes); bestätigt wird je Benutzer,
    wieder anzeigen lässt er sich unter Einstellungen → Anrufe.
  - Reihenfolge: **Zerleger und Filter zuerst** (unabhängig von 5.6, in der heutigen Oberfläche), die Seiten „Anrufe“ und „Anruf“ danach mit
    der neuen Oberfläche (nach 5.6 Stufe B).
  - Entwurf: beide Seiten in `docs/entwurf/` mit erfundenen Anrufen, `sip`/`rtp` im Anzeigefilter des Entwurfs.
  - **Platz (2026-11-10):** „Anrufe“ steht **nicht in der Kopfleiste**, weil Telefonie seltener gebraucht wird (Variante D von vier
    verglichenen: A kompakt mit acht Reitern, B Hosts/Dienste/Programme nur als Symbol, C diese drei als ein Reiter, D Anrufe unter Statistik).
    Statistik bekommt die Unterreiter **„Protokolle“** (der bisherige Inhalt) und **„Anrufe“** (mit Zahl der Anrufe). Weitere Wege: Knopf
    „Anruf“ in der Leiste über der Paketliste, „Gehört zum Anruf …“ in den Paketdetails, „Auffällig“ in der Übersicht.
- **Im Entwurf** (2026-11-09; Einzelheiten und „Zum Ausprobieren“ in `docs/entwurf/README.md`):
  - Paketliste: SIP, SDP, RTP und RTCP mit eigener Zeilenfarbe (`--row-voip`), Info wie „Request: INVITE sip:…“, „Status: 200 OK (INVITE)“,
    „PT=ITU-T G.711 PCMA, SSRC=…, Seq=…“. Paketdetails mit den Schichten SIP (alle Kopfzeilen, Antworten mit Paket und Zeit), SDP, RTP
    („Ausgehandelt per SDP in Paket …“) und RTCP (Sender Report, Source Description); oben der Link „Gehört zum Anruf …“.
  - Filterfelder: `sip`, `sdp`, `rtp`, `rtcp`, `sip.method`, `sip.status_code`, `sip.call_id`, `sip.from`, `sip.to`, `rtp.ssrc`, `rtp.seq`,
    `rtp.p_type` und `voip.call_id` (alle Pakete eines Anrufs: SIP und die darin ausgehandelten RTP-/RTCP-Ströme). Statistik: SIP, SDP, RTP
    und RTCP in der Protokollhierarchie.
  - Reiter **Anrufe** (Statistik): Kacheln; Liste mit Zustand (läuft, beendet, verpasst, abgelehnt, nur RTP), Beginn, Dauer,
    Richtung, Von, An, Codec, Qualität, Pakete, Anhören; die Anmeldung (REGISTER) als Fußzeile; Hilfe „So kommen Anrufe in die Aufzeichnung“
    (Softphone auf dem Rechner; Telefon am Router: Mitschnitt der FRITZ!Box oder Port-Spiegelung; SIP über TLS und SRTP bleiben unlesbar).
  - Seite **Anruf** (Statistik › Anrufe › Anruf): Fakten (Von, An, Richtung, Zeiten, Codec mit SDP-Paket, Programm, Anbieter, Call-ID, Anmeldung per Digest); **Anhören**
    (Wellenform je Richtung, verlorene Pakete rot und als Stille, Beide/Gesendet/Empfangen, Klick springt); **Ablauf** als Leiterdiagramm
    (SIP-Nachrichten, Sprache als Band); **Sprachströme** mit Paketen, Verlust, Jitter (RFC 3550), größtem Abstand, Folgefehlern, Bewertung
    und den RTCP-Berichten. Bewertung: gut unter 1 % Verlust und unter 30 ms Jitter, schlecht ab 3 % oder 50 ms, dazwischen mäßig; bei
    „schlecht“ ein Hinweis auf der Seite und unter „Auffällig“ in der Übersicht.
  - Einstellungen → Anrufe: „RTP auch ohne SIP erkennen“ (an der Form: Version 2, gleichbleibende SSRC, fortlaufende Nummern über mindestens
    10 Pakete; **standardmäßig an**, Nutzer 2026-11-10), „SIP auch auf anderen Ports als 5060 erkennen“ (an der ersten Zeile; im Entwurf an);
    Stand des Hinweises zu § 201.
  - Beispiele: ausgehend mit 407 und „486 Busy Here“, ausgehend angenommen (PCMA, 1:31), eingehend mit schlechter Verbindung (PCMU, 4,3 %
    Verlust, 41,7 ms Jitter), eingehend mit unterdrückter Nummer verpasst (CANCEL, 487), ausgehend laufend, ein RTP-Strom ohne SIP (Typ 111,
    vermutlich SRTP) und die Anmeldung mit 401/200. Der Entwurf rechnet mit 60 ms je Paket und spielt keinen Ton.
  - **Kopfleiste:** bleibt wie abgenommen (sieben Reiter, „Aufzeichnung läuft“). Beim Messen fielen Überläufe der abgenommenen Fassung auf:
    900–906 px (6 px), 1260–1264 px (5 px), 1360–1439 px (bis 32 px), ohne Aufzeichnung auch 920–936 px (16 px). Behoben (**abgenommen**, Nutzer 2026-11-10):
    Programmname neben dem Logo erst ab 1440 px (bisher ab 1360), „Aufzeichnung läuft“ erst ab 1280 px (bisher ab 1260), Reiter nur als
    Symbol unter 960 px (bisher unter 900). Gemessen von 760 bis 1920 px, laufend, pausiert und ohne Aufzeichnung: kein Überlauf, mindestens
    15 px Luft.
  - Geprüft (2026-11-09, nach dem Umzug unter Statistik erneut 2026-11-10): alle Seiten ohne Skriptfehler; Bildschirmfotos bei 1280 × 820
    hell und dunkel und bei 760 × 480 (Liste, Anrufe A–F, Pakete mit `sip or rtp` samt Knopf „Anruf“, SIP- und RTCP-Paket, Statistik mit
    Unterreitern, Einstellungen, leere Liste, ohne Aufzeichnung, Hinweis zu § 201, Abspielen); die Anrufliste passt ohne Querscrollen von 760
    bis 1920 px.
- **Umsetzung** (Vorschlag, je Stufe nach „go“ mit Test-Paket):
  1. **Zerleger** `src/dissect/sip.rs` (SIP über UDP und TCP, Port 5060 und an der ersten Zeile; SDP im Körper) und `rtp.rs` (RTP; RTCP SR,
     RR, SDES, BYE). RTP läuft auf ausgehandelten, wechselnden Ports: Die Zerlegung bekommt dafür eine kleine Tabelle der im SDP
     ausgehandelten Endpunkte (je Aufzeichnung, in `src/engine/`), sonst greift die Heuristik. SIP über TLS (5061) und SRTP bleiben
     „verschlüsselt“.
  2. **Filter und Statistik:** die Felder oben in `src/filter/fields.rs`, Zeilenfarbe und Info in `src/view/packets.rs`, Protokollhierarchie
     in `src/view/statistics.rs`. Tests mit selbst erzeugten Paketen (Sinuston als G.711); keine echten Mitschnitte und keine echten
     Rufnummern im Repo.
  3. **Anrufe** (nach 5.6 Stufe B): Seitendaten `src/view/calls.rs` (Anrufe aus den SIP-Dialogen nach Call-ID, Ströme nach SSRC, Verlust und
     Jitter nach RFC 3550), Seiten in `src/gui/pages/` (Statistik mit Unterreitern), Texte in `locales/`.
  4. **Anhören:** G.711 dekodieren (eigener Code nach der ITU-T-Norm), Wiedergabe über `rodio` (schon für die Töne im Programm), WAV schreiben
     (8 kHz, 16 bit, Stereo: links gesendet, rechts empfangen; eigener kleiner Schreiber oder eine Crate mit passender Lizenz), Lücken als
     Stille. Braucht vollständige Pakete im Paketspeicher; mit „Nur Kopfdaten“ nicht abspielbar.
  5. **MCP:** ein Lesewerkzeug `calls` (Liste, Ablauf, Qualität), **nie Ton**. `Authorization`, `Proxy-Authorization` und die
     `…-Authenticate`-Köpfe fallen schon unter `auth` in `SECRET_HEADERS` (`src/mcp/privacy.rs`); prüfen, dass das auch bei SIP über UDP
     greift. **Rufnummern gekürzt** (Nutzer, 2026-11-10): in SIP-URIs, `tel:`-URIs und Kopfzeilen (From, To, Contact,
     P-Asserted-Identity …) bleiben nur die letzten zwei Ziffern, die übrigen werden `x` (`sip:+xxxxxxxxxx67@sip.example.net`), in den
     Antworten der Werkzeuge wie in Paketinhalten; Anzeigenamen und Domains bleiben.
- **Abgenommen (Nutzer, 2026-11-10):** der Entwurf („schaut gut aus“, 2026-11-09), Anrufe unter Statistik (Variante D), die verschobenen
  Grenzen der Kopfleiste, „RTP auch ohne SIP erkennen“ standardmäßig an, Rufnummern über MCP gekürzt auf die letzten zwei Ziffern.
- **Umgesetzt: Stufen 1 und 2** (2026-11-10, Test-Paket 0.5.0):
  - Zerleger `src/dissect/sip.rs`: SIP über UDP und TCP auf Port 5060, mit der Einstellung auch auf anderen Ports (an der ersten Zeile);
    alle Kopfzeilen, Kurzformen (`f:`, `i:` …), UTF-8; Körper nach Content-Length; SDP mit allen Zeilen und den Medien (`rtpmap`,
    `a=rtcp`, `a=rtcp-mux`, SRTP-Profile). `src/dissect/rtp.rs`: RTP mit CSRC, Erweiterung und Padding, Payload-Typen nach RFC 3551;
    RTCP SR, RR, SDES, BYE, APP und zusammengesetzte Pakete, ein verschlüsselter Rest als Daten. Die Zerleger bleiben ohne Zustand.
  - Zuordnung in `src/engine/analysis/voip.rs`, in der Reihenfolge der Aufzeichnung: SIP-Transaktionen nach Call-ID und CSeq, Anrufe ab
    INVITE (REGISTER ist kein Anruf), die im SDP ausgehandelten Endpunkte (RTCP auf `a=rtcp`, bei `rtcp-mux` auf demselben Port, sonst
    Port + 1). RTP ohne SIP an der Form: Version 2, gleichbleibende SSRC, Nummern in Schritten von 1 bis 3 über 10 Pakete, beide Ports ab
    1024; die ersten Pakete des Stroms werden nachträglich markiert, RTCP dazu auf Port + 1 mit strenger Prüfung.
  - Paketdetails: „→ Antwort 180 in Paket 4 (nach 612 ms)“, „← Antwort auf Paket …“, „Ausgehandelt per SDP in Paket …“, beim RTP ohne SIP
    „Erkannt: ohne SIP, an der Form …“ (wie im Entwurf).
  - Filter: alle Felder des Entwurfs, dazu `sip.cseq.seq`, `sip.cseq.method`, `rtp.timestamp`, `rtp.marker` und `rtcp.pt`; Zeilenfarbe
    `voip` (`sip or rtp or rtcp`); SIP, SDP, RTP und RTCP in der Protokollhierarchie. Der Treffer-Index wartet am Live-Ende, bis ein Strom
    als RTP erkannt oder verworfen ist (wie bei den TCP-Befunden).
  - Einstellungen `calls.rtp_without_sip` und `calls.sip_any_port` in `conf.toml` (beide an); die Schalter kommen mit Stufe 3.
  - MCP: Kopfzeilen mit geheimem Namen in SIP-Schichten (Authorization, Proxy-Authorization, …-Authenticate) als `[hidden]`. Rufnummern
    kürzt seit Stufe 5 `src/mcp/privacy.rs`.
  - Tests: 11 Zerlegertests (`src/dissect/tests/voip.rs`), 4 zur Zuordnung (Anruf mit SIP und RTP, Filter, RTP ohne SIP, abgebrochener
    Strom und niedrige Ports), einer zum Treffer-Index, Zufallstests für SIP, RTP, RTCP und die Zuordnung (je 300 000 Durchläufe ohne
    Befund). Bildtest mit einem erfundenen Anruf (`GUI_SHOTS_ONLY=voip`: Paketliste, SIP-, RTP- und RTCP-Details, Statistik), gegen den
    Entwurf geprüft.
  - **Abweichungen vom Entwurf** (bewusst): Die Feldwerte der Zerleger sind englisch wie bei allen übrigen Zerlegern und in den Antworten
    über MCP (Entwurf: „erstes Paket des Stroms“, „verschlüsselt oder unbekannt“, „20 ms Sprache“, „2,0 %“, „ms bei 8000 Hz“; bei uns
    „start of a talkspurt“ nach RFC 3550, „encrypted or unknown“, „20 ms of speech“, „2.0 %“, „ms at 8000 Hz“); der Zeitstempel des Sender
    Reports steht wie bei NTP. Übersetzt sind die Bezüge und „Erkannt“.
- **Umgesetzt: Stufe 3** (2026-11-10, Test-Paket 0.5.0):
  - Seitendaten `src/view/calls.rs`: Anrufe aus den SIP-Dialogen (Call-ID), Ströme nach SSRC und Richtung; Verlust aus den laufenden
    Nummern (RFC 3550 A.1, mit Überlauf), Jitter nach A.8 (nur bei bekanntem Takt, sonst „unbekannt“), größter Abstand, Folgefehler
    (Pakete nach einem späteren oder doppelt); Zustände wie im Entwurf (läuft, klingelt, beendet selbst/Gegenseite, verpasst, abgebrochen,
    abgelehnt mit Bedeutung des Codes, nur RTP), Anmeldung (REGISTER mit 200 OK, Gültigkeit, Digest), Bewertung gut/mäßig/schlecht mit
    Hinweis auf der Seite und unter „Auffällig“. Die Daten werden in drei Schritten gelesen (Analyse, Paketspeicher, dann ohne Sperre).
  - Seiten: Statistik mit Unterreitern „Protokolle | Anrufe n“; Anrufe (Kacheln, Liste, Anmeldung als Fuß, Hilfe; leer mit Hilfe); Anruf
    (Fakten, Karte „Anhören“, Ablauf als Leiter mit Band, Sprachströme mit RTCP-Fuß). Wege dorthin: Knopf „Anruf“ über der Paketliste,
    „Gehört zum Anruf …“ in den Paketdetails, „Auffällig“ in der Übersicht. Einstellungen → Anrufe: die zwei Schalter, Anhören, Stand des
    Hinweises zu § 201 mit „wieder zeigen“ (`calls.law_confirmed`, Datum).
  - Richtung: „dieser Rechner“ ist der Absender von REGISTER (das angemeldete Telefon), sonst zählt die Richtung der Pakete. In einer Datei
    gilt jede private Adresse als lokal; fehlt dort die Anmeldung (etwa ein Mitschnitt der FRITZ!Box, der erst nach dem REGISTER beginnt),
    kann die Richtung falsch sein.
  - **Schriften ohne Ligaturen** wie im Entwurf (`font-variant-ligatures: none`): egui 0.37 formt Text mit harfrust, Fira Code machte so
    aus „0x“ ein „×“ (auch in Paketdetails und Filtern). `theme::without_ligatures` benennt beim Laden `liga`, `clig`, `dlig` und `calt` in
    der GSUB-Tabelle um; die Schriftdateien bleiben unverändert.
  - Tests: Testaufnahme `src/view/calls/tests.rs` mit den Anrufen des Entwurfs (Anmeldung 14:36:09, RTP ohne SIP, abgelehnt mit 407 und
    486, beendet 1:31, schlecht mit 4,3 % Verlust, verpasst, laufend), fünf Tests der Seitendaten, einer zu den Ligaturen. Bildtest
    `GUI_SHOTS_ONLY=voip` mit Liste, allen Arten von Anrufen, Einstellungen, Übersicht, schmalem Fenster, Englisch und leerer Liste; gegen
    die Bildschirmfotos des Entwurfs geprüft (Liste, Anrufe A, B, C, E).
  - **Abweichungen vom Entwurf** (bewusst): ohne bekannten Namen stehen Adressen statt `sip.example.net` (im Ablauf die Adresse als Kopf,
    darunter nur „SIP“/„RTP“); das Programm (`linphone`) nur bei einer Live-Aufzeichnung; beim RTP ohne SIP fehlt unter „Inhalt“ der nicht
    prüfbare Satz „die Steuerung lief verschlüsselt über HTTPS“; die Schalter unter Einstellungen → Anrufe gelten sofort für die danach
    analysierten Pakete; kommen Sprache und SIP vom selben Rechner, hat der Ablauf zwei Spuren; ein Anruf, der am Ende einer Datei noch
    läuft, heißt „Läuft“.
- **Umgesetzt: Stufe 4** (2026-11-10, Test-Paket 0.5.0):
  - `src/view/calls/audio.rs`: G.711 A-law und µ-law nach den Tabellen der ITU-T-Empfehlung (eigener Code), beide Richtungen auf einer
    Zeitachse. Die Achse ist die Gesprächszeit (von der Annahme oder der ersten Sprache bis zum Ende des Anrufs oder der letzten Sprache;
    beim laufenden Anruf bis jetzt), höchstens 4 Stunden. Jedes Paket liegt an seinem RTP-Zeitstempel (mit Überlauf), Lücken in den
    laufenden Nummern bleiben still und werden rot. Wellenform: 150 Balken je Richtung (Effektivwert, bezogen auf den lautesten Balken
    beider Seiten, leise Stellen bleiben flach). WAV mit eigenem Kopf (44 Byte): 8 kHz, 16 bit, Stereo, links gesendet, rechts empfangen;
    Dateiname `openpacketmon-call-JJJJMMTT-HHMMSS.wav` nach dem Beginn des Anrufs.
  - `src/gui/pages/calls/player.rs`: Karte „Anhören“ wie im Entwurf (Spuren „Gesendet“/„Empfangen“ mit Namen, Wellenform, Abspielkopf,
    Zeitachse, „Abspielen“/„Anhalten“, Zeit, „Beide | Gesendet | Empfangen“, Hilfe mit den Sätzen zu Rot und zum laufenden Anruf; unter
    900 px die Namen über der Wellenform); ein Klick auf die Wellenform springt dorthin. Wiedergabe über `rodio` als ein Kanal (beide
    Seiten gemischt oder eine), Ende am Schluss, beim Verlassen der Seite und ohne Aufzeichnung; ohne Soundausgabe ein roter Hinweis.
    Knopf „Als WAV speichern“ rechts neben „Pakete des Anrufs“ (Dateidialog, Meldung „Gespeichert“). Vor dem ersten Anhören oder Speichern
    der Hinweis zu § 201 als Dialog („Verstanden, anhören“ / „Verstanden, speichern“); das Datum steht in `calls.law_confirmed` und unter
    Einstellungen → Anrufe.
  - Seitenkopf des Anrufs: die Zeile mit dem Zustand ist eine Textzeile (23 px wie im Entwurf, vorher eine Bedienzeile mit 32 px); die
    Karten stehen dadurch 9 px höher, wie im Entwurf.
  - Tests: G.711 gegen Werte der Tabellen und hin und zurück mit einem Test-Kodierer (alle 256 A-law-Codes, µ-law ohne die zweite Null
    0x7F), WAV-Kopf, Mischen der Kanäle, `call_audio` auf der Testaufnahme (Achse 1:31, Richtung, rote Balken und Stille beim schlechten
    Anruf, laufender Anruf, nichts zu hören bei abgelehnt, verpasst und RTP ohne SIP). Die Testaufnahme trägt erfundene „Sprache“ (Ton mit
    abwechselnden Sprechern wie die Kurve des Entwurfs). Bildtest mit Player halb abgespielt („Gesendet“), Hinweisdialog und schmalem
    Fenster; gegen den Entwurf geprüft (Anruf A und B, Dialog: Zeilen und Fußleiste pixelgleich).
  - **Abweichungen vom Entwurf** (bewusst): Der schlechte Anruf der Testaufnahme ist im Player 1:03 lang statt 1:00 (sein empfangener
    Strom hat wie in den Zahlen des Entwurfs 1044 Pakete à 60 ms, länger als die Gesprächszeit); Namen in der Spalte links werden ein
    Zeichen früher gekürzt (Fira Code ist als Datei 2,5 % breiter als die Webschrift des Entwurfs); im Dialog kein Fokusrahmen um den
    Hauptknopf; abgespielt wird ein Kanal, die WAV-Datei ist Stereo.
  - Nicht automatisch geprüft: echte Tonausgabe und Dateidialog (die Tests spielen nie Ton); zum Ausprobieren mit einem echten Anruf.
- **Umgesetzt: Stufe 5** (2026-11-10, Test-Paket 0.5.0):
  - Werkzeug **`calls`** (`src/mcp/tools.rs`, `CallsArgs` in `args.rs`, `server.rs`): ohne Argument die Liste (Kacheln als `summary`; je
    Anruf `id` wie `sip-3` oder `rtp-7`, Zustand, Beginn, Dauer, Richtung, Von und An mit Adresse, Codec, Qualität, Pakete; höchstens 100,
    neueste zuerst; die Anmeldung), mit `call` die Seite des Anrufs (Fakten als Text, Spuren, Ablauf mit Paketnummern, Sprachströme mit
    Verlust, Jitter, größtem Abstand, Folgefehlern, Bewertung und Filter, RTCP, Hinweis bei schlechter Verbindung, Filter aller Pakete).
    Nie Ton: keine Karte „Anhören“, kein WAV. Die Antworten laufen wie alle durch `finish`.
  - **Rufnummern** (`Numbers` in `src/mcp/privacy.rs`): die Ziffern im Benutzerteil von `sip:`/`sips:`-URIs (bis `@`; ohne `@` ist es ein
    Host und bleibt, außer er beginnt mit `+`), in `tel:`-URIs (Parameter wie `;ext=` je für sich), in den Anzeigenamen der Adressköpfe
    (From, To, Contact, P-Asserted-Identity, P-Preferred-Identity, P-Called-Party-ID, Remote-Party-ID, Diversion, History-Info, Referred-By,
    Refer-To, Reply-To, mit Kurzformen) und in der SDP-Zeile `p=` behalten nur die letzten zwei; ein Passwort in einer SIP-URI
    (`sip:alice:geheim@…`) wird ganz `x`. Das gilt für jeden Text jeder Antwort (`clean_text`), die Adressköpfe in den Feldern der
    SIP-Schicht, die Namen auf den Seiten der Anrufe (auch ohne Schema wie `+4930…@host`; IP-Adressen bleiben) und für Bytes (`Masker`,
    über Segmentgrenzen; an einer Schnittkante im Zweifel auch die letzten zwei Ziffern). Filter-Orakel: `sip.from` und `sip.to` sind im
    Index des Assistenten gekürzt.
  - **Was sonst Ton oder Nummern verriete:** Die Nutzdaten von RTP (auch eines noch nicht entschiedenen Kandidaten) werden in
    `follow_stream` und `packet_bytes` zu `x`, der RTP-Kopf bleibt; in RTCP-Quellbeschreibungen werden PHONE, NAME, CNAME und EMAIL gekürzt
    (Feld und Bytes, `sdes_items` in `src/dissect/rtp.rs`); SRTP-Schlüssel (`a=crypto:`, `a=key-mgmt:`, `k=`) ganz geschwärzt, im Feld
    als `crypto:[hidden]`; der Kopf `Identity` (RFC 8224, trägt die Nummern beider Seiten signiert, Kurzform `y`) gilt als geheim.
  - Geprüft: `Authorization`, `Proxy-Authorization` und `…-Authenticate` greifen bei SIP über UDP (Feld `[hidden]`, Bytes `x`).
  - Texte: `settings-mcp-hint` wie im Entwurf („… Statistik und Anrufe (ohne Ton; Rufnummern bis auf die letzten zwei Ziffern gekürzt)“),
    englisch entsprechend; README (Abschnitt „AI assistants“), Beschreibungen und Anweisungen des MCP-Servers.
  - Tests: sechs in `privacy.rs` (Nummern in allen Formen, was bleibt, Adressen, eine SIP-Nachricht in Bytes an jeder Schnittstelle,
    SRTP-Schlüssel, Ton und RTCP), der Zufallstest des Maskierers mit SIP-Teilen (Stücke verbergen nie weniger als das Ganze; 1 000 000
    Durchläufe ohne Befund); drei in `tools.rs` (ein Testanruf mit Nummern an jeder Stelle: keine Antwort und kein Byte enthält mehr als
    die letzten zwei Ziffern, kein Ton, kein SRTP-Schlüssel; der Filter findet keine Nummer; `calls` auf der Testaufnahme), `calls` im
    Zufallstest der Werkzeuge (100 000 Durchläufe ohne Befund) und in `tests/mcp.rs`.
  - **Grenzen** (bewusst): Ziffern in Anzeigenamen werden auch gekürzt, wenn sie keine Nummer sind („Büro 2041“ → „Büro xx41“), ebenso ein
    Benutzerteil mit Ziffern (`anna2024@…` → `annaxx24@…`); Nummern an anderen Stellen (etwa im Text einer SIP-MESSAGE, in XML-Körpern oder
    im Betreff) bleiben, wie Geheimnisse an anderen Stellen.

---

## 5. Offene Punkte (Stand 2026-11-10)

1. Neue Oberfläche (5.6): im Test-Paket 0.5.0; Test durch den Nutzer.
2. VoIP (5.7, „go“ am 2026-11-10): Stufen 1–5 (Zerleger, Filter, Anrufe, Anhören, MCP) im Test-Paket 0.5.0; Test mit einem echten Anruf
   (Tonausgabe, Dateidialog, Rufnummern über MCP).
3. Windows-Reste (`docs/windows.md` 8): CI-Job, Test auf sauberem System, MCP-Einrichtung in `docs/windows-tester.md`, MCP-Integrationstests
   unter Windows von den echten Einstellungen trennen.
4. Rückmeldung des Originals zum Pull Request mit der Korrektur aus 4.8 (Einzelheiten nur in `CLAUDE.local.md`).
5. Weitere Ideen in 5.2, noch nicht beschlossen.

Die Namenssuche (früher Abschnitt 6, 2026-10-28) ist erledigt: **openpacketmon**, Marke vom Nutzer geprüft.
