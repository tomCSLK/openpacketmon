# Windows-Version von openpacketmon

Stand: 2026-10-30, ergänzt 2026-11-02 (MCP, Mitschnitt-Helfer, vorgezogener Beginn, Installer für Tester, Erfahrungen beim Einrichten des Windows-Rechners). Diese Datei beschreibt, was nötig ist, um das Projekt auf einem Windows-Rechner fortzusetzen und dort eine
Windows-Version zu bauen und zu testen.

**Heutiger Stand (2026-11-02, abends):** Im Zweig `windows` bauen **Debug- und Release-Fassung** unter Windows, die Tests sind grün, und
das Fenster startet. Die Release-Fassung öffnet kein Konsolenfenster, auch nicht für die Helfer, und `openpacketmon.exe` trägt Icon
und Versionsangaben (Abschnitt 4). Dazu gekommen: Adapter erscheinen mit ihrer Beschreibung statt `\Device\NPF_{…}`, eine
importierte PCAP-Datei ist gleich nach dem Stoppen wieder frei, ohne Npcap startet das Programm mit einem Hinweis statt abzustürzen,
die C-Laufzeit ist statisch gelinkt, der Helfer schneidet im Test auf dem Npcap-Loopback-Adapter mit, und es gibt einen
**Installer** (Inno Setup, `dev/build-installer.ps1`, Abschnitt 6) samt Anleitung für Tester (`docs/windows-tester.md`).
Mitschnitt im Fenster vom Nutzer getestet, bisher ohne Probleme; MCP geprüft; erste Tester-Fassung **0.3.11** gebaut (Abschnitt 6).
**Signiert und veröffentlicht (2026-11-03):** Der Installer 0.4.0 ist mit dem Certum-Zertifikat signiert, vom Nutzer mit Smart App
Control getestet und zum Release `v0.4.0` auf GitHub hinzugefügt (Abschnitt 6.1). **Noch nicht ausprobiert:** Installation auf einem
sauberen System.
Die Punkte in Abschnitt 4 stammen aus einer Durchsicht des Codes auf dem Linux-Rechner, ergänzt um das, was unter Windows
aufgefallen ist.

**Erster `cargo check --workspace` unter Windows (2026-11-02, Zweig `windows`):** Die Abhängigkeiten bauen, auch `aws-lc-sys`,
`ring` und `pcap` ohne CMake und NASM. Einziger Fehler war `libc::timeval` in `src/networking/types/capture_helper.rs`
(`suseconds_t` fehlt, Felder unter Windows 32-Bit `c_long`); behoben mit `as _`, danach grün. Übrig sind Warnungen über Code,
der nur unter Linux genutzt wird: `PIPE_BYTES`, `let mut cap` und (Clippy) ein unnötiger Rückgabewert in `capture/src/main.rs`,
`get_desc` in `src/networking/types/capture_context.rs`. **Behoben** (2026-11-02): `PIPE_BYTES` und das `mut` nur noch unter Linux,
`drop_rights` ohne Linux mit begründetem `allow(clippy::unnecessary_wraps)`, `get_desc` mit begründetem `allow(dead_code)`, bis die
Adapter unter Windows mit ihrer Beschreibung erscheinen (Tabelle in Abschnitt 4). `cargo clippy -- -D warnings` ist unter Windows grün,
der Helfer auch gegen `--target x86_64-unknown-linux-gnu` (mit `LIBPCAP_VER` gesetzt, nur Prüfung ohne Linken). `cargo build --workspace` linkt danach ohne weitere Fehler gegen `wpcap.lib`; `openpacketmon.exe --version`
läuft, `openpacketmon-capture.exe` startet (findet also `wpcap.dll`). Dass sich etwas bauen lässt, heißt nicht, dass es unter
Windows richtig läuft (Abschnitt 4). `cargo build --release` scheiterte zuerst an `redirect_stdout_stderr_to_file` (nur
`cfg(all(windows, not(debug_assertions)))`, Crate `gag` fehlt in `Cargo.toml`); behoben, siehe unten.

**Release-Fassung (2026-11-02):** Nach dem Entfernen von `redirect_stdout_stderr_to_file` baut `cargo build --release --workspace`
(`openpacketmon.exe` rund 26 MiB, Helfer 183 KiB). Geprüft: Beim Start ohne Terminal erscheint nur das Fenster „openpacketmon“, kein
Konsolenfenster; die 9 Helfer haben je einen Konsolen-Host ohne Fenster (`CREATE_NO_WINDOW`) und sind nach dem Beenden weg.
Icon und Versionsangaben (Produktname, Version 0.3.10) stehen in der `.exe`. `openpacketmon.exe mcp` antwortet über Pipes
(`initialize`, `tools/list`, `capture_status` mit der erwarteten Absage ohne offenes Fenster). Ausgaben von Kommandozeilen-Optionen
erscheinen nur, wenn die Ausgabe umgeleitet wird, etwa `openpacketmon.exe --list-adapters | Out-Host` in PowerShell.

**Erster Start des Fensters (2026-11-02, Debug-Fassung):** `openpacketmon.exe` öffnet das Fenster „openpacketmon“ (rund 370 MB
Arbeitsspeicher in der Debug-Fassung) und startet 9 Helfer für die Adapter-Vorschau; nach dem Beenden des Prozesses waren nach 3 s
keine Helfer mehr übrig. Nur 8 s gelaufen und hart beendet, also weder Bedienung noch Rückfrage beim Schließen noch Mitschnitt geprüft.

**Erster `cargo test --workspace` (2026-11-02):** 384 Tests bestanden, 2 gescheitert, 5 übersprungen (die schon unter Linux
übersprungenen; `capture/tests/live.rs` besteht nur, weil es ohne `unshare` gleich aufhört, prüft unter Windows also nichts).
- `mmdb::geo::tests::the_file_of_the_settings_comes_first`: `geo_dir` braucht `HOME`, das es unter PowerShell nicht gibt; erwartet,
  siehe `geo_dir` in Abschnitt 4. **Behoben:** `geo_dir` nimmt unter Windows `%LOCALAPPDATA%\openpacketmon\geo`. Die Pfade von
  `geoipupdate` (`/var/lib/GeoIP`) bleiben vorerst auch unter Windows in der Liste, ebenso der Hinweistext `settings-mmdb-hint`, der sie
  nennt; beides noch zu entscheiden (Abschnitt 4).
- `engine::packets::tests::packets_are_written_as_pcap`: löscht den Ordner, während `cap` die Datei noch offen hält; unter Windows
  lässt sich eine offene Datei nicht löschen (Fehler 32). **Behoben** mit `drop(cap)` vor `remove_dir_all`.
