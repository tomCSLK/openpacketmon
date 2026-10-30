# PLAN.md – Fork von Sniffnet

Stand: 2026-10-28. Phasen **1, 2 und 3 erledigt** (Repo privat: https://github.com/tomCSLK/openpacketmon; Kern als Engine, Browser-UI auf 127.0.0.1, `.deb` mit systemd-User-Dienst). Nächster Schritt: Phase 4 vor dem Öffentlich-Schalten, Phase 5 gemeinsam festlegen.

Upstream: https://github.com/GyulyVGC/sniffnet – Autor Giuliano Bellini, letztes Release **v1.5.1** (2026-07-22),
geprüfter Stand `main` @ `ab4bf73` (2026-10-20).

> Hinweis: Die Lizenzprüfung ist eine sorgfältige technische Auswertung der Lizenztexte, keine Rechtsberatung.

---

## 0. Entscheidungen des Nutzers (2026-10-28)

| Thema | Entscheidung |
|---|---|
| GitHub | Zuerst **privates** Repo, später öffentlich schaltbar. |
| Lizenz | **Wie das Original: `MIT OR Apache-2.0`**, auch für eigene Änderungen. |
| Funktionsumfang | **Wie das Original** (Liste in Abschnitt 2.1). Bewusst entfernt (2026-10-29): Mini-Ansicht, Benachrichtigungen nach Datenmenge, Blacklist und Webhook, Favoriten für Dienste, die **IP-Blacklist** ganz. |
| Oberfläche | **Komplett neu, im Stil von opennetmon.** Fonts und Icons des Originals sind damit irrelevant. |
| Einsatzort | **Auf dem Client** installiert (Arbeitsrechner), nicht auf dem Server. **Keine Server-Installation vorgesehen** (2026-10-28): Bedienung nur am selben Rechner, keine Handy-Darstellung nötig. |
| Name | **openpacketmon** (2026-10-28). Crate, Binary, Paket und GitHub-Repo `tomCSLK/openpacketmon`. Markenregister prüft der Nutzer. |
| Logo | **`~/Bilder/openpacketmon_Logo_v2.jpeg`** (2400×1792, dunkler Hintergrund: Netz-Kugel mit Paket und Pulslinie, Schriftzug „openpacketmon – PACKET MONITORING“). Original bleibt außerhalb des Repos; `_v1` ist verworfen. |
| Verhältnis zu upstream | **Abspalten** (eigenständige Weiterentwicklung, kein regelmäßiges Mergen). Erweiterungen planen wir gemeinsam. |
| Plattform | **Zunächst nur Linux** (Zorin OS / Ubuntu, `.deb`). **Windows später vorgesehen** (2026-10-28; Npcap installiert der Nutzer selbst, wird nie mitgeliefert); macOS offen. |
| UI-Variante | **A – lokaler Dienst + Browser** (Abschnitt 3); **dazu B – eigenes Fenster mit Symbol im Infobereich** (beschlossen und umgesetzt 2026-10-28, Abschnitt 4 „Fenster-Programm“). |
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
Für Pakete: `THIRD-PARTY-LICENSES.txt` automatisch erzeugen (Skript wie `PRTG_Nachbau/dev/third-party-licenses.py` oder `cargo-about`),
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

## 3. Architektur der neuen Oberfläche (Vorschlag)

opennetmon ist eine **Web-Oberfläche** (axum + askama, `static/app.css` als Designsystem, uPlot, Fluent, `rust-embed`).
„Aussehen wie opennetmon“ heißt am einfachsten: dieselbe Technik, auf dem Client.

| Variante | Beschreibung | Bewertung |
|---|---|---|
| **A – lokaler Dienst + Browser** (**gewählt** 2026-10-28) | Ein Programm mit Aufzeichnungsrechten (`cap_net_raw`) läuft im Hintergrund (systemd-Benutzer- oder Systemdienst) und liefert die UI nur auf `127.0.0.1`. Bedienung im Browser, Startmenü-Eintrag öffnet die Seite. | Größte Wiederverwendung von opennetmon (CSS, Templates, Graphen, Übersetzungen). Aufzeichnung läuft weiter, wenn der Browser zu ist. Rechte sauber getrennt. |
| B – eigenes Fenster (WebView: `wry`/Tauri) | Dieselbe HTML-Oberfläche in einem App-Fenster statt im Browser. | Wirkt wie eine Desktop-App; zusätzliche Abhängigkeiten (WebKitGTK, WebView2). Lässt sich **später auf A aufsetzen**. |
| C – iced behalten, nur umstylen | Original-GUI-Technik mit opennetmon-Farben. | Sieht nie wirklich wie opennetmon aus, kein Wiederverwenden. Nicht empfohlen. |

Bei A wichtig: Zugriff auch lokal absichern (andere Benutzer auf dem Rechner könnten den Port erreichen) – Anmeldung oder
Zugangstoken im Startlink; nur `127.0.0.1` binden; Schreibaktionen mit CSRF-Schutz wie in opennetmon.

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
stehen – `dependabot.yml` kommt dann ebenfalls weg). Lokaler Ordner `Sniffnet_Fork/` ist das Repo.
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
   `PLAN.md` beschrieben. Offen gelassen: Herkunft/Rechte der Vorlage (nur Farbwerte übernommen, keine Assets).

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
2. **IPFIX-Collector** (lauscht standardmäßig auf `0.0.0.0:4739`): eine Vorlage mit 16 000 Feldern der Länge 0 und
   ein 64-KB-Datagramm kosteten ~6 s CPU je Datagramm; 2000 solcher Vorlagen ~390 MB (Grenze 10 000 → ~1,9 GB).
   Jetzt werden Felder der Länge 0 verworfen (jedes Feld braucht ≥ 1 Byte) und Vorlagen mit mehr als 512 Feldern
   ignoriert (`collect.rs`); Test `test_process_datagram_hostile_templates`.
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

