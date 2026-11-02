# Windows-Version von openpacketmon

Stand: 2026-10-30, ergänzt 2026-11-02 (MCP, Mitschnitt-Helfer, vorgezogener Beginn, Installer für Tester, Erfahrungen beim Einrichten des Windows-Rechners). Diese Datei beschreibt, was nötig ist, um das Projekt auf einem Windows-Rechner fortzusetzen und dort eine
Windows-Version zu bauen und zu testen.

**Heutiger Stand (2026-11-02):** Im Zweig `windows` baut die **Debug-Fassung** unter Windows, die Tests sind grün, und das Fenster
startet (kurzer Start: Titel „openpacketmon“, je Adapter ein Helfer für die Vorschau, nach dem Beenden keine Helfer übrig). Bedient,
mitgeschnitten und importiert wurde darin noch nichts. **Die Release-Fassung baut noch nicht** (`gag`, Abschnitt 4), es gibt kein
Icon, keinen Installer, und es geht ein Konsolenfenster mit auf. Eine Fassung für Tester gibt es also noch nicht.
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
Windows richtig läuft (Abschnitt 4). Gebaut ist bisher nur die Debug-Fassung; `cargo build --release` scheitert erwartungsgemäß an
`redirect_stdout_stderr_to_file` (nur `cfg(all(windows, not(debug_assertions)))`, Crate `gag` fehlt in `Cargo.toml`).

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
  Lauf lassen sie sich löschen. **Ursache gefunden (2026-11-02), offen:** Es liegt nicht an den Tests, sondern an der
  Handle-Vererbung an den Mitschnitt-Helfer (Punkt „Helfer erbt offene Dateien“ in Abschnitt 4). Nebenbei gefunden: Die GUI-Tests
  löschen ihren Ordner `openpacketmon-gui-<pid>` nie (auch unter Linux nicht), und `engine::tests::pcap_import_fills_the_state` und
  `engine::packets::tests::packets_are_written_as_pcap` benutzen denselben Ordner `openpacketmon-test-<pid>`, obwohl sie gleichzeitig
  laufen können. Ein Versuch mit einem gemeinsamen Aufräum-Helfer (`remove_test_dir`, löscht mit Wiederholung bis 10 s und meldet
  übrig gebliebene Ordner als Fehler) ließ unter Windows 21 Tests scheitern, solange die Ursache besteht; zurückgenommen.
- Im zweiten Lauf zeigte sich `mcp::access::tests::turning_the_access_off_and_on_during_questions` (4 von 5 Läufen rot): Unter
  Windows wird eine Verbindung auf einen gerade geschlossenen Port von `127.0.0.1` nicht sofort abgewiesen, sondern hängt bis
  `CONNECT_TIMEOUT` (2 s); der Test wartete nach dem Umschalten nur 200 ms. **Behoben:** Er wartet jetzt bis zur ersten Antwort,
  höchstens 10 s. Im Programm heißt das nur: Nach dem Schließen des Fensters bekommt ein Assistent die Absage unter Windows 2 s später.
- Danach `cargo test --workspace` **grün** (386 bestanden, 5 übersprungen).

