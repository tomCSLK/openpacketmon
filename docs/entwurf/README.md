# Entwurf der neuen Oberfläche (openpacketmon)

Klickbare HTML-Entwürfe zur Abnahme der Gestaltung. **Kein Anwendungscode.** Die echte Oberfläche zeichnet openpacketmon mit egui
(`src/gui/`); übernommen würden Farben, Abstände, Schriften, Symbole, Aufbau und Logo, nicht das HTML.

Gestaltung und Farben wie **opensecmon** und der Entwurf der neuen Oberfläche von **opennetmon** (abgenommen am 2026-11-09),
Inhalt und Funktionen wie openpacketmon 0.4.0. Stand: 2026-11-09, **vom Nutzer abgenommen** (siehe „Abgenommen“), vorerst nur dokumentiert und
geplant. Nichts davon ist in der Anwendung umgesetzt.

- Start: `index.html` im Browser öffnen (geht direkt als Datei; geprüft wurde über `python3 -m http.server`).
- Die Entwurfsleiste oben wechselt die Seite, den **Zustand der Aufzeichnung** („läuft“, „pausiert“, „keine“), den Fall **„Update“**
  („mit Launcher“, „ohne Launcher“ wie unter Windows, „keins“ = aktuell) und das **Thema** (System, Hell, Dunkel).
  Läuft die Aufzeichnung, kommen wie im Programm alle 0,3 s neue Pakete dazu (Paketliste, Statusleiste, Übersicht).
- **Alle Daten sind erfunden** und stammen aus den Dokumentationsbereichen: 192.0.2.0/24 (eigenes Netz), 198.51.100.0/24, 203.0.113.0/24,
  2001:db8::/32, MAC 00:00:5e:00:53:xx, AS 64496–64511, Namen unter example.com/.net/.org und `.local` (einzige Ausnahme: 172.17.0.1, die
  übliche Adresse von `docker0` in der Liste der Adapter). Rund 48 000 Pakete, darin zwei
  ausführlich gebaute Abläufe: DNS und TLS-Handshake mit SNI (Pakete 1200–1220, mit Dup ACK und Retransmission) und eine HTTP-Abfrage mit
  zusammengesetzter Antwort (2400–2413).

## Seiten

| Datei | Inhalt |
|---|---|
| `aufzeichnung.html` | ohne Aufzeichnung: Quelle (Adapter mit Kurven, ausgegraute ohne IP-Verkehr; Datei; IPFIX), BPF-Filter, Paketspeicher, PCAP-Datei; sonst Zustand mit Pausieren/Beenden |
| `uebersicht.html` | Einheit Bytes/Bits/Pakete, sechs Kacheln, „Auffällig“ (Vorschlag), Verkehr pro Sekunde (5 min, mit Tooltip), Top-Hosts/-Dienste/-Programme |
| `verbindungen.html` | Verbindungsfilter mit Prüfung, Vorschlägen und Hilfe, Sortierung, „Nur Favoriten“, Seiten (`?f=…`) |
| `verbindung.html` | Fakten, Gegenstelle mit Stern, Sprünge zu Paketen, Statistik und Stream (`?id=st-http`, `st-tls`, `st-dns1` …) |
| `pakete.html` | durchgehende Paketliste (virtuell, 48 000 Zeilen) mit Live-Folgen, Anzeigefilter, „Gehe zu Paket“ (Strg+G), Leiste für das gewählte Paket, ↑/↓/Enter/Pos1/Ende (`?f=`, `?c=` Verbindung, `?cf=` Verbindungsfilter, `?n=`) |
| `paket.html` | Schichten und Bytes; ein Feld markiert seine Bytes und umgekehrt, Bezüge zu anderen Paketen, ←/→ blättern (`?n=1205` Client Hello, `?n=2409` zusammengesetzte HTTP-Antwort) |
| `stream.html` | Stream folgen: Client und Server farbig, Richtung, Text oder Hex (`?c=st-http`) |
| `statistik.html` | Kacheln, Verkehr der Auswahl, Protokollhierarchie mit Sprüngen, TCP-Analyse, Antwortzeiten (`?f=`, `?c=`, `?cf=`) |
| `hosts.html`, `dienste.html`, `programme.html` | Listen mit Verkehrsbalken, Favoriten (nicht bei Diensten), Sprung in die Verbindungen |
| `benachrichtigungen.html` | Verkehr der Favoriten |
| `einstellungen.html` | ein Bereich mit Unternavigation: Anzeige, Benachrichtigungen, Paketspeicher, Geodaten, KI-Assistenten (MCP), Aktualisierungen (Versionen, Weg zum Update, Release-Notes), Über und Lizenzen (`#mcp` …; `?notes=1#aktualisierungen` springt zu den Release-Notes) |
| `logo.html` | fünf Logo-Entwürfe, **gewählt C „Lupe über Paketen“**; jeder lässt sich zum Vergleich in die Kopfleiste setzen |
| `breiten.html` | dieselbe Seite bei 1440 × 900, 1280 × 820 (Standardgröße des Fensters, maßgeblich), 1024 × 720 und 760 × 480 (nicht eigens gestaltet) |