**Haltepunkt (2026-10-30):** Der IPFIX-Befund aus 4.8 steckt weiter in Sniffnet (`collect.rs` in `main` @ `931d7ede`
unverändert, der ganze IPFIX-Pfad gleich). Am 2026-10-30 per E-Mail an `security@sniffnet.app` gemeldet (Weg laut deren
`SECURITY.md`), mit dem Versprechen, das Repo bis zu ihrer Antwort privat zu lassen, weil Code, Test und 4.8 den Fund
offenlegen. Messung auf unserem Code ohne die Korrektur (Release ohne LTO): 11,7 s CPU je 65-KB-Datagramm, 2000 Vorlagen
377 MiB. Sniffnet selbst wurde nicht gebaut. Die übrigen Befunde aus 4.8 liegen im neuen Code, nicht im Original.
**Nicht umschalten, bevor Sniffnet geantwortet hat oder ein Termin vereinbart ist.** `dev/release.sh --publish` hält bei
einem privaten Repo an.

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

Noch offen:
- Nutzertest 0.2.6 abschließen; Version nennt der Nutzer.
- Herkunft der Vorlage `../design-system` klären (Phase 3 Punkt 7, nur Farbwerte übernommen).
- Trockenlauf `dev/release.sh`, Installationstest auf sauberem Ubuntu 24.04 und Debian 13.
- Repo-Einstellungen: Themen (`network-monitor`, `packet-capture`, `rust`, `linux`), Force-Push auf `main` sperren.
  Beschreibung gesetzt, Wiki/Projects aus.
- Sichtbarkeit auf **öffentlich** (macht der Nutzer oder nach ausdrücklicher Freigabe per `gh repo edit
  --visibility public --accept-visibility-change-consequences`). Direkt danach „Private vulnerability reporting“ an: Die API
  antwortet beim privaten Repo mit 404, und `SECURITY.md` verweist darauf.
- Release: `dev/release.sh --publish --notes <datei>`, danach `cargo test test_fetch_latest_release_from_github -- --ignored`.

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

---

## 5. Offene Fragen an den Nutzer

Keine offenen Fragen. Phasen 1 und 2 am 2026-10-28 umgesetzt; Phase 3 erst nach Freigabe.

## 6. Namenskandidaten (erledigt: **openpacketmon** gewählt)

Am 2026-10-28 weder auf crates.io noch als exakter GitHub-Repo-Name vergeben: `opentrafficmon`, `opennetscope`, `openpacketmon`,
`opennettraffic`, `opennetwatch`, `netlauscher`. (`trafficlens` ist auf GitHub mehrfach vergeben.)
Markenregister sind damit **nicht** geprüft – das macht der Nutzer vor der Festlegung.