**Smart App Control** (auf diesem Rechner an) blockiert ab und zu frisch gebaute, unsignierte Dateien aus `target\`, auch Testdateien
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

Für den Installer kommt später WiX oder Inno Setup dazu (Abschnitt 6).

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
  (Arbeitsregeln) und `PLAN.md` 4.10 (Haltepunkt Sniffnet).

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
| `src/utils/formatted_strings.rs` `redirect_stdout_stderr_to_file` | nutzt die Crate `gag`, die nicht mehr in `Cargo.toml` steht, **baut im Release nicht** (2026-11-02 bestätigt: `gag` fehlt) | Funktion entfernen oder `gag` wieder aufnehmen; Log-Datei wird gebraucht, wenn es kein Konsolenfenster gibt |
| `src/mmdb/geo.rs` `geo_dir` | `HOME`, `.local/share` (`HOME` fehlt unter Windows) | `%LOCALAPPDATA%\openpacketmon\geo` (**erledigt** 2026-11-02) |
| `src/mmdb/geo.rs` `geoipupdate_file` | `/var/lib/GeoIP/…` | unter Windows weglassen oder den Ordner von `geoipupdate` nehmen, Test anpassen |
| `src/networking/traffic_preview.rs`, `capture_context.rs` | aus Sniffnet, schon für Windows vorbereitet | prüfen |
| `capture/src/main.rs` (Helfer) | unter Linux nicht blockierender Adapter mit eigenem `poll`, Rechteabgabe, größere Pipe | außerhalb von Linux liest er blockierend mit dem Timeout von libpcap, die drei Funktionen `drop_rights`, `enlarge_pipe`, `wait_for_packets` tun dort nichts. Prüfen: Kommt unter Npcap der Timeout auch ohne Pakete? Sonst bleibt eine pausierte Aufzeichnung stehen (siehe `open` in der Datei). Pfad mit `.exe` löst `helper_path` schon auf |
| Adapterliste, `--adapter`, Aufzeichnungsseite | Namen wie `enp0s31f6` | unter Windows heißen sie `\Device\NPF_{GUID}`: überall die Beschreibung zeigen (`get_desc`), den Namen nur intern verwenden |
| Latenzmessung (`surge-ping`) | unter Linux unprivilegiert | ICMP-Sockets ohne Adminrechte gibt es unter Windows vermutlich nicht: prüfen, sonst Hinweis in der Oberfläche |
| Programme je Verbindung (`listeners`) | hat einen Windows-Teil | prüfen, auch die Kürzung auf 15 Zeichen in unserem Code (betrifft nur Linux) |
| Doku-Texte | „Linux“, `setcap`, `systemctl`, `/usr/share/doc/…` | Oberfläche und README prüfen |
| `src/config/export_pcap.rs` | Standardordner aus `HOME` | `HOME` fehlt unter Windows: `USERPROFILE` bzw. den Dokumente-Ordner nehmen |
| `src/mcp/access.rs` `access_path`, Schreiben von `mcp-access.json` | Rechte `0600` nur unter `#[cfg(unix)]` | Unter Windows erben Dateien in `%APPDATA%` normalerweise die Rechte des Benutzerprofils (Benutzer, SYSTEM, Administratoren). Prüfen mit `icacls`; falls andere Benutzer lesen können, die Rechte beim Anlegen einschränken. Der gegenseitige Nachweis (HMAC, `src/mcp/protocol.rs`) schützt zusätzlich, der Token geht nie über die Leitung |
| `src/mcp/` Tür auf `127.0.0.1` | unter Linux erreichbar für alle lokalen Benutzer, abgesichert durch den Nachweis | gleich; prüfen, ob die Windows-Firewall beim Lauschen auf `127.0.0.1` nachfragt (sollte sie nicht) |
| Adapter ohne IP-Verkehr (`TrafficPreview::no_ip`, `CaptureError::NoIp`, 2026-11-02) | Bluetooth, NFLOG, NFQUEUE grau | unter Npcap prüfen, welche Adapter einen nicht unterstützten Link-Typ melden (z. B. Bluetooth, WAN Miniport) |

Der Build von Windows braucht in `build.rs` zusätzlich das Programm-Icon: Sniffnet nutzte dafür `winresource`. Bei uns wurde das
entfernt, es kommt für `openpacketmon.exe` mit einem eigenen `.ico` wieder dazu (aus
`openpacketmon_256x256.png` u. a. erzeugen, **nicht** das `sniffnet.ico` des Originals).

### Fenster (`src/gui/`)

egui, wgpu und `rfd` laufen unter Windows, der Code dort ist bisher aber nur unter Linux gelaufen. Zu prüfen: Dateidialoge, Ton (`rodio` nutzt dort WASAPI statt ALSA),
`LANG`-Abfrage in `language_of`, Konsole (`#![cfg_attr(windows, windows_subsystem = "windows")]`, sonst geht mit dem Fenster ein Konsolenfenster auf) und Verhalten beim Schließen.

Die Trennung der Rechte bleibt: Unter Linux hat nur der Helfer `openpacketmon-capture` die Mitschnittrechte und gibt sie nach dem
Öffnen des Adapters ab, das Programm läuft ohne. Unter Windows entscheidet die Installation von Npcap, ob normale Benutzer
mitschneiden dürfen oder nur Administratoren. Das Programm soll nie als Administrator laufen müssen.

### Helfer erbt offene Dateien (gefunden 2026-11-02, offen)

libpcap öffnet PCAP-Dateien unter Windows mit `fopen` der C-Laufzeit, solche Handles sind **vererbbar**. Rust startet Kindprozesse
unter Windows mit Handle-Vererbung (für die Pipes), also erbt jeder Helfer alle in dem Moment offenen vererbbaren Handles. Nach
`Engine::stop` startet die Adapter-Vorschau sofort einen `openpacketmon-capture.exe` je Adapter, während der Import-Thread die Datei
noch offen hält (`parse_packets` wartet nach dem Dateiende einige Sekunden auf Namensauflösungen und prüft dabei nicht, ob gestoppt
wurde). Folge: **Nach dem Stoppen eines PCAP-Imports bleibt die Datei gesperrt** (Löschen, Umbenennen, Überschreiben), bis ein neuer
Mitschnitt beginnt oder das Programm schließt; der Helfer bekommt Handles, die ihn nichts angehen. Nachgewiesen mit
`pcap_import_fills_the_state`: Die Datei blieb über 60 s gesperrt; 0,2 s nach dem Beenden der Helfer ließ sie sich löschen. Gleiches
droht der beim Mitschnitt geschriebenen PCAP-Datei.