## Was sich gegenüber heute ändert

- **Kopfleiste statt violetter Seitenleiste:** vorn Übersicht, Verbindungen, Pakete, Statistik, Hosts, Dienste, Programme; unter 1180 px wandern
  die letzten drei unter „Mehr“, unter 900 px bleiben nur Symbole. Benachrichtigungen werden zur Glocke mit Zähler, Einstellungen zum Zahnrad.
  Die Paketliste bekommt dadurch die volle Breite des Fensters.
- **Aufzeichnung immer sichtbar:** rechts oben Zustand, Adapter, Dauer, Pausieren/Fortsetzen und Beenden; ohne Aufzeichnung der Knopf
  „Aufzeichnung starten“. Die Seite „Aufzeichnung“ ist damit kein Menüpunkt mehr.
- **Statusleiste unten** statt des Fußes der Seitenleiste: Pakete, Paketspeicher, verworfene Pakete, Hinweis auf KI-Assistenten (grün, orange bei
  Paketinhalten), neue Version, Version, Quellcode, Sniffnet, Lizenzen und „IP Geolocation by DB-IP“. Der DB-IP-Hinweis (CC BY 4.0) bleibt bei
  jeder Breite sichtbar; was nicht passt, entfällt in dieser Reihenfolge: Quellcode und Sniffnet (unter 1440 px, stehen unter „Über und Lizenzen“),
  verworfene Pakete (unter 1024 px), Paketspeicher, KI-Hinweis und Version (unter 900 px).
- **Neue Version** (Ergänzung zu den Updates, 2026-11-09, abgestimmt mit dem Entwurf des geplanten Launchers): openpacketmon sucht wie heute
  selbst auf GitHub nach einer neuen Version und lädt oder installiert nichts. Der Hinweis „Version x.y verfügbar“ in der Statusleiste öffnet
  ein kleines Fenster über der Statusleiste: „openpacketmon x.y ist erschienen“, installierte Version, Datum, Knöpfe „Was ist neu?“ und
  - **mit Launcher** (Linux, Launcher installiert): „Der Launcher lädt, prüft und installiert es. openpacketmon muss dafür geschlossen sein;
    der Launcher wartet darauf.“ Knopf **„Aktualisieren“** startet den Launcher mit der Seite von openpacketmon (läuft er schon, zeigt sein
    Fenster sie). Läuft eine Aufzeichnung, dazu der Hinweis, vorher als PCAP zu sichern;
  - **ohne Launcher** (und immer unter Windows): „Das Paket liegt auf der Release-Seite“, Knopf **„Release-Seite öffnen“**.

  „Was ist neu?“ führt zu **Einstellungen → Aktualisierungen**: Installiert, Neueste Version (mit Datum oder „aktuell“), Zuletzt gesucht,
  derselbe Knopf, ohne Launcher kurz die Schritte von Hand (Linux: `.deb` und `SHA256SUMS` laden, `sha256sum -c`, `apt install`; Windows:
  Installer), darunter **„Was ist neu in x.y“** mit den Release-Notes von GitHub (englisch wie dort). Ohne neue Version kein Hinweis in der
  Statusleiste; die Seite zeigt „aktuell“ und die Release-Notes der installierten Version. Name und Aufruf des Launchers stehen nicht im
  Repo, solange er nicht öffentlich ist (`CLAUDE.local.md`).
