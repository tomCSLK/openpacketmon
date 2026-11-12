# CLAUDE.md – openpacketmon (Sniffnet-Fork)

Sprache: Deutsch. **openpacketmon** („Packet Monitoring“) ist ein Fork und die Weiterentwicklung von **Sniffnet**
(https://github.com/GyulyVGC/sniffnet), einem Netzwerkverkehrs-Monitor in Rust: Verkehr nach Hosts, Ländern, Diensten und Programmen bis zum
einzelnen Paket, mit Paketliste, Zerlegern, Anzeigefilter und Statistik („kleines Wireshark“). Native Oberfläche mit **egui**, Linux und Windows.
Repo `tomCSLK/openpacketmon`, **öffentlich** seit 2026-11-02, Lizenz **MIT OR Apache-2.0** wie das Original. Name seit 2026-10-28.
Logo seit der neuen Oberfläche: Zeichen C „Lupe über Paketen“ (`resources/icons/logo-mark.svg`); Programmsymbole und README-Bild erzeugt
`dev/make-icons.py`. Die alten Logo-Originale liegen außerhalb des Repos (`~/Bilder/openpacketmon_Logo_v2.jpeg`, `openpacketmon_icon_v2.jpeg`).

## Wo steht was

| Datei | Inhalt | Laden |
|---|---|---|
| `CLAUDE.md` | Regeln und Grundwissen für jede Sitzung | automatisch |
| `CLAUDE.local.md` | Laptop, gemeldeter Befund beim Original, Pfade zu nicht öffentlichen Projekten (gitignored) | automatisch |
| `PLAN.md` | oben der Stand; Entscheidungen (0), Lizenzprüfung (1), Bestandsaufnahme und Phasen 1–4 (2–4, Verlauf), Vorhaben 5.1–5.8, offene Punkte (5) | **maßgeblich**, vor Arbeit am Vorhaben lesen |
| `docs/windows.md` | alles zu Windows: Einrichten, Code-Unterschiede, Tests, Installer, Code-Signatur, CI, offene Punkte | vor Arbeit unter oder für Windows |
| `docs/entwurf/` | klickbarer Entwurf der neuen Oberfläche (HTML, erfundene Daten), `README.md` dort | vor Arbeit an der Oberfläche |
| `README.md`, `CHANGELOG.md`, `SECURITY.md`, `CONTRIBUTING.md`, `NOTICE` | öffentlich, Englisch | bei Bedarf |

**Diese Datei kurz halten (unter 15 000 Zeichen).** Stand und Verlauf gehören nach `PLAN.md`, Windows nach `docs/windows.md`. Die ausführliche
Fassung bis 2026-11-09 (mit dem Verlauf der Browser-Oberfläche) steht in der Git-Historie.

## Aufbau

- `capture/` – **`openpacketmon-capture`**, der Mitschnitt-Helfer: nur er hat Rechte (`cap_net_raw,cap_net_admin` per `setcap` im `.deb`), gibt sie
  nach dem Öffnen des Adapters ab und reicht Pakete über eine Pipe weiter (Promiscuous Mode mit `--promisc on`, Rückfall ohne);
  übersetzt BPF erst ohne Rechte. `openpacketmon` selbst läuft ohne Sonderrechte. Stirbt der Helfer, meldet das die Oberfläche (`CaptureStopped`). Live-Test `capture/tests/live.rs` im Netz-Namensraum.
- `src/engine/` – Zustand der Aufzeichnung, Paketspeicher `packets.rs` (nur im RAM, wählbar bis 2 GiB und 2 Millionen Pakete), TCP-Analyse
  `analysis.rs`. `Engine::stop` wartet, bis der Mitschnitt seine Dateien geschlossen hat.
- `src/dissect/` – eigene Zerleger (DNS/mDNS/LLMNR, DHCP, TLS mit SNI, HTTP/1, SSDP, NTP, QUIC-Kopf); `src/filter/` – eigene Filtergrammatik.
- `src/view/` – **Seitendaten ohne Oberfläche** (was jede Seite zeigt, Treffer-Index `filter_index.rs`, Stream, Statistik, Vorschläge,
  Übersetzung `i18n.rs` mit Prüfung `every_message_is_used`). Neue Seitendaten gehören hierher.
- `src/gui/` – egui/eframe: `mod.rs` (Fenster 1280 × 820, mindestens 760 × 480), `theme.rs` (Farben), `pages/`, `widgets.rs`, `tests.rs` (Bedienung
  und Bilder). Wireshark-Muster: Fenster schließen beendet das Programm, mit Rückfrage bei laufender Aufzeichnung. Unter Windows zeichnet es mit
  DirectX 12, sonst OpenGL (`WGPU_BACKEND` gilt weiter).
- `src/mcp/` – **`openpacketmon mcp`** (stdio, `rmcp`) fragt das offene Fenster über `127.0.0.1` ab; Fenster und Client weisen sich per
  HMAC-SHA256 aus, Token in `mcp-access.json` (`0600`), standardmäßig aus. Acht Lesewerkzeuge ohne Nutzdaten (darunter `calls`), dazu
  `follow_stream` und `packet_bytes` nur mit dem Schalter „Paketinhalte erlauben“. **Alle Antworten laufen durch `finish` in `tools.rs`**
  (Bereinigung in `privacy.rs`: Cookies, Anmeldeköpfe, URL-Queries, SRTP-Schlüssel, Geheimnisse auch in Bytes durch `x`; Rufnummern bis
  auf die letzten zwei Ziffern, in Texten wie in Bytes; RTP-Nutzdaten, also Ton, nie). Paketinhalte sind ungeprüfte Eingaben für das
  Sprachmodell. `README.md` „opens no network port“ gilt nur, solange der Zugang aus ist.
- `src/mmdb/` – Geodaten DB-IP Lite (CC BY 4.0) auf Klick nach `~/.local/share/openpacketmon/geo` (Windows `%LOCALAPPDATA%\openpacketmon\geo`),
  sonst `/var/lib/GeoIP` oder eigener Pfad. `src/npcap.rs` – `wpcap.dll` verzögert laden (Windows). `src/utils/check_updates.rs` – Update-Prüfung
  über den Release-Titel auf GitHub.
- `locales/de`, `locales/en` – alle Texte (Fluent); `lib/sniffnet-packet-parser` aus dem Original; `resources/packaging/` (`.deb`, Inno Setup);
  `resources/test/` (IPinfo-Beispieldaten, CC BY 4.0).
- `dev/` – `build-deb.sh`, `release.sh`, `build-installer.ps1` (Windows), `third-party-licenses.py`, `check-notices.sh`, `update-services.py`
  (IANA-Dienstliste, CC0), `make-icons.py` (Programmsymbole, README-Bild; Pillow), danach `make-ico.py`.

## Arbeitsregeln

- Erst planen, Umsetzung nur nach ausdrücklicher Freigabe durch den Nutzer; neue Vorhaben erst nach „go“.
- Committen, Pushen und Pakete bauen nur auf Ansage des Nutzers; die Version nennt der Nutzer. Releases, Tags und Veröffentlichen nur auf Ansage.
  Nach einer Änderung Doku nachziehen, lokal prüfen und berichten, nicht auf die CI warten.
- Nach jeder Änderung `PLAN.md` (oben „Stand“ und das Vorhaben), den Stand hier und die Zeile in `~/Claude/Projekte/CLAUDE.md` nachziehen
  (dort alten Stand ersetzen, nicht ergänzen).
- Heimnetz-Details und Lokales nur in `CLAUDE.local.md`. Einzelheiten zum Befund, der an Sniffnet gemeldet wurde (Messwerte, Ablauf, Mail), nur
  dort, nie in Commits, Issues oder öffentlichen Dateien, bis das Original ihn behoben hat; öffentlich nur „ein Befund aus 4.8“.
- Den geplanten Launcher (eigenes, noch nicht öffentliches Projekt) in öffentlichen Dateien nur „der geplante Launcher“ nennen; Name, Aufruf
  und Pfad stehen in `CLAUDE.local.md`.
- Oberfläche: Farben und Aufbau in `src/gui/theme.rs`, Seitendaten in `src/view/`, Zeichnen in `src/gui/`, Bausteine in `src/gui/widgets.rs`,
  Texte nur über `locales/` (jeder Text in `de` und `en`). Gestaltung nach `docs/entwurf/` (grau mit `#1c5ea8`, Fira Sans/Code); neue Seiten
  gegen Bildschirmfotos des Entwurfs prüfen (`PLAN.md` 5.6).
- **Wireshark ist GPL:** daraus nie Code, Zerleger oder Filtergrammatik übernehmen.
- Lizenz- und Copyright-Hinweise des Originals nie entfernen. Wer eine bisher **unveränderte** Datei des Originals ändert (oder Code daraus in
  eine neue Datei übernimmt), setzt oben die zwei Zeilen „Modified for openpacketmon (2026), see NOTICE. Based on `<pfad>` of Sniffnet …“ (wie
  in den übrigen Dateien) und trägt die Datei in `NOTICE` ein (Apache-2.0 §4b); Dateien ohne Kommentarmöglichkeit nur in `NOTICE`. Prüfen mit
  `dev/check-notices.sh` (klont das Original bei `ab4bf739d464`; findet keine Umbenennungen; neu geschriebene Dateien an Pfaden des Originals
  stehen dort in der Liste `replaced`).
- Nach Änderungen an Abhängigkeiten `THIRD-PARTY-LICENSES.txt` mit `dev/third-party-licenses.py` neu erzeugen und mitcommitten (Linux und
  Windows in einer Liste; `dev/build-deb.sh` hält sonst an). Neue Schriften, Symbole und Daten nur mit geklärter Lizenz.
- Npcap niemals mitliefern (proprietär).
- Upstream-Fixes nur gezielt übernehmen (`git format-patch`/`git am` aus einem temporären Klon des Originals; Bezug ist Upstream-Commit
  `ab4bf739d464`, den Tag `upstream-base` gibt es nur noch im Bundle `~/Dokumente/openpacketmon-history-bis-2026-10-29.bundle`), nie die
  Upstream-Historie einspielen.
- Beiträge an das Original nur klein und mit Offenlegung der KI-Hilfe (dessen `CONTRIBUTING.md`). Eigene `CONTRIBUTING.md`: DCO, KI-Hilfe offenlegen.
- Testinstanzen mit `XDG_DATA_HOME`/`XDG_CONFIG_HOME` im Scratchpad starten, damit die echten Einstellungen (`~/.config/openpacketmon/conf.toml`)
  unberührt bleiben.

## Prüfen, bauen, veröffentlichen

- Build unter Linux braucht `libpcap-dev`, `libasound2-dev` (Ton) und `libfontconfig1-dev`. Rust Edition 2024, Workspace mit `capture/` und
  `lib/sniffnet-packet-parser`. Die CI nimmt immer das neueste stable (seit 2026-11-12 Rust 1.100, lokal ebenso), die Windows-Fassung 1.99.1
  (`$env:RUSTUP_TOOLCHAIN = '1.99.1'`, mit 1.100 stürzt der Helfer ab);
  deshalb keine Funktionen der Standardbibliothek ab 1.100 und Clippy-Lint `map_unwrap_or` aus. Cargo baut ohne Warnung, so soll es bleiben.
- Prüfen: `cargo test --workspace`, `cargo clippy -- -D warnings` (ohne `--all-targets`, wie das Original; der Testcode hat viele
  Pedantic-Meldungen), `cargo fmt --all -- --check`, `cargo deny --workspace check` (cargo-deny 0.20.2; ohne `--workspace` fehlt der Helfer;
  prüft Linux und Windows). Lange Zufallstests: `FUZZ_ITERATIONS=1000000 cargo test --release fuzz`.
- Jeder `cargo build` entfernt die `setcap`-Rechte des Helfers. Live-Mitschnitt aus `target/` nur nach
  `sudo setcap cap_net_raw,cap_net_admin=eip target/…/openpacketmon-capture`; das Hauptprogramm braucht keine Rechte.
- **Versionen** wie bei opennetmon: `0.x.y` ohne Datum, vor jedem Test-Paket in `Cargo.toml` und `capture/Cargo.toml` erhöhen, danach
  `cargo update --workspace --offline` (sonst scheitert die Lizenzprüfung an `Cargo.lock`); `y` bei Kleinigkeiten, `x` bei größeren Schritten.
- **Test-Paket:** `dev/build-deb.sh` baut alles mit der Version aus `Cargo.toml` und nennt den Installationsbefehl. Es hält an, wenn die
  Version nicht neuer als die installierte ist (`apt install` überspringt ein Paket gleicher Version stillschweigend); `--force` baut trotzdem,
  dann `apt install --reinstall`. Von Hand: `cargo build --release --workspace && cargo deb` (`cargo deb` allein baut den Helfer nicht neu).
- **Release:** `dev/release.sh` (Release-Titel `vX.Y.Z`, sonst bleibt die Update-Prüfung stumm; Ablauf in `PLAN.md` 4.10). Windows-Installer
  auf dem Windows-Rechner bauen und signieren: `powershell -ExecutionPolicy Bypass -File dev\build-installer.ps1 -CertSha1
  5A5AE6506A7649F1FFBD9856FB22E63E39D98C17` (SimplySign Desktop angemeldet; Certum-Zertifikat bis 2027-11-03), dann
  `dev/release.sh --add-windows <installer> --publish` in Git Bash nach dem Release unter Linux (`docs/windows.md` 6.1).

## Stand und offene Punkte

Maßgeblich ist `PLAN.md` (oben „Stand“, Abschnitt 5). Kurz (2026-11-12):

- **v0.5.2** ist released (2026-11-12, Linux) mit Promiscuous Mode und dem Startknopf oben; der **signierte Windows-Installer** ist am
  selben Tag hinzugefügt (Rust 1.99.1; `docs/windows.md` oben unter „0.5.1 und 0.5.2“). Danach Abhängigkeiten und `Cargo.toml`
  aufgeräumt (`PLAN.md` oben).
- **Windows nur mit Rust 1.99.1 bauen:** Mit 1.100 importiert der Helfer `WaitOnAddress` aus `vertdll.dll` und stürzt ab (alte
  `windows-sys` 0.36.1 über `pcap` 2.5.0, windows-rs #2027); `docs/windows.md` oben. Korrektur im Code noch offen.
- **Befund am TAP** (`PLAN.md` 5.8, 2026-11-12): Am Monitoring-Port eines Netzwerk-TAPs fehlte unter Linux und Windows mehr als die
  Hälfte der Pakete gegenüber Wireshark, weil der Helfer fest ohne Promiscuous Mode öffnete. **Umgesetzt** (Schritt 2, Test-Paket 0.5.1):
  Einstellung `capture.promiscuous` (Standard an), Schalter auf der Seite Aufzeichnung, Helfer-Option `--promisc on|off` mit Rückfall
  ohne den Modus, Zustand in `Ready` und in der laufenden Aufzeichnung. Am TAP unter Linux bestätigt; offen Windows
  (`docs/windows.md` oben unter „0.5.1 und 0.5.2“), Schritte 3 und 4.
- **Startknopf oben** (`PLAN.md` 5.6, 2026-11-12, Test-Paket 0.5.2): startet auf der Seite Aufzeichnung jetzt wie der untere (vorher nur Seitenwechsel).
- **v0.5.0** ist released (2026-11-10) mit neuer Oberfläche und VoIP, für Linux und mit **signiertem Windows-Installer** (am selben Tag
  hinzugefügt); Test durch den Nutzer steht aus. Unter Windows alle Tests grün; Smart App Control sperrt dort Proc-Macro-DLLs, Abhilfe
  in `docs/windows.md` oben. Update von 0.4.0 mit dem Installer beim Nutzer reibungslos; Test läuft.
- **Neue Oberfläche im Stil von opensecmon** (`PLAN.md` 5.6, `docs/entwurf/`): am 2026-11-10 in egui umgesetzt (Stufen A–D: Farben und
  Schriften, Kopf- und Statusleiste mit Hinweis auf neue Versionen, alle Seiten samt Einstellungen mit Unternavigation und Release-Notes,
  Symbole, Logo C, Programmsymbole, README-Bild), gegen den Entwurf geprüft; Abweichungen in `PLAN.md` 5.6.
- **VoIP** (`PLAN.md` 5.7): SIP/SDP/RTP/RTCP in Paketliste und Filter, unter Statistik der Reiter „Anrufe“ (Ablauf, Sprachqualität, Anhören
  von G.711 mit Hinweis auf § 201 StGB; nicht in der Kopfleiste, weil selten gebraucht), Entwurf **abgenommen** (2026-11-10). „go“ am
  2026-11-10. **Stufen 1–5 umgesetzt** (Zerleger `src/dissect/sip.rs`, `rtp.rs`, Zuordnung `src/engine/analysis/voip.rs`, Filter,
  Zeilenfarbe, Protokollhierarchie; Seitendaten `src/view/calls.rs` mit Testaufnahme `calls/tests.rs`, Seiten `src/gui/pages/calls.rs`,
  Einstellungen → Anrufe; Anhören: G.711 und WAV in `src/view/calls/audio.rs`, Player und Dialog zu § 201 in
  `src/gui/pages/calls/player.rs`; MCP-Werkzeug `calls`, Rufnummern gekürzt mit `Numbers` in `src/mcp/privacy.rs`; Bildtest
  `GUI_SHOTS_ONLY=voip`); als Nächstes der Test mit einem echten Anruf. Echte Tonausgabe und Dateidialog sind nur von Hand zu
  prüfen. Keine echten Mitschnitte oder Rufnummern ins Repo, Ton nie über MCP, Rufnummern über MCP bis auf die letzten zwei
  Ziffern gekürzt.
- Windows-Reste (`docs/windows.md` 8): CI-Job, Test auf sauberem System, MCP-Einrichtung für Tester, MCP-Tests unter Windows von den echten
  Einstellungen trennen (`confy` ignoriert dort `XDG_CONFIG_HOME`).
- Pull Request mit der Korrektur des Befunds beim Original angenommen (2026-11-12, Zweig `v1.6.0`, `CLAUDE.local.md`).
- Ideen ohne Beschluss in `PLAN.md` 5.2.