Mögliche Lösungen, noch nicht entschieden (Nutzer, 2026-11-02: erst notieren):
- **Nur die Pipes vererben:** Helfer über `CreateProcessW` mit `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` starten. Behebt die Ursache
  grundsätzlich, braucht aber `unsafe` (Workspace hat `unsafe_code = "forbid"`, laut `PLAN.md` auch für den Helfer), etwa in einer
  kleinen eigenen Crate nur für Windows. `windows-sys` 0.61 steckt schon im Lockfile. Rusts sichere Fassung
  (`CommandExt::spawn_with_attributes`) ist in 1.99 noch unstabil (`windows_process_extensions_raw_attribute`).
- **Stop wartet auf den Import:** `parse_packets` bricht das Warten auf Namensauflösungen beim Stoppen ab, `Engine::stop` startet die
  Vorschau erst, wenn der Mitschnitt-Thread seine Dateien geschlossen hat. Ohne `unsafe`, deckt aber nur diesen Ablauf ab.

Danach die Tests aufräumen lassen (Abschnitt oben, `remove_test_dir`).

### Was es unter Windows nicht gibt

- **`setcap` und `postinst`:** Das übernimmt Npcap.
- **`.deb`:** ersetzt durch einen Installer (Abschnitt 6).

## 5. Was unter Windows getestet werden muss

1. `cargo test --workspace`: **erledigt** (2026-11-02, grün, Einzelheiten oben); `capture/tests/live.rs` prüft unter Windows nichts.
2. Start ohne Npcap: verständlicher Hinweis mit Link auf npcap.com, kein Absturz. Da `wpcap.dll` beim Laden des Programms
   gebraucht wird, startet das Programm ohne Npcap womöglich gar nicht; dann braucht es verzögertes Laden. Der Helfer allein löst
   das noch nicht: `openpacketmon` nutzt `pcap` weiter für die Adapterliste, für Aufnahmedateien und zum Schreiben von PCAP-Dateien.
3. Mitschnitt auf WLAN, Ethernet und dem Npcap-Loopback-Adapter, jeweils als normaler Benutzer und als Administrator.
4. Zerleger und Paketliste: Link-Typen unter Windows (kein „Linux cooked capture“), TCP-Analyse mit Offloading (große
   Segmente).
5. Programme je Verbindung, Latenzmessung, Geodaten-Download, Benachrichtigungen mit Ton, PCAP-Download und -Import.
6. IPFIX-Collector: Die Windows-Firewall fragt nach, weil er auf `0.0.0.0:4739` lauscht. Die Oberfläche auf `127.0.0.1` löst
   keine Nachfrage aus.
7. Sicherheit: Der Sicherheitstest aus `PLAN.md` 4.8 für die geänderten Stellen, vor allem die Rechte des Helfers.
8. Sauberes System: Installer, Deinstallation, Update über eine ältere Version.
9. MCP-Anbindung (`PLAN.md` 5.4): `openpacketmon.exe mcp` mit Claude Code unter Windows. Dabei prüfen, dass nur der Benutzer `mcp-access.json` im
   Ordner der Einstellungen lesen kann: Die Rechte `0600` setzt der Code nur unter Unix.

## 6. Installer und Release

**Entschieden (Nutzer, 2026-11-02):** Die Tester bekommen **gleich einen Installer**, zunächst **unsigniert** (SmartScreen warnt
dann; „Weitere Informationen → Trotzdem ausführen“). Solange der Haltepunkt aus `PLAN.md` 4.10 gilt, bekommen Tester nur
die Installer-Datei, keinen Zugang zum Repo und keine Release-Seite: Der Code zeigt die an Sniffnet gemeldete Lücke. Der
Installer nennt Npcap und dessen Download, enthält es aber nicht. Die kostenlose Npcap-Lizenz erlaubt den privaten Einsatz
auf bis zu fünf Rechnern; beim Weitergeben an Tester darauf hinweisen. Werkzeug: WiX (MSI, wie Sniffnet, mit `cargo wix`)
oder Inno Setup (einfacher, eine `.exe`); die Wahl auf dem Windows-Rechner treffen und hier eintragen.