- **Farben:** graue Flächen mit ruhigem Blau (`#1c5ea8`) statt Violett, Zustände immer mit Symbol und Text. Die Farben der Paketliste behalten
  ihre Bedeutung, sind aber leiser getönt und für hell und dunkel abgestimmt; Eingehend Blau, Ausgehend Orange (wie opennetmon).
- **Schrift:** Fira Sans und Fira Code (Adressen, Zeiten, Hex, Filter) statt Inter; dichte Zeilen (Tabellen 34 px, Paketliste 26 px).
  Ligaturen sind aus (sonst würde Fira Code aus „0x“ ein „×“ machen; egui setzt ohnehin keine).
- **Einstellungen** als ein Bereich mit Unternavigation; „Lizenzen“ wandert dorthin („Über und Lizenzen“), der Link in der Statusleiste bleibt.
- **Neu** (gibt es heute nicht, vom Nutzer alle übernommen): „Auffällig“ auf der Übersicht, Tastenkürzel-Hilfe mit „?“, „/“ springt
  ins Filterfeld, Leiste über der Paketliste für das gewählte Paket, Rückfrage beim Beenden der Aufzeichnung mit den Zahlen, die verloren gehen (heute nur beim Schließen des Fensters).
- **Logo:** neues Zeichen im Stil der beiden anderen Programme statt der violetten Kugel: **C „Lupe über Paketen“** (Seite Logo).

## Abgenommen (Nutzer, 2026-11-09)

1. Farben und Aufbau (Kopfleiste, Aufzeichnung oben rechts, Statusleiste) passen.
2. Logo: **C „Lupe über Paketen“**. Damit es eindeutig bleibt, bekommen Such- und Filterfelder kein Lupensymbol (im Entwurf haben sie keins).
3. Die neuen Teile bleiben alle: „Auffällig“, Tastenkürzel-Hilfe, „/“, Leiste für das gewählte Paket, Rückfrage beim Beenden.
4. Breite: openpacketmon ist ein Desktop-Programm, maßgeblich ist **1280 × 820** (heute schon die Standardgröße des Fensters). Kleinere Größen
   werden nicht eigens gestaltet und geprüft. **Mindestgröße bleibt 760 × 480** wie heute (Vorschlag, vom Nutzer bestätigt), nicht
   1280 × 820, weil das Fenster sonst auf Bildschirmen mit 1366 × 768 nicht mehr ganz Platz hätte. Unter 1280 × 820 wird die Seite nur
   enger, nach den Regeln oben (unter 1180 px „Mehr“, unter 900 px nur Symbole, Statusleiste kürzt in fester Reihenfolge).
5. Vorerst **nur dokumentiert und geplant**: Der Entwurf liegt im Repo (auf Wunsch des Nutzers committet), umgesetzt wird nichts.
6. Die angekündigte **Ergänzung zu den Updates** ist der Hinweis auf neue Versionen nach der Absprache mit dem Launcher (oben „Neue Version“),
   am 2026-11-09 in den Entwurf eingebaut. **Noch nicht abgenommen.**

