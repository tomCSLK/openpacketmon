# Windows-Version von openpacketmon

Stand: 2026-10-30. Diese Datei beschreibt, was nötig ist, um das Projekt auf einem Windows-Rechner fortzusetzen und dort eine
Windows-Version zu bauen und zu testen.

**Heutiger Stand:** openpacketmon läuft nur unter Linux und **baut unter Windows noch nicht**. Es ist nichts davon umgesetzt.
Die Punkte in Abschnitt 4 stammen aus einer Durchsicht des Codes; ein Build für Windows war auf dem Linux-Rechner nicht
möglich (die Abhängigkeiten `ring` und `aws-lc-sys` brauchen einen C-Compiler für Windows). Die vollständige Liste
der Fehler liefert also erst der erste `cargo check` unter Windows.

**Feste Vorgaben:** Npcap wird **nie mitgeliefert**, der Nutzer installiert es selbst (proprietäre Lizenz). Wireshark-Code
bleibt tabu. Die Arbeitsregeln in `CLAUDE.md` gelten unverändert. Die Arbeit beginnt **erst nach dem ersten Release (0.4.0 oder
0.5.0) und nach der MCP-Anbindung** (`PLAN.md` 5.4), die Windows-Version enthält MCP also schon. Seit der Umstellung auf egui
(`PLAN.md` 5.5) gibt es keinen Webserver und kein Webview mehr. Der Installer soll, wenn möglich, signiert
werden (Nutzer, 2026-10-30/31).

## 1. Was auf dem Windows-Rechner installiert sein muss

Vorgesehen sind Windows 10/11 x64. Die `winget`-Befehle sind Vorschläge und noch nicht ausprobiert.

| Was | Wofür | Installation |
|---|---|---|
| Git für Windows | Repo, außerdem Git Bash für `dev/*.sh` | `winget install Git.Git` |
| Visual Studio Build Tools mit „Desktop development with C++“ und Windows SDK | Linker und C-Compiler für Rust | `winget install Microsoft.VisualStudio.2022.BuildTools`, danach die Komponente im Visual Studio Installer wählen |
| Rust (rustup, Toolchain `stable-x86_64-pc-windows-msvc`) | Bauen | `winget install Rustlang.Rustup` |
| Npcap | Mitschnitt zur Laufzeit, die Tests und der Build von `pcap` brauchen `wpcap.dll` | https://npcap.com/#download |
| Npcap SDK | Linken gegen `wpcap.lib` | https://npcap.com/#download, entpacken, `Lib\x64` in die Umgebungsvariable `LIB` |
| Python 3 | `dev/third-party-licenses.py`, `dev/update-services.py` | `winget install Python.Python.3.12` |
| GitHub CLI | Anmeldung, Repo, später Releases | `winget install GitHub.cli` |
| cargo-deny 0.20.2 | Lizenz- und Sicherheitsprüfung | `cargo install cargo-deny --locked --version 0.20.2` |
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
# der Pfad ist ein Beispiel: dorthin, wo das Npcap SDK entpackt wurde
[Environment]::SetEnvironmentVariable("LIB", "C:\npcap-sdk\Lib\x64;" + [Environment]::GetEnvironmentVariable("LIB", "User"), "User")
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
| `src/utils/formatted_strings.rs` `redirect_stdout_stderr_to_file` | nutzt die Crate `gag`, die nicht mehr in `Cargo.toml` steht, **baut im Release nicht** | Funktion entfernen oder `gag` wieder aufnehmen; Log-Datei wird gebraucht, wenn es kein Konsolenfenster gibt |
| `src/mmdb/geo.rs` `geo_dir` | `HOME`, `.local/share` (`HOME` fehlt unter Windows) | `%LOCALAPPDATA%\openpacketmon\geo` |
| `src/mmdb/geo.rs` `geoipupdate_file` | `/var/lib/GeoIP/…` | unter Windows weglassen oder den Ordner von `geoipupdate` nehmen, Test anpassen |
| `src/networking/traffic_preview.rs`, `capture_context.rs` | aus Sniffnet, schon für Windows vorbereitet | prüfen |
| `capture/src/main.rs` (Helfer) | unter Linux nicht blockierender Adapter mit eigenem `poll`, Rechteabgabe, größere Pipe | außerhalb von Linux liest er blockierend mit dem Timeout von libpcap, die drei Funktionen `drop_rights`, `enlarge_pipe`, `wait_for_packets` tun dort nichts. Prüfen: Kommt unter Npcap der Timeout auch ohne Pakete? Sonst bleibt eine pausierte Aufzeichnung stehen (siehe `open` in der Datei). Pfad mit `.exe` löst `helper_path` schon auf |
| Adapterliste, `--adapter`, Aufzeichnungsseite | Namen wie `enp0s31f6` | unter Windows heißen sie `\Device\NPF_{GUID}`: überall die Beschreibung zeigen (`get_desc`), den Namen nur intern verwenden |
| Latenzmessung (`surge-ping`) | unter Linux unprivilegiert | ICMP-Sockets ohne Adminrechte gibt es unter Windows vermutlich nicht: prüfen, sonst Hinweis in der Oberfläche |
| Programme je Verbindung (`listeners`) | hat einen Windows-Teil | prüfen, auch die Kürzung auf 15 Zeichen in unserem Code (betrifft nur Linux) |
| Doku-Texte | „Linux“, `setcap`, `systemctl`, `/usr/share/doc/…` | Oberfläche und README prüfen |