- Aus demselben Grund lassen weitere Tests ihre Ordner in `%TEMP%` liegen, ohne zu scheitern, weil sie mit `let _ =` aufräumen
  (`openpacketmon-test-*` aus `engine::tests`, `openpacketmon-gui-*`, `openpacketmon-mcp-*`, `openpacketmon-mcp-tools-*`); nach dem
  Lauf lassen sie sich löschen. **Ursache:** die Handle-Vererbung an den Mitschnitt-Helfer (Punkt „Helfer erbt offene Dateien“ in
  Abschnitt 4). Nebenbei gefunden: Die GUI-Tests löschten ihren Ordner `openpacketmon-gui-<pid>` nie (auch unter Linux nicht), und
  `engine::tests::pcap_import_fills_the_state` und `engine::packets::tests::packets_are_written_as_pcap` benutzten denselben Ordner
  `openpacketmon-test-<pid>`, obwohl sie gleichzeitig laufen können. **Behoben (2026-11-02):** Alle Tests räumen mit
  `engine::tests::remove_test_dir` auf (Wiederholung bis 10 s, sonst Fehler), jeder hat einen eigenen Ordner, und in Unit-Tests starten
  keine echten Helfer mehr (`capture_helper::tests::REAL_HELPERS`). Danach bleiben keine Ordner mehr in `%TEMP%`.
- Im zweiten Lauf zeigte sich `mcp::access::tests::turning_the_access_off_and_on_during_questions` (4 von 5 Läufen rot): Unter
  Windows wird eine Verbindung auf einen gerade geschlossenen Port von `127.0.0.1` nicht sofort abgewiesen, sondern hängt bis
  `CONNECT_TIMEOUT` (2 s); der Test wartete nach dem Umschalten nur 200 ms. **Behoben:** Er wartet jetzt bis zur ersten Antwort,
  höchstens 10 s. Im Programm heißt das nur: Nach dem Schließen des Fensters bekommt ein Assistent die Absage unter Windows 2 s später.
- Danach `cargo test --workspace` **grün** (386 bestanden, 5 übersprungen).
- `mcp::access::tests::a_flood_of_connections_passes_and_the_door_answers_again` scheiterte unter Windows in 2 von 8 Läufen mit
  `NotAvailable`: Windows weist bei voller Warteschlange sofort ab, statt die Verbindung warten zu lassen. Der Test lässt diese
  Antwort unter Windows zu (2026-11-02); geprüft wird weiter, dass die Tür danach wieder antwortet.
- Stand 2026-11-02 abends: 388 bestanden, 6 übersprungen, keine Testordner übrig.