## Umsetzung in egui (Vorschlag, erst nach Abnahme der Ergänzung zu den Updates und nach „go“, in Stufen mit Test-Paket)

| Stufe | Inhalt | Dateien in der Anwendung |
|---|---|---|
| A | Farben hell/dunkel aus `assets/tokens.css`, Abstände, Zeilenhöhen, Farben der Paketliste, Schriften Fira Sans/Code (als TTF, egui liest kein woff2) statt Inter | `src/gui/theme.rs`, `resources/fonts/`, `THIRD-PARTY-LICENSES.txt` (`dev/third-party-licenses.py`) |
| B | Kopfleiste (`Panel::top`) mit Navigation, „Mehr“, Aufzeichnungsanzeige, Glocke, Thema, Hilfe, Zahnrad; Statusleiste (`Panel::bottom`); Seitenleiste entfällt; Fenster bleibt 1280 × 820, Mindestgröße 760 × 480 | `src/gui/mod.rs`, `src/gui/widgets.rs`, Texte in `locales/` |
| C | Seite für Seite: Übersicht, Paketliste und -details, Verbindungen, Statistik, Listen, Einstellungen mit Unternavigation, Aufzeichnung | `src/gui/pages/*.rs`, `src/gui/chart.rs`, `src/gui/filter_field.rs`; Seitendaten bleiben in `src/view/` |
| D | Symbole nach Lucide statt der heutigen SVGs, Logo C und Programmsymbole (PNG 16–1024, `.ico`), README-Bild | `resources/icons/`, `resources/packaging/`, `docs/assets/`, `dev/make-ico.py` |

Neue Version (in B und C): `src/utils/check_updates.rs` liest heute nur den Release-Titel (`name`, `vX.Y.Z`); dazu kommen aus derselben
Antwort `body` (Release-Notes, Markdown) und `published_at`, außerdem die Zeit der letzten Suche. Die Release-Notes brauchen einen kleinen
Markdown-Renderer (Überschriften, Listen, Links, Code; eigener Code oder eine Crate mit passender Lizenz, dann in `deny.toml` und die
Lizenzliste). Den Launcher erkennt openpacketmon an seiner Programmdatei (nur Linux); der Aufruf startet ihn ohne Shell und wartet nicht auf ihn.
Die Release-Notes sind Fremdtext aus dem Netz: Links nur für `https://`, kein HTML, Länge begrenzen.

Zu beachten: `src/gui/tests.rs` prüft Bedienung und Bilder, die Übersetzungsprüfung (`every_message_is_used`) neue und entfallende Texte,
`deny.toml` und die Lizenzliste neue Schriften. Die Lucide-Symbole stehen unter ISC und müssen in die Lizenzliste.

## Dateien

| Datei | Inhalt |
|---|---|
| `assets/tokens.css` | Farben hell/dunkel, Schriften, Abstände, Farben der Paketliste und von Stream folgen |
| `assets/app.css` | Bauteile und Breiten |
| `assets/icons.js` | Symbole nach Lucide (ISC), nachgezeichnet, dazu die Logo-Entwürfe `m-a` bis `m-e` und zum Vergleich `m-sec`, `m-net` |
| `assets/data.js` | erfundene Pakete, Verbindungen, Schichten und Bytes, Filter mit eigener kleiner Grammatik (nur für den Entwurf) |
| `assets/app.js` | Rahmen: Entwurfsleiste, Kopfleiste, Statusleiste, Thema, Tastenkürzel, Dialoge, Live-Takt |
| `assets/pages.js` | Logik der Seiten |
| `fonts/` | Fira Sans 400/500/600, Fira Code 400/500 (latin, woff2) von Fontsource 5.3.0, SIL OFL 1.1 (`OFL-*.txt`) |

Kein Code aus Wireshark: Feldnamen und Info-Texte folgen der üblichen Schreibweise, Zerlegung und Filter sind eigene Nachbildungen für den Entwurf.