- Sniffnet baute ein MSI mit WiX: `resources/packaging/windows/setup.wxs`, `EULA.rtf` und Grafiken im Original bei Commit
  `ab4bf739d464` (bei uns entfernt, siehe `PLAN.md` Phase 2). Wird davon etwas übernommen, gilt die Kennzeichnungspflicht aus
  `CLAUDE.md`, und Sniffnet-Logo, -Grafiken und -Namen müssen raus.
- Der Installer darf Npcap nicht enthalten, er nennt nur den Download.
- **Code-Signatur:** Der Installer soll, wenn möglich, signiert werden (Nutzer, 2026-10-30); ohne Signatur warnt Windows
  SmartScreen. Die Wege stehen in Abschnitt 6.1.
- `dev/release.sh` kennt nur das `.deb`. Es braucht eine Erweiterung oder einen Schritt in GitHub Actions.
- Die Update-Prüfung vergleicht nur Versionsnummern. Der Hinweis muss unter Windows auf den Installer zeigen.
- `THIRD-PARTY-LICENSES.txt` muss die Windows-Crates enthalten, siehe Abschnitt 7.

### 6.1 Code-Signatur

Recherche vom 2026-10-30, vor der Umsetzung erneut prüfen. Vorgehen: zuerst SignPath versuchen, sonst ein OV-Zertifikat kaufen.
Auch mit Signatur kann SmartScreen anfangs warnen, bis der Installer genug Reputation gesammelt hat.

| Weg | Für uns | Bedingungen und Haken |
|---|---|---|
| **SignPath Foundation** (kostenlos für Open Source, https://signpath.org/terms.html) | **erster Versuch, Zusage nicht sicher** | Siehe unten |
| **OV-Zertifikat** einer Zertifizierungsstelle | Rückfall, geht auch für Privatpersonen | Kostet jährlich Geld, Preise vor dem Kauf vergleichen. Der Schlüssel liegt auf Hardware-Token oder in einem Cloud-Dienst, das muss zum Build passen |
| **Azure Artifact Signing** (früher Trusted Signing) | **kommt nicht infrage** | Laut einer Antwort auf Microsoft Q&A für Einzelentwickler nur in USA und Kanada, Organisationen auch in der EU. Ab 9,99 $ im Monat, braucht ein bezahltes Azure-Abo |

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

- `deny.toml`: `targets` um `x86_64-pc-windows-msvc` ergänzen.
- `dev/third-party-licenses.py`: `TARGET` gilt nur für Linux. Es kommen z. B. die `windows-*`-Crates dazu,
  die Windows-Lizenzliste kann also größer werden. Am besten erzeugt das Skript eine gemeinsame Liste für beide Plattformen.
- `dev/*.sh` laufen in Git Bash. `dev/build-deb.sh` und `dev/release.sh` sind nur für Linux gedacht.
- `.github/workflows/ci.yml`: einen Job auf `windows-latest` ergänzen, mit Npcap SDK (Sniffnet lädt `npcap-sdk-1.16.zip` von
  npcap.com und setzt `LIB`). Die Sniffnet-CI installiert Npcap per OEM-Installer über ein Secret, das wir nicht haben, und
  lässt Windows ohne das Secret bei Pull Requests aus. Bei uns ist also zuerst `fmt`, `check` und `clippy` realistisch. Ob
  `cargo test` in der CI ohne Npcap läuft, muss sich zeigen.

## 8. Reihenfolge

1. Einrichten (Abschnitt 1 bis 3), `cargo check --workspace` und die echte Fehlerliste mit Abschnitt 4 abgleichen. **Erledigt** (2026-11-02).
2. Programm bauen und starten, ein Mitschnitt mit Npcap. **Teilweise:** Debug-Fassung baut und das Fenster startet; Release-Fassung
   (`gag`) und Mitschnitt fehlen noch.
3. `cargo test`, Pfade und Rechte. **Tests grün** (2026-11-02); Rechte von `mcp-access.json` und Helfer erbt offene Dateien offen.
4. Windows-Eigenheiten aus Abschnitt 5.
5. CI, Lizenzliste, `deny.toml`.
6. Installer (unsigniert), Test auf sauberem System, README; dann **Weitergabe an die Tester** als Datei (Abschnitt 6).
   Ein Windows-Release auf GitHub erst mit dem ersten Release nach dem Haltepunkt.
7. Signatur beantragen (Abschnitt 6.1), danach signierte Installer.

**Beginn (Nutzer, 2026-11-02):** Windows wird **vor** dem ersten Release gemacht, weil andere Tester eine Windows-Fassung
brauchen; MCP ist fertig. Bis zum Release gilt auch auf dem Windows-Rechner der Haltepunkt aus `PLAN.md` 4.10. Die Abschnitte 4 und 5 stammen aus der Zeit vor MCP: Beim Beginn die Liste um das ergänzen, was
seitdem dazugekommen ist.