Der Build von Windows braucht in `build.rs` zusätzlich das Programm-Icon: Sniffnet nutzte dafür `winresource`. Bei uns wurde das
entfernt, es kommt für `openpacketmon.exe` mit einem eigenen `.ico` wieder dazu (aus
`openpacketmon_256x256.png` u. a. erzeugen, **nicht** das `sniffnet.ico` des Originals).

### Fenster (`src/gui/`)

egui, wgpu und `rfd` laufen unter Windows, der Code dort ist bisher aber nur unter Linux gelaufen. Zu prüfen: Dateidialoge, Ton (`rodio` nutzt dort WASAPI statt ALSA),
`LANG`-Abfrage in `language_of`, Konsole (`#![cfg_attr(windows, windows_subsystem = "windows")]`, sonst geht mit dem Fenster ein Konsolenfenster auf) und Verhalten beim Schließen.

Die Trennung der Rechte bleibt: Unter Linux hat nur der Helfer `openpacketmon-capture` die Mitschnittrechte und gibt sie nach dem
Öffnen des Adapters ab, das Programm läuft ohne. Unter Windows entscheidet die Installation von Npcap, ob normale Benutzer
mitschneiden dürfen oder nur Administratoren. Das Programm soll nie als Administrator laufen müssen.

### Was es unter Windows nicht gibt

- **`setcap` und `postinst`:** Das übernimmt Npcap.
- **`.deb`:** ersetzt durch einen Installer (Abschnitt 6).

## 5. Was unter Windows getestet werden muss

1. `cargo test --workspace`: Tests mit Pfaden (`/var/lib/GeoIP`, `/usr/bin/…`) und Rechten scheitern vermutlich zuerst.
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
9. MCP-Anbindung (`PLAN.md` 5.4): `openpacketmon.exe mcp` mit Claude Code unter Windows.

## 6. Installer und Release

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

1. Einrichten (Abschnitt 1 bis 3), `cargo check --workspace` und die echte Fehlerliste mit Abschnitt 4 abgleichen.
2. Programm bauen und starten, ein Mitschnitt mit Npcap.
3. `cargo test`, Pfade und Rechte.
4. Windows-Eigenheiten aus Abschnitt 5.
5. CI, Lizenzliste, `deny.toml`.
6. Installer, Test auf sauberem System, README, erstes Windows-Release, zunächst unsigniert.
7. Signatur beantragen (Abschnitt 6.1), danach signierte Installer.

**Beginn (Nutzer, 2026-10-30/31):** Reihenfolge erstes Release → MCP-Anbindung → Windows. Bis zum Release gilt auch auf dem Windows-Rechner
der Haltepunkt aus `PLAN.md` 4.10. Die Abschnitte 4 und 5 stammen aus der Zeit vor MCP: Beim Beginn die Liste um das ergänzen, was
seitdem dazugekommen ist.