**Smart App Control** (auf diesem Rechner an) blockiert ab und zu frisch gebaute, unsignierte Dateien aus `target\`, auch Testdateien
und Build-Skripte von Abhängigkeiten (beim ersten Release-Build `crc32fast`; Abhilfe: dessen Ordner unter `target\release\build\` löschen)
(„Eine Anwendungssteuerungsrichtlinie hat diese Datei blockiert“, Fehler 4551; Einträge im Ereignisprotokoll
`Microsoft-Windows-CodeIntegrity/Operational`, IDs 3033/3077). Es bewertet jede Datei einzeln; neu linken hilft (z. B. Zeitstempel von
`src/main.rs` setzen, dann erneut `cargo test`). Für den Installer heißt das: Bei Testern mit Smart App Control kann ein unsigniertes
Programm ganz blockiert sein, nicht nur von SmartScreen gewarnt (Abschnitt 6).

**Feste Vorgaben:** Npcap wird **nie mitgeliefert**, der Nutzer installiert es selbst (proprietäre Lizenz). Wireshark-Code
bleibt tabu. Die Arbeitsregeln in `CLAUDE.md` gelten unverändert. Die Arbeit beginnt **vor dem ersten Release** (vorgezogen
2026-11-02, siehe Abschnitt 8); die MCP-Anbindung (`PLAN.md` 5.4) ist fertig, die Windows-Version enthält sie also. Seit der Umstellung auf egui
(`PLAN.md` 5.5) gibt es keinen Webserver und kein Webview mehr. Der Installer soll, wenn möglich, signiert
werden (Nutzer, 2026-10-30/31).

## 1. Was auf dem Windows-Rechner installiert sein muss

Vorgesehen sind Windows 10/11 x64. Auf dem eigenen Windows-Rechner (Windows 11 Pro) am 2026-11-02 eingerichtet; dort
ausprobiert sind `winget install Rustlang.Rustup` (rustup 1.29.2, rustc 1.99.1), `cargo install cargo-deny` und Npcap 1.90 mit
SDK 1.17. Git, GitHub CLI, Python 3.12 und Visual Studio waren schon vorhanden, die übrigen `winget`-Befehle sind also weiter Vorschläge.

| Was | Wofür | Installation |
|---|---|---|
| Git für Windows | Repo, außerdem Git Bash für `dev/*.sh` | `winget install Git.Git` |
| Visual Studio Build Tools mit „Desktop development with C++“ und Windows SDK | Linker und C-Compiler für Rust | `winget install Microsoft.VisualStudio.2022.BuildTools`, danach die Komponente im Visual Studio Installer wählen. Ein vorhandenes Visual Studio mit C++-Werkzeugen und Windows SDK genügt (auf dem Rechner: Visual Studio Community 2026, SDK 10.0.26100), dann keine Build Tools dazu |
| Rust (rustup, Toolchain `stable-x86_64-pc-windows-msvc`) | Bauen | `winget install Rustlang.Rustup`; legt die Toolchain gleich mit an. `%USERPROFILE%\.cargo\bin` steht erst in einem **neuen** Terminal im Pfad |
| Npcap | Mitschnitt zur Laufzeit, die Tests und der Build von `pcap` brauchen `wpcap.dll` | https://npcap.com/#download (`npcap-<version>.exe`, signiert von Nmap Software LLC). **Nicht über winget** zu haben, und die freie Ausgabe lässt sich nicht still installieren (`/S` nur bei Npcap OEM): Installer von Hand mit Administratorrechten durchklicken und **„Install Npcap in WinPcap API-compatible Mode“ anhaken**, sonst liegt `wpcap.dll` nur in `System32\Npcap` und nicht im Suchpfad. Prüfen: `Test-Path C:\Windows\System32\wpcap.dll` und `Get-Service npcap` |
| Npcap SDK | Linken gegen `wpcap.lib` | https://npcap.com/#download (`npcap-sdk-<version>.zip`, ohne veröffentlichte Prüfsumme), entpacken (auf dem Rechner nach `%USERPROFILE%\npcap-sdk`, braucht keine Administratorrechte), `Lib\x64` in die Benutzervariable `LIB` (Befehl in Abschnitt 3) |
| Python 3 | `dev/third-party-licenses.py`, `dev/update-services.py` | `winget install Python.Python.3.12` |
| GitHub CLI | Anmeldung, Repo, später Releases | `winget install GitHub.cli` |
| cargo-deny 0.20.2 | Lizenz- und Sicherheitsprüfung | `cargo install cargo-deny --locked --version 0.20.2` (baut einige Minuten) |
| Claude Code | Weiterarbeit | wie gewohnt |

Falls `aws-lc-sys` beim Bauen CMake oder NASM verlangt, beides nachinstallieren (`winget install Kitware.CMake`,
`winget install NASM.NASM`).

Für den Installer: **Inno Setup 6** (`winget install JRSoftware.InnoSetup --scope user`, landet in
`%LOCALAPPDATA%\Programs\Inno Setup 6`; auf dem Rechner 6.7.3). Lizenz: freie Nutzung für jeden Zweck, kommerzielle Nutzer werden um
einen Kauf gebeten, nicht verpflichtet.

**Zum Testen sinnvoll:** eine echte Netzwerkkarte, und für den Installationstest eine zweite, saubere Windows-Umgebung
(virtuelle Maschine), in der noch kein Npcap und kein Rust installiert ist.

## 2. Zugänge und Dateien

- **GitHub:** Das Repo `tomCSLK/openpacketmon` ist noch privat, also vorher `gh auth login` mit dem Konto `tomCSLK`.
- **Im Repo ist alles, was zum Bauen nötig ist.** Das gilt auch für die Icons bis 1024 px (`resources/packaging/linux/graphics/`).
- **Nur auf dem Linux-Laptop, zum Bauen nicht nötig:**
  - `CLAUDE.local.md` (nur Angaben zum Laptop),
  - `~/Claude/Projekte/CLAUDE.md` (Übersicht aller Projekte),
  - `~/Bilder/openpacketmon_*` (Original-Logos),
  - `../PRTG_Nachbau` (opennetmon) und `../design-system`,
  - `~/Dokumente/openpacketmon-history-bis-2026-10-29.bundle` (alte Historie),
  - der lokale Zweig `design-muster`.
- **Notizen von Claude Code** liegen nur auf dem Laptop. Die wichtigen Regeln stehen deshalb auch in `CLAUDE.md`
  (Arbeitsregeln) und `PLAN.md` 4.10 (Haltepunkt, seit 2026-11-02 aufgehoben).

## 3. Einrichten

In PowerShell, nach dem Installieren von Abschnitt 1:

```powershell
gh auth login
gh repo clone tomCSLK/openpacketmon
cd openpacketmon
# dorthin, wo das Npcap SDK entpackt wurde (auf dem eigenen Rechner %USERPROFILE%\npcap-sdk)
[Environment]::SetEnvironmentVariable("LIB", "$env:USERPROFILE\npcap-sdk\Lib\x64;" + [Environment]::GetEnvironmentVariable("LIB", "User"), "User")
# neues Terminal öffnen, dann:
cargo check --workspace
```

`.gitattributes` setzt für Textdateien `eol=lf`, die Zeilenenden bleiben also auch unter Windows richtig.

Die Arbeit gehört in einen eigenen Zweig (z. B. `windows`), bis sie unter Linux und Windows grün ist. Die Linux-Fassung muss
dabei unverändert weiter bauen; Windows-Code kommt hinter `#[cfg(windows)]`, Linux-Code hinter `#[cfg(unix)]`.

## 4. Was im Code zu tun ist

### Programm `openpacketmon`

| Stelle | Heute | Für Windows |
|---|---|---|
| `src/utils/formatted_strings.rs` `redirect_stdout_stderr_to_file` | nutzte die Crate `gag`, die nicht mehr in `Cargo.toml` steht, **baute im Release nicht** | **erledigt** (2026-11-02): Funktion entfernt, sie wurde nirgends aufgerufen. Ohne Konsole gehen Meldungen über `eprintln!` (z. B. `error_logger`) verloren; eine Log-Datei (`get_logs_file_path` gibt es noch) bei Bedarf später |
| Konsolenfenster | ging mit dem Fenster auf | **erledigt** (2026-11-02): `windows_subsystem = "windows"` in `src/main.rs`, nur in der Release-Fassung (die Debug-Fassung behält die Konsole für ihre Ausgaben). Die Helfer startet `capture_helper.rs` mit `CREATE_NO_WINDOW`, sonst bekäme jeder ein eigenes Konsolenfenster. **Folge:** In der Release-Fassung zeigen `--version`, `--help`, `--list-adapters`, `--config-path`, `--restore-default` und Fehler zu `--adapter` im Terminal nichts an (Abhilfe bräuchte `unsafe` mit `AttachConsole` oder ein zweites Programm; ohne `unsafe` ginge ein Dialog). `openpacketmon.exe mcp` ist nicht betroffen, es spricht über Pipes |
| Programm-Icon und Versionsangaben | fehlten | **erledigt** (2026-11-02): `dev/make-ico.py` setzt `resources/packaging/windows/openpacketmon.ico` aus den PNGs von 16 bis 256 px zusammen (unverändert als PNG eingebettet, ohne Bildbibliothek); `build.rs` bettet es mit `winresource` (MIT, nur Build-Abhängigkeit unter Windows) samt Produktname, Beschreibung und Version ein. Braucht `rc.exe` aus dem Windows SDK |
| `src/mmdb/geo.rs` `geo_dir` | `HOME`, `.local/share` (`HOME` fehlt unter Windows) | `%LOCALAPPDATA%\openpacketmon\geo` (**erledigt** 2026-11-02) |
| `src/mmdb/geo.rs` `geoipupdate_file` | `/var/lib/GeoIP/…` | unter Windows weglassen oder den Ordner von `geoipupdate` nehmen, Test anpassen |
| `src/networking/traffic_preview.rs`, `capture_context.rs` | aus Sniffnet, schon für Windows vorbereitet | prüfen |
| `capture/src/main.rs` (Helfer) | unter Linux nicht blockierender Adapter mit eigenem `poll`, Rechteabgabe, größere Pipe | außerhalb von Linux liest er blockierend mit dem Timeout von libpcap, die drei Funktionen `drop_rights`, `enlarge_pipe`, `wait_for_packets` tun dort nichts. **Geprüft** (2026-11-02, `captures_on_the_npcap_loopback_adapter` in `capture/tests/live.rs`): Unter Npcap kommt der Timeout auch ohne Pakete, eine Pause bleibt also nicht hängen. Pfad mit `.exe` löst `helper_path` schon auf |
| Adapterliste, `--adapter`, Aufzeichnungsseite | Namen wie `enp0s31f6` | **erledigt** (2026-11-02): Unter Windows heißen sie `\Device\NPF_{GUID}`. `MyDevice::display_name` zeigt dort die Beschreibung (Adapterliste, Übersicht, laufende Aufzeichnung, MCP `source_name`), intern bleibt der Name. `--adapter` und gespeicherte Einstellungen finden einen Adapter auch über seine Beschreibung (`MyDevice::is_called`, ohne Groß-/Kleinschreibung) |
| Start ohne Npcap | `wpcap.dll` wurde beim Laden gebraucht, ohne Npcap startete das Programm gar nicht | **erledigt** (2026-11-02): `build.rs` linkt `openpacketmon.exe` mit `/DELAYLOAD:wpcap.dll` (plus `delayimp.lib`); `src/npcap.rs` prüft vor dem Fenster, ob `%SystemRoot%\System32\wpcap.dll` da ist, sonst Dialog mit Angebot, `https://npcap.com/#download` zu öffnen, und Ende. Der Helfer lädt `wpcap.dll` normal, er startet nur, wenn Npcap da ist |
| C-Laufzeit | `VCRUNTIME140.dll` und `api-ms-win-crt-*` wurden gebraucht | **erledigt** (2026-11-02): `.cargo/config.toml` linkt sie für `x86_64-pc-windows-msvc` statisch (`+crt-static`), der Installer braucht kein Visual-C++-Paket. `dumpbin /dependents` zeigt nur noch Systembibliotheken und `wpcap.dll` |
| Sprache beim ersten Start | `language_of` liest `LC_ALL`, `LC_MESSAGES`, `LANG` | **erledigt** (2026-11-02): Unter Windows gibt es die Variablen meist nicht (nur Shells wie Git Bash setzen sie). Sind sie leer, nimmt `language_of` die Anzeigesprachen von Windows in der Reihenfolge des Benutzers (Crate `sys-locale` 0.3.2, MIT OR Apache-2.0, nur unter Windows, ohne weitere Abhängigkeiten). Geprüft: Npcap-Dialog ohne `LANG` auf Deutsch („Npcap fehlt“, Ja/Nein), Test `windows_names_the_languages_of_the_user`. Eine in den Einstellungen gewählte Sprache gilt weiter zuerst |
| Tests und echte Einstellungen | `tests/mcp.rs` setzt `HOME` und `XDG_CONFIG_HOME` auf einen Testordner | **offen** (gefunden 2026-11-02): `confy` fragt unter Windows den Ordner bei Windows selbst ab und beachtet die Variablen nicht. Die MCP-Integrationstests lesen dort also die echte `mcp-access.json`; läuft gleichzeitig ein Fenster mit eingeschaltetem Zugang, antwortet es den Tests, und ein Test scheitert (einmal beobachtet). Nur lesend, aber nicht abgeschottet |
| Latenzmessung (`surge-ping`) | unter Linux unprivilegiert | ICMP-Sockets ohne Adminrechte gibt es unter Windows vermutlich nicht: prüfen, sonst Hinweis in der Oberfläche |
| Programme je Verbindung (`listeners`) | hat einen Windows-Teil | prüfen, auch die Kürzung auf 15 Zeichen in unserem Code (betrifft nur Linux) |
| Doku-Texte | „Linux“, `setcap`, `systemctl`, `/usr/share/doc/…` | Oberfläche und README prüfen |
| `src/config/export_pcap.rs` | Standardordner aus `HOME` | `HOME` fehlt unter Windows: `USERPROFILE` bzw. den Dokumente-Ordner nehmen |
| `src/mcp/access.rs` `access_path`, Schreiben von `mcp-access.json` | Rechte `0600` nur unter `#[cfg(unix)]` | Unter Windows erben Dateien in `%APPDATA%` normalerweise die Rechte des Benutzerprofils (Benutzer, SYSTEM, Administratoren). **Geprüft** (2026-11-02, `icacls`): nur SYSTEM, Administratoren und der Benutzer, keine Änderung nötig (Abschnitt 5 Punkt 9). Der gegenseitige Nachweis (HMAC, `src/mcp/protocol.rs`) schützt zusätzlich, der Token geht nie über die Leitung |
| `src/mcp/` Tür auf `127.0.0.1` | unter Linux erreichbar für alle lokalen Benutzer, abgesichert durch den Nachweis | gleich; prüfen, ob die Windows-Firewall beim Lauschen auf `127.0.0.1` nachfragt (sollte sie nicht) |
| Adapter ohne IP-Verkehr (`TrafficPreview::no_ip`, `CaptureError::NoIp`, 2026-11-02) | Bluetooth, NFLOG, NFQUEUE grau | unter Npcap prüfen, welche Adapter einen nicht unterstützten Link-Typ melden (z. B. Bluetooth, WAN Miniport) |

Das Programm-Icon bettet `build.rs` mit `winresource` ein, wie bei Sniffnet, aber mit eigenem `.ico` (nicht das `sniffnet.ico` des
Originals); siehe Tabelle. Der Helfer `openpacketmon-capture.exe` hat kein Icon, er läuft ohne Fenster.

### Fenster (`src/gui/`)

egui, wgpu und `rfd` laufen unter Windows, der Code dort ist bisher aber nur unter Linux gelaufen. Zu prüfen: Dateidialoge, Ton (`rodio` nutzt dort WASAPI statt ALSA),
`LANG`-Abfrage in `language_of` und Verhalten beim Schließen. Die Konsole ist erledigt (Tabelle oben).

**Grafikschnittstelle (2026-11-02):** Mit den Vorgaben von egui nahm wgpu unter Windows Vulkan, und das Fenster blinkte, sobald die
Maus darüber fuhr (Intel Arc Graphics, Treiber 32.0.101.9150). Vom Nutzer verglichen mit `WGPU_BACKEND`: Nur Vulkan blinkt, DirectX 12
und OpenGL nicht. Seitdem gibt `wgpu_options` in `src/gui/mod.rs` unter Windows DirectX 12 vor, sonst OpenGL; Vulkan nicht mehr.
`WGPU_BACKEND` (`dx12`, `vulkan`, `gl`) entscheidet weiter, wenn es gesetzt ist, etwa zur Fehlersuche bei Testern. Linux unverändert.
Geprüft: Die Release-Fassung lädt `d3d12.dll`, nicht `vulkan-1.dll`.

Die Trennung der Rechte bleibt: Unter Linux hat nur der Helfer `openpacketmon-capture` die Mitschnittrechte und gibt sie nach dem
Öffnen des Adapters ab, das Programm läuft ohne. Unter Windows entscheidet die Installation von Npcap, ob normale Benutzer
mitschneiden dürfen oder nur Administratoren. Das Programm soll nie als Administrator laufen müssen.

### Helfer erbt offene Dateien (gefunden 2026-11-02, gelöst mit „Stop wartet“)

libpcap öffnet PCAP-Dateien unter Windows mit `fopen` der C-Laufzeit, solche Handles sind **vererbbar**. Rust startet Kindprozesse
unter Windows mit Handle-Vererbung (für die Pipes), also erbt jeder Helfer alle in dem Moment offenen vererbbaren Handles. Nach
`Engine::stop` startet die Adapter-Vorschau sofort einen `openpacketmon-capture.exe` je Adapter, während der Import-Thread die Datei
noch offen hält (`parse_packets` wartet nach dem Dateiende einige Sekunden auf Namensauflösungen und prüft dabei nicht, ob gestoppt
wurde). Folge: **Nach dem Stoppen eines PCAP-Imports bleibt die Datei gesperrt** (Löschen, Umbenennen, Überschreiben), bis ein neuer
Mitschnitt beginnt oder das Programm schließt; der Helfer bekommt Handles, die ihn nichts angehen. Nachgewiesen mit
`pcap_import_fills_the_state`: Die Datei blieb über 60 s gesperrt; 0,2 s nach dem Beenden der Helfer ließ sie sich löschen. Gleiches
droht der beim Mitschnitt geschriebenen PCAP-Datei.

Mögliche Lösungen (Nutzer, 2026-11-02: erst notieren; danach entschieden: **Stop wartet**, ohne `unsafe`):
- **Nur die Pipes vererben:** Helfer über `CreateProcessW` mit `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` starten. Behebt die Ursache
  grundsätzlich, braucht aber `unsafe` (Workspace hat `unsafe_code = "forbid"`, laut `PLAN.md` auch für den Helfer), etwa in einer
  kleinen eigenen Crate nur für Windows. `windows-sys` 0.61 steckt schon im Lockfile. Rusts sichere Fassung
  (`CommandExt::spawn_with_attributes`) ist in 1.99 noch unstabil (`windows_process_extensions_raw_attribute`).
- **Stop wartet auf den Import:** `parse_packets` bricht das Warten auf Namensauflösungen beim Stoppen ab, `Engine::stop` startet die
  Vorschau erst, wenn der Mitschnitt-Thread seine Dateien geschlossen hat. Ohne `unsafe`, deckt aber nur diesen Ablauf ab.

**Umgesetzt (2026-11-02):** `spawn_capture_thread` gibt einen `CaptureEnded`-Empfänger zurück, dessen Sender im Mitschnitt-Thread
und im Thread von `packet_stream` lebt; er meldet sich, sobald beide beendet sind und ihre Dateien geschlossen haben. `packet_stream`
hört nach dem Dateiende auf, und das Warten auf Namensauflösungen in `parse_packets` endet, sobald gestoppt wurde (prüft alle 100 ms).
`Engine::stop` wartet darauf bis zu 2 s (`CAPTURE_END_TIMEOUT`), erst dann startet die Vorschau ihre Helfer. Geprüft mit dem
ignorierten Test `engine::tests::the_imported_file_is_free_right_after_stopping` (startet echte Helfer, Stopp mitten im Lesen und nach
dem Dateiende; allein laufen lassen: `cargo test --bin openpacketmon the_imported_file -- --ignored`): ohne die Änderung „still locked“
(Fehler 32), mit ihr grün. Die Tests räumen jetzt auf (Abschnitt oben). **Grenze:** Andere offene vererbbare Handles (etwa eine PCAP-Datei,
die gerade beim Mitschnitt geschrieben wird) erbt ein Helfer weiterhin, wenn er in dem Moment startet; die grundsätzliche Lösung bleibt
die Handle-Liste mit `unsafe`, falls das einmal stört.

### Was es unter Windows nicht gibt

- **`setcap` und `postinst`:** Das übernimmt Npcap.
- **`.deb`:** ersetzt durch einen Installer (Abschnitt 6).

## 5. Was unter Windows getestet werden muss

1. `cargo test --workspace`: **erledigt** (2026-11-02, grün, Einzelheiten oben). `capture/tests/live.rs` prüft unter Windows den
   Helfer auf `\Device\NPF_Loopback` (`captures_on_the_npcap_loopback_adapter`, mit Wachhund, der den Helfer nach 60 s beendet).
2. Start ohne Npcap: **erledigt** (2026-11-02): verzögertes Laden von `wpcap.dll` und Dialog (Tabelle in Abschnitt 4). Geprüft mit
   einem falschen `SystemRoot`: Dialog „Npcap is missing“ mit Ja/Nein, kein Fenster, keine Helfer. Ohne echtes Deinstallieren von Npcap
   nicht geprüft; das gehört zum Test auf einem sauberen System (Punkt 8).
3. Mitschnitt auf WLAN, Ethernet und dem Npcap-Loopback-Adapter, jeweils als normaler Benutzer und als Administrator. Der Helfer
   allein ist auf dem Loopback-Adapter geprüft (Punkt 1, Link-Typ 0 = Null/Loopback); im Fenster noch nicht.
4. Zerleger und Paketliste: Link-Typen unter Windows (kein „Linux cooked capture“), TCP-Analyse mit Offloading (große
   Segmente).
5. Programme je Verbindung, Latenzmessung, Geodaten-Download, Benachrichtigungen mit Ton, PCAP-Download und -Import.
6. IPFIX-Collector: Die Windows-Firewall fragt nach, weil er auf `0.0.0.0:4739` lauscht. Die Oberfläche auf `127.0.0.1` löst
   keine Nachfrage aus.
7. Sicherheit: Der Sicherheitstest aus `PLAN.md` 4.8 für die geänderten Stellen, vor allem die Rechte des Helfers.
8. Sauberes System: Installer, Deinstallation, Update über eine ältere Version.
9. MCP-Anbindung (`PLAN.md` 5.4): `openpacketmon.exe mcp` mit Claude Code unter Windows. Dabei prüfen, dass nur der Benutzer `mcp-access.json` im
   Ordner der Einstellungen lesen kann: Die Rechte `0600` setzt der Code nur unter Unix. **Geprüft** (2026-11-02, installierte Fassung
   aus dem Installer, Mitschnitt auf einem USB-Ethernet-Adapter, Zugang im Fenster eingeschaltet): Über `openpacketmon.exe mcp` antworten
   `capture_status` (Adapter mit Beschreibung), `overview` (Hosts mit Geodaten, Programme wie `chrome.exe`), `statistics` (Protokolle,
   Befunde, TCP-Handshake-Zeiten), `connections`, `packets` mit Filtern (`dns`, `tls.sni`) und `packet_detail`; `follow_stream` und
   `packet_bytes` lehnen ohne „Paketinhalte erlauben“ ab wie vorgesehen. `mcp-access.json` in `%APPDATA%\openpacketmon\config` erbt
   die Rechte des Profils: nur SYSTEM, Administratoren und der Benutzer selbst (`icacls`), also wie `0600` unter Linux, wo root ebenfalls
   lesen kann; keine Änderung nötig. **Mit „Paketinhalte erlauben“** (gleich danach): eigene HTTP-Anfrage an `example.com` mit
   erfundenem Cookie, `Authorization: Bearer …` und Token in der URL-Query. Liste zeigt `GET /test?[hidden]`; `follow_stream` (Text und
   Hex), `packet_detail` und `packet_bytes` liefern die Anfrage mit `x` statt der Werte (`Cookie: xxx…`, `Authorization: xxxxxx xxx…`,
   `/test?xxx…`), die Geheimnisse kommen weder als Text noch als Hex-Bytes vor; die Antwort des Servers kommt vollständig. Mit echtem
   Claude Code als Client unter Windows noch nicht.

## 6. Installer und Release

**Entschieden (Nutzer, 2026-11-02):** Die Tester bekommen **gleich einen Installer**, zunächst **unsigniert** (SmartScreen warnt
dann; „Weitere Informationen → Trotzdem ausführen“). Bis zum 2026-11-02 bekamen Tester nur die Installer-Datei
(Haltepunkt aus `PLAN.md` 4.10); seitdem ist das Repo öffentlich, ein Release mit Installer folgt. Der
Installer nennt Npcap und dessen Download, enthält es aber nicht. Die kostenlose Npcap-Lizenz erlaubt den privaten Einsatz
auf bis zu fünf Rechnern; beim Weitergeben an Tester darauf hinweisen.

**Gewählt (Nutzer, 2026-11-02): Inno Setup** (eine `.exe`, einfacher als WiX). Umgesetzt:
- `resources/packaging/windows/openpacketmon.iss`: feste `AppId`, Deutsch und Englisch, **standardmäßig nur für den aktuellen
  Benutzer** ohne Administratorrechte (`PrivilegesRequired=lowest`), wahlweise für alle (Frage am Anfang). Installiert beide `.exe`,
  `LICENSE-MIT.txt`, `LICENSE-APACHE.txt`, `NOTICE.txt` und `THIRD-PARTY-LICENSES.txt`, legt einen Startmenü-Eintrag an und auf Wunsch ein
  Desktop-Symbol. Fehlt `{sys}\wpcap.dll`, zeigt eine eigene Seite den Hinweis auf Npcap mit einem Knopf zur Downloadseite, und der
  Haken „openpacketmon starten“ entfällt. Beim Deinstallieren bleiben die Einstellungen in `%APPDATA%\openpacketmon`.
- `dev/build-installer.ps1`: liest die Version aus `Cargo.toml` (muss gleich `capture/Cargo.toml` sein), baut mit
  `cargo build --release --locked --workspace`, prüft `THIRD-PARTY-LICENSES.txt`, ruft `ISCC.exe` und gibt Pfad und SHA-256 von
  `target\installer\openpacketmon-<version>-setup.exe` aus. Aufruf: `powershell -ExecutionPolicy Bypass -File dev\build-installer.ps1`.
  Die Version nennt der Nutzer, wie beim `.deb`.
- Lokal geprüft (2026-11-02, Installer mit der unveränderten Version 0.3.10, nur zum Testen): stille Installation mit `/CURRENTUSER`
  in einen Testordner, Dateien, Startmenü-Eintrag und Eintrag unter „Installierte Apps“ (openpacketmon 0.3.10) da, Programm startet
  aus dem Installationsordner; stille Deinstallation entfernt Ordner, Eintrag und Startmenü, die Einstellungen bleiben.
  **Noch nicht** auf einem sauberen System (Abschnitt 5 Punkt 8; in der Windows-Sandbox bisher nur stille Installation und
  Signaturen, 6.1) und nicht von Hand durchgeklickt.
- **Erste Tester-Fassung 0.3.11** (2026-11-02, Version vom Nutzer): `target\installer\openpacketmon-0.3.11-setup.exe`, 10 179 802 Bytes,
  SHA-256 `d36c0f589da73f98d57dd12f7431deb973f45a4916280799af2ed17174753b0e`, unsigniert. Weitergabe nur als Datei (damals galt der Haltepunkt 4.10).
- `docs/windows-tester.md`: Anleitung für Tester (Npcap mit WinPcap-Modus, Smart App Control, SmartScreen, was ausprobieren,
  Rückmeldung, Deinstallation).

- Sniffnet baute ein MSI mit WiX: `resources/packaging/windows/setup.wxs`, `EULA.rtf` und Grafiken im Original bei Commit
  `ab4bf739d464` (bei uns entfernt, siehe `PLAN.md` Phase 2). Wird davon etwas übernommen, gilt die Kennzeichnungspflicht aus
  `CLAUDE.md`, und Sniffnet-Logo, -Grafiken und -Namen müssen raus.
- Der Installer darf Npcap nicht enthalten, er nennt nur den Download.
- **Code-Signatur:** Der Installer soll, wenn möglich, signiert werden (Nutzer, 2026-10-30); ohne Signatur warnt Windows
  SmartScreen, und **Smart App Control blockiert ihn ganz**. Die Tester haben Smart App Control an (Nutzer, 2026-11-02), können
  0.3.11 also nicht installieren. Die Wege und der Stand stehen in Abschnitt 6.1.
- `dev/release.sh` nimmt seit 2026-11-03 den signierten Installer mit (Abschnitt 6.1, „Release“); gebaut und signiert wird er
  weiter hier mit `dev\build-installer.ps1`.
- Die Update-Prüfung vergleicht nur Versionsnummern. Der Hinweis muss unter Windows auf den Installer zeigen.
- `THIRD-PARTY-LICENSES.txt` enthält jetzt auch die Windows-Crates, siehe Abschnitt 7.

### 6.1 Code-Signatur

Recherche vom 2026-10-30, ergänzt 2026-11-02; vor der Umsetzung erneut prüfen. Auch mit Signatur kann SmartScreen anfangs warnen,
bis der Installer genug Reputation gesammelt hat. Smart App Control dagegen lässt ein Programm mit gültiger Signatur einer
vertrauenswürdigen Zertifizierungsstelle laufen.

**Stand (Nutzer, 2026-11-02):** Smart App Control blockiert den unsignierten Installer bei den Testern. Die kostenlosen
Wege (SignPath, Certum Open Source) verlangen ein öffentlich sichtbares Projekt; deshalb hat der Nutzer den Haltepunkt aus
`PLAN.md` 4.10 am 2026-11-02 aufgehoben und das Repo öffentlich geschaltet. Nächster Schritt: Antrag stellen. Ein normales OV-Zertifikat ist dem Nutzer **zu teuer** für Open-Source-Software und wird
nicht weiter verfolgt.

| Weg | Für uns | Bedingungen und Haken |
|---|---|---|
| **SignPath Foundation** (kostenlos für Open Source, https://signpath.org/terms.html) | **erster Versuch nach dem Haltepunkt, Zusage nicht sicher** | Siehe unten |
| **Certum Open Source Code Signing** (https://shop.certum.eu/code-signing.html) | **Rückfall nach dem Haltepunkt** | Ab 49 € (Cloud-Signatur SimplySign) oder 69 € (mit Karte und Leser), nur für Privatpersonen, im Zertifikat steht „Open Source Developer“. Certum verlangt die Adresse eines öffentlich auffindbaren Projekts, das die Verbindung zum Antragsteller zeigt, und stellt sonst nichts aus (https://www.support.certum.eu/en/required-documents/). Gültig höchstens 459 Tage **Geprüft 2026-11-02** (Shop und https://support.certum.eu/en/code-signing-required-documents/, Stand 2026-06-18): Cloud 49 €, Satz mit Karte und Leser 69 €, nur Aktivierungscode für eigene Karte 25 €; nur für natürliche Personen, im Zertifikat Name mit „Open Source Developer“, Organisation „Open Source Developer“; Identität automatisch online, per Notar, Registrierungsstelle oder Fotos mit Ausweis (beide Seiten); dazu eine **Versorgerrechnung** (Strom, Gas, Wasser, Telefon) auf den eigenen Namen und die Projektadresse, aus der die Verbindung zum Antragsteller klar hervorgeht; Unterlagen per Upload im Certum-Konto. Cloud: SimplySign-App (Android/iOS) für Zugangscodes, SimplySign Desktop, höchstens 5000 Signaturen im Monat; laut Shop auch Signieren mit Werkzeugen unter Linux/macOS. Bei kommerziellem Vertrieb wird das Zertifikat widerrufen. Die Produktseite der Cloud-Variante zeigte beim Abruf „Product is out of stock“, die Seitendaten aber „in stock“: im Shop prüfen |
| **OV-Zertifikat** einer Zertifizierungsstelle | **abgelehnt** (Nutzer, 2026-11-02: zu teuer) | Etwa Certum Standard 169 € mit Karte, 209 € in der Cloud, jährlich |
| **Azure Artifact Signing** (früher Trusted Signing) | **kommt nicht infrage** | Laut Microsoft-Quickstart (https://learn.microsoft.com/azure/artifact-signing/quickstart) für Einzelentwickler nur in USA und Kanada, Organisationen auch in der EU. Ab 9,99 $ im Monat, braucht ein bezahltes Azure-Abo |
| **Microsoft Store** (versteckter Eintrag, MSIX) | nicht weiter verfolgt | Microsoft signiert das Paket, verlangt aber Umbau auf MSIX und eine Prüfung im Store; ein Store-Eintrag ist schon eine Form der Veröffentlichung (Haltepunkt) |

Smart App Control bei den Testern auszuschalten, ist kein Ausweg: Seit dem April-Update 2026 lässt es sich ohne Zurücksetzen von
Windows wieder einschalten, nach KB5127648 (Oktober 2026) bleibt der Schalter bei manchen aber ausgegraut; und wieder
eingeschaltet blockiert es das Programm trotzdem.

**Zertifikat (2026-11-03):** Certum „Open Source Code Signing in the Cloud“ ausgestellt und aktiviert, `CN=Open Source Developer
<Name>`, ausgestellt von „Certum Code Signing 2021 CA“, RSA 3072, gültig bis 2027-11-03, Fingerabdruck
`5A5AE6506A7649F1FFBD9856FB22E63E39D98C17`. Die Zertifikatsdateien (`.cer`, `.pem`) enthalten nur den öffentlichen Teil und gehören
nicht ins Repo; der Schlüssel bleibt bei Certum.

**Erster signierter Bau (2026-11-03):** `openpacketmon-0.4.0-setup.exe` mit der Version aus `Cargo.toml` (0.4.0, schon als Linux-Release
veröffentlicht), SHA-256 `cea6950721d2c2e9ab68d4307aa602f2ebfd78a12db541d765762b3ebf3b00cf`. `signtool verify /pa /all` bestätigt
`openpacketmon.exe`, `openpacketmon-capture.exe` und den Installer (SHA-256, Zeitstempel RFC 3161). Der Deinstaller wird von Inno
Setup im Installer signiert und lässt sich erst nach einer Installation prüfen; das steht noch aus, ebenso der Test bei einem Tester
mit Smart App Control. **Test bestanden (Nutzer, 2026-11-03).** Am selben Tag zum bestehenden Release `v0.4.0` hinzugefügt
(dazu neue `SHA256SUMS`, Release-Text mit Abschnitt Windows). Der Programmcode ist derselbe wie beim Tag `v0.4.0`; danach haben sich
nur `dev\build-installer.ps1` und `openpacketmon.iss` (Signatur) geändert.

**Signieren im Skript (vorbereitet 2026-11-02, mit dem Zertifikat am 2026-11-03 geprüft):** `dev\build-installer.ps1 -CertSha1 <Fingerabdruck>`
(oder Variable `OPENPACKETMON_SIGN_SHA1`) signiert `openpacketmon.exe` und `openpacketmon-capture.exe` vor dem Verpacken, Inno Setup
signiert Installer und Deinstaller (`#ifdef Sign` in `openpacketmon.iss`: `SignTool=openpacketmon`, `SignedUninstaller=yes`, der
Befehl kommt als `/Sopenpacketmon=…` von ISCC). Befehl wie in Certums Anleitung „Code Signing in the Cloud – Using Signtool“:
`signtool sign /sha1 <Fingerabdruck> /tr http://time.certum.pl /td sha256 /fd sha256`; danach `signtool verify /pa /all` für alle drei
Dateien. Das Skript sucht `signtool.exe` im `PATH` oder im neuesten Windows SDK (`%ProgramFiles(x86)%\Windows Kits\10\bin\<Version>\x64`)
und hält an, wenn das Zertifikat nicht im Speicher des Benutzers (`Cert:\CurrentUser\My`) liegt oder abgelaufen ist. Ohne
Fingerabdruck bleibt alles unsigniert, mit Hinweis am Ende.

Einrichten auf dem Windows-Rechner (so beim ersten Mal am 2026-11-03 durchlaufen):
1. Aktivieren: Certum schickt eine E-Mail mit Link; auf der Seite dahinter den Aktivierungscode aus dem Certum-Konto eingeben, dann
   erscheint ein QR-Code. In der SimplySign-App auf dem Handy **„Andere Aktivierungsmethoden“ → „QR-Code scannen“** wählen (nicht
   „Zugang wiederherstellen“, das endet mit „invalid token“) und als Modus „Signieren und Codes erzeugen“ nehmen.
   SimplySign Desktop installieren, dann über das Symbol im Infobereich „Connect to SimplySign“: E-Mail-Adresse des Certum-Kontos
   und ein Code aus der App. Erst danach erscheint das Zertifikat mit privatem Schlüssel im Zertifikatspeicher des Benutzers; vorher
   läuft SimplySign Desktop zwar, aber ohne Zertifikat. Nach einer Weile meldet es sich ab, dann mit neuem Code wieder anmelden.
2. Fingerabdruck ablesen: `Get-ChildItem Cert:\CurrentUser\My | Where-Object Subject -like '*Open Source Developer*' | Format-List Subject, Thumbprint, NotAfter`.
3. `signtool` vorhanden? Kommt mit dem Windows SDK (Visual Studio „Desktopentwicklung mit C++“), sonst
   `winget install Microsoft.WindowsSDK.10.0.26100`.
4. `powershell -ExecutionPolicy Bypass -File dev\build-installer.ps1 -CertSha1 <Fingerabdruck>`; SimplySign Desktop muss dabei
   angemeldet sein.
5. Prüfen: Eigenschaften → Digitale Signaturen beim Installer, beim installierten `openpacketmon.exe` und beim Deinstaller; dann
   bei einem Tester mit Smart App Control installieren. Dabei auch prüfen, ob Smart App Control die Datei durchlässt, die der
   Inno-Installer beim Start in den Temp-Ordner entpackt.

**Deinstaller in der Windows-Sandbox geprüft (2026-11-03):** Der veröffentlichte `openpacketmon-0.4.0-setup.exe` wurde in einer frischen
Windows-Sandbox still installiert (`/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /CURRENTUSER`, Exitcode 0, kein Neustart nötig), danach
`Get-AuthenticodeSignature` auf Installer und installierte Dateien. Installer, `openpacketmon.exe`, `openpacketmon-capture.exe` und
`unins000.exe` (alle unter `%LOCALAPPDATA%\Programs\openpacketmon`): „Valid“, Signierer „Open Source Developer Thomas Waldemar
Cieslik“, Aussteller „Certum Code Signing 2021 CA“, Zeitstempel „Certum Timestamp 2026“. Die Sandbox muss der Nutzer einmal
einschalten (Windows-Feature „Windows-Sandbox“, Administrator und Neustart); danach genügt eine `.wsb`-Datei mit dem Installer in
einem nur lesbaren und einem beschreibbaren Ordner und einem Prüfskript als `LogonCommand`. Nicht geprüft: Deinstallation,
Update über eine ältere Version, Start ohne Npcap (Abschnitt 5 Punkt 8).

**Release (2026-11-03):** Den Installer baut und signiert der Windows-Rechner, das `.deb` der Linux-Rechner. `dev/release.sh` hat dafür zwei Wege:
- `dev/release.sh --add-windows target/installer/openpacketmon-X.Y.Z-setup.exe --publish` in Git Bash hier, **nachdem** das Release
  unter Linux erstellt ist: baut nichts, prüft Dateiname (Version aus `Cargo.toml`), Signatur (`signtool verify /pa /all`) und dass sich
  seit dem Tag nur Doku, `dev/`, `resources/packaging/` und `.github/` geändert haben; dann Installer hoch, `SHA256SUMS` um seine
  Zeile ergänzt und ersetzt, alles wieder geladen und gegen `SHA256SUMS` geprüft. Ohne `--publish` nur die Prüfungen. Den Release-Text
  um Windows ergänzen (`gh release edit vX.Y.Z --notes-file …`). Geprüft 2026-11-03 ohne `--publish` an `v0.4.0`, auch mit falscher
  Version und mit unsignierter Datei (beide abgewiesen).
- `dev/release.sh --windows openpacketmon-X.Y.Z-setup.exe --publish --notes …` unter Linux, wenn der Installer vorher dorthin kopiert
  wurde: kommt mit ins neue Release und in `SHA256SUMS`; die Signatur prüft dort `osslsigncode` (`sudo apt install osslsigncode`).
  Noch nicht ausprobiert. Dass der Installer aus demselben Commit gebaut ist, prüft das Skript in diesem Weg nicht.

Was SignPath verlangt und was das für uns heißt:

- **Bereits veröffentlicht:** Das Projekt muss schon in der Form veröffentlicht sein, die signiert werden soll. Es braucht also
  ein öffentliches Repo und zuerst ein **unsigniertes Windows-Release**. SignPath verlangt außerdem eine nachprüfbare Reputation.
- **Build in der CI:** Die Binärdateien müssen nachprüfbar aus dem Quellcode entstehen. Der Windows-Build muss also in GitHub
  Actions laufen, nicht auf dem eigenen Rechner. Jedes Release wird von Hand freigegeben.
- **Kein Hacking-Werkzeug:** Verboten sind Funktionen, die Sicherheitslücken finden oder ausnutzen, auch Diagnosewerkzeuge, die
  aktiv nach ausnutzbaren Schwachstellen suchen. openpacketmon sucht nicht nach Lücken, sondern zeigt Verkehr. Trotzdem ist es ein
  Paketmitschnitt-Programm, und SignPath entscheidet ohne Einspruchsmöglichkeit. **Hier kann die Ablehnung kommen.**
- **Herausgeber:** Im Zertifikat steht „SignPath Foundation“, nicht der Name des Projekts oder des Nutzers.
- **Keine proprietären Teile:** Npcap liegt nicht bei, das passt. Alle Komponenten brauchen eine OSI-Lizenz.
- **Website:** Auf Startseite und Download-Seite ein Abschnitt „Code signing policy“ mit dem Satz aus den Bedingungen, Rollen
  und Datenschutzangabe. Die Update-Prüfung ist in der README beschrieben und abschaltbar; ob das reicht, beim Antrag klären.
- **Konten:** Mehr-Faktor-Anmeldung für SignPath und GitHub.

## 7. Prüfwerkzeuge und CI

- `deny.toml`: **erledigt** (2026-11-02): `targets` um `x86_64-pc-windows-msvc` ergänzt, `BSL-1.0` erlaubt (nur `clipboard-win` und
  `error-code`, die Zwischenablage von egui unter Windows).
- `dev/third-party-licenses.py`: läuft auch unter Windows und schreibt dort dieselbe Datei (Pfade mit `/`, Zeilenenden LF).
  **Erledigt** (2026-11-02): **eine gemeinsame Liste** für Linux und Windows (`TARGETS`, Vereinigung beider Abhängigkeitsbäume, 482
  statt 452 Crates), mit Hinweisen, dass Npcap nicht beiliegt und die C-Laufzeit von Microsoft unter Windows statisch gelinkt ist.
  `winresource` steht nicht darin: Es ist nur Build-Abhängigkeit unter Windows und wird nicht ins Programm übersetzt.
- `dev/*.sh` laufen in Git Bash. `dev/build-deb.sh` und `dev/release.sh` sind für Linux gedacht; nur `dev/release.sh --add-windows` läuft auch hier.
- `.github/workflows/ci.yml`: einen Job auf `windows-latest` ergänzen, mit Npcap SDK (Sniffnet lädt `npcap-sdk-1.16.zip` von
  npcap.com und setzt `LIB`). Die Sniffnet-CI installiert Npcap per OEM-Installer über ein Secret, das wir nicht haben, und
  lässt Windows ohne das Secret bei Pull Requests aus. Bei uns ist also zuerst `fmt`, `check` und `clippy` realistisch. Ob
  `cargo test` in der CI ohne Npcap läuft, muss sich zeigen.

## 8. Reihenfolge

1. Einrichten (Abschnitt 1 bis 3), `cargo check --workspace` und die echte Fehlerliste mit Abschnitt 4 abgleichen. **Erledigt** (2026-11-02).
2. Programm bauen und starten, ein Mitschnitt mit Npcap. **Teilweise:** Debug- und Release-Fassung bauen, das Fenster startet ohne
   Konsole und mit Icon, der Helfer schneidet im Test mit; Mitschnitt im Fenster vom Nutzer getestet (2026-11-02).
3. `cargo test`, Pfade und Rechte. **Tests grün** (2026-11-02), Helfer erbt offene Dateien gelöst; Rechte von `mcp-access.json` geprüft (Abschnitt 5 Punkt 9). Offen: MCP-Integrationstests
   nicht von den echten Einstellungen getrennt (Abschnitt 4).
4. Windows-Eigenheiten aus Abschnitt 5. **Teilweise:** Adapternamen, Start ohne Npcap, C-Laufzeit, Npcap-Timeout erledigt.
5. CI, Lizenzliste, `deny.toml`. **Lizenzliste und `deny.toml` erledigt**, CI-Job für Windows offen.
6. Installer (unsigniert), Test auf sauberem System, README; dann **Weitergabe an die Tester** als Datei (Abschnitt 6). **Installer
   und Tester-Anleitung erledigt**, erste Tester-Fassung 0.3.11 gebaut; offen: Test auf sauberem System, README, MCP-Einrichtung
   (`claude mcp add …`) in `docs/windows-tester.md`. README hat seit 2026-11-03 einen Abschnitt Windows. **Signierter Installer
   0.4.0 im Release `v0.4.0`** (2026-11-03).
7. Signatur beantragen (Abschnitt 6.1), danach signierte Installer. **Erledigt** (2026-11-03): Certum Open Source, Cloud; mit Smart
   App Control getestet. `dev/release.sh` nimmt den Installer seit 2026-11-03 mit.

**Beginn (Nutzer, 2026-11-02):** Windows wird **vor** dem ersten Release gemacht, weil andere Tester eine Windows-Fassung
brauchen; MCP ist fertig. Der Haltepunkt aus `PLAN.md` 4.10 galt auch auf dem Windows-Rechner; er ist seit 2026-11-02 aufgehoben. Die Abschnitte 4 und 5 stammen aus der Zeit vor MCP: Beim Beginn die Liste um das ergänzen, was
seitdem dazugekommen ist.
