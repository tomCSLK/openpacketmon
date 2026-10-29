# Texte der Seiten (templates/ und src/web/). Deutsch ist die Ausgangssprache.

## Rahmen und Navigation
nav-main = Hauptnavigation
nav-traffic = Verkehr
nav-overview = Übersicht
nav-connections = Verbindungen
nav-packets = Pakete
nav-statistics = Statistik
nav-hosts = Hosts
nav-services = Dienste
nav-programs = Programme
nav-notifications = Benachrichtigungen
nav-setup = Einrichtung
nav-capture = Aufzeichnung
nav-settings = Einstellungen
nav-theme = Design
nav-theme-toggle = Design umschalten
source-code = Quellcode
licenses = Lizenzen
based-on-sniffnet = basiert auf Sniffnet
update-available = Version { $version } verfügbar
back = Zurück
apply = Anwenden
save = Speichern
unknown = unbekannt
not-applicable = nicht zutreffend
show-all = Alle anzeigen
unit = Einheit
unit-bytes = Bytes
unit-bits = Bits
unit-packets = Pakete
sort = Sortierung
sort-desc = Größte zuerst
sort-asc = Kleinste zuerst
sort-recent = Neueste zuerst
only-favorites = Nur Favoriten
favorite = Favorit
favorite-add = Zu den Favoriten
favorite-remove = Aus den Favoriten entfernen
favorite-address-add = Adresse zu den Favoriten
favorite-address-remove = Adresse aus den Favoriten entfernen
no-capture = Es läuft keine Aufzeichnung.
no-capture-start = Aufzeichnung starten
geo-missing = Ohne Geodatenbank bleiben Länder und Netzbetreiber leer.
geo-missing-link = In den Einstellungen herunterladen
flash-settings-saved = Einstellungen gespeichert.
flash-geo-downloaded = Geodatenbanken heruntergeladen. Sie gelten für Adressen, die ab jetzt erkannt werden; für die bisherigen die Aufzeichnung neu starten.
flash-notifications-cleared = Benachrichtigungen gelöscht.
flash-capture-stopped = Aufzeichnung beendet, die Daten wurden verworfen.

## Anmeldung und Fehler
signed-out-title = Nicht angemeldet
signed-out-text = Öffne openpacketmon über das Startmenü oder mit diesem Befehl, dann meldet sich der Browser selbst an:
signed-out-hint = Der Startlink mit dem Zugangsschlüssel gilt, solange openpacketmon läuft, und ist nur für deinen Benutzer lesbar.
error-title = Fehler
error-forbidden = Die Anfrage wurde abgelehnt (abgelaufenes Formular oder fremde Seite). Lade die Seite neu und versuche es noch einmal.
error-not-found = Diese Seite gibt es nicht (mehr). Eine Verbindung verschwindet z. B., wenn die Aufzeichnung beendet wird.

## Aufzeichnung
capture-title = Aufzeichnung
capture-subtitle = Wähle aus, welcher Verkehr untersucht wird.
capture-source = Quelle
source-device = Netzwerkadapter
source-file = Aufzeichnungsdatei
source-ipfix = IPFIX-Kollektor
capture-adapters = Netzwerkadapter
capture-adapters-hint = Die Kurven zeigen die Pakete der letzten 30 Sekunden; so erkennst du den Adapter mit Verkehr.
capture-adapter = Adapter
capture-addresses = Adressen
capture-activity = Pakete (30 s)
capture-no-adapters = Keine Netzwerkadapter gefunden.
capture-no-adapters-hint = openpacketmon braucht das Recht, Pakete mitzuschneiden (setcap cap_net_raw,cap_net_admin=eip, beim Paket eingerichtet).
capture-export-enable = Pakete zusätzlich in eine PCAP-Datei schreiben
capture-export-dir = Ordner
capture-export-name = Dateiname
capture-packets-full = Vollständige Pakete
capture-packets-headers = Nur Kopfdaten
capture-packets-off = Aus
capture-packets-hint = Gespeicherte Pakete zeigen die Seiten Pakete, Paketdetails und Stream folgen und lassen sich als PCAP herunterladen. Vollständige Pakete brauchen mehr Arbeitsspeicher (höchstens 512 MiB), nur damit sind Inhalte wie der TLS-Servername (SNI) und Stream folgen vollständig. „Nur Kopfdaten“ behält die ersten 128 Bytes jedes Pakets.
capture-packets-full-max = vollständige Pakete, höchstens { $max } Pakete
capture-packets-headers-max = nur Kopfdaten (128 Bytes je Paket), höchstens { $max } Pakete
capture-file-path = Pfad der Datei (PCAP oder PCAPNG)
capture-file-hint = Die Datei wird auf diesem Rechner gelesen; der Pfad muss für deinen Benutzer lesbar sein.
capture-ipfix-addr = Adresse
capture-ipfix-port = UDP-Port
capture-ipfix-hint = openpacketmon empfängt Flussdaten (IPFIX) von Routern oder Switches. Leer lassen für alle Adressen und Port 4739.
capture-filter = Filter
capture-bpf = BPF-Filter (optional)
capture-bpf-hint = Nur passende Pakete werden ausgewertet, z. B. „tcp port 443“ oder „host 192.168.1.1“ (Syntax wie bei tcpdump).
capture-start = Aufzeichnung starten
capture-running = Aufzeichnung läuft
capture-export = PCAP-Datei
capture-link-type = Link-Typ
capture-listening = Empfängt auf
capture-dropped = Verworfene Pakete
capture-dropped-text = { $adapter } vom Adapter, { $app } von openpacketmon (Puffer voll)
capture-to-overview = Zur Übersicht
capture-pause = Pausieren
capture-resume = Fortsetzen
capture-stop = Beenden
capture-stop-hint = Beim Beenden werden die gesammelten Daten verworfen.
capture-error = Die Aufzeichnung konnte nicht gestartet werden: { $error }
capture-error-unknown = Die Aufzeichnung konnte nicht gestartet werden.
capture-file-missing = Gib den Pfad der Aufzeichnungsdatei an.
capture-adapter-missing = Wähle einen Netzwerkadapter aus der Liste.
capture-ipfix-undecodable = Die empfangenen IPFIX-Daten können zurzeit nicht ausgewertet werden (Vorlage des Exporters fehlt noch).
capture-state-running = läuft
capture-state-paused = pausiert
capture-state-finished = Datei vollständig gelesen
capture-state-error = Fehler

## Übersicht
tile-total = Daten gesamt
tile-in = Eingehend
tile-out = Ausgehend
tile-connections = Verbindungen
tile-hosts = Hosts
tile-dropped = Verworfene Pakete
chart-title = Verkehr pro Sekunde
chart-empty = Noch keine Daten.
top-hosts = Top-Hosts
top-services = Top-Dienste
top-programs = Top-Programme

## Hosts, Dienste, Programme
hosts-subtitle = Gegenstellen mit Domain, autonomem System und Land.
services-subtitle = Dienste nach Port und Protokoll.
programs-subtitle = Programme dieses Rechners, die Verkehr verursachen.
programs-unsupported = Programme werden nur bei der Aufzeichnung eines Netzwerkadapters erkannt.
entries-count = { $count ->
    [one] 1 Eintrag
   *[other] { $count } Einträge
}
entries-empty = Noch keine Einträge.
entries-filter = Name enthält …
entries-truncated = Angezeigt werden { $shown } von { $total } Einträgen.
entry-name = Name
entry-traffic = Verkehr
entry-in = Eingehend
entry-out = Ausgehend
entry-total = Gesamt
tag-local = lokal
tag-loopback = Loopback
tag-bogon = Bogon: { $kind }
tag-multicast = Multicast
tag-broadcast = Broadcast
category-web = Web
category-email = E-Mail
category-chat = Chat und Nachrichten
category-media = Sprache und Medien
category-files = Dateiübertragung und Freigaben
category-storage = Speicher und Sicherung
category-database = Datenbanken und Caches
category-remote = Fernzugriff
category-management = Überwachung und Verwaltung
category-discovery = Namensauflösung und Diensterkennung
category-network = Netzwerkinfrastruktur
category-vpn = VPN, Tunnel und Proxys
category-identity = Identität und Verzeichnisse
category-printing = Drucken und Scannen
category-gaming = Spiele
category-industrial = Industrie und Automation
category-development = Softwareentwicklung
category-middleware = Middleware
category-security = Sicherheit
category-licensing = Softwarelizenzen
category-other = Sonstige

## Verbindungen
connections-subtitle = Alle Verbindungen der Aufzeichnung mit Filter und Seiten.
filter-reset = Zurücksetzen
connections-none = Keine Verbindungen
connections-filter = Verbindungsfilter
connections-filter-placeholder = Verbindungsfilter, z. B. ip.addr == 192.168.1.20 oder port == 443 oder domain contains google
connections-filter-help = Hilfe zum Verbindungsfilter
connections-filter-help-text = Dieselbe Sprache wie der Anzeigefilter der Pakete, mit Feldern der Verbindungen. Ein Protokoll allein (tcp, udp, icmp, arp, ipv6) wählt die Verbindungen dieses Protokolls; favorite allein die Verbindungen von Favoriten (Hosts, Programme, Adressen). Vergleiche: == != < <= > >= contains und in {"{"}80 443{"}"}, verknüpft mit and, or, not und Klammern. Text ohne Groß-/Kleinschreibung, Adressen auch als Netz (ip.addr == 192.168.1.0/24). remote ist die Gegenstelle, domain der Name des Hosts (auch Reverse-DNS), as.name und country kommen aus der Geolokalisierung, program sucht in Name und Pfad, direction ist incoming oder outgoing. Beispiele:
connections-range = Verbindungen { $from }–{ $to }
connections-of = von { $total } ({ $data })
connections-empty = Noch keine Verbindungen.
connections-none-filtered = Keine Verbindung passt zum Filter.
rows-per-page = Zeilen pro Seite
pages = Seiten
page-label = Seite
page-of-total = von { $pages }
page-first = Erste
page-last = Letzte
page-prev = Zurück
page-next = Weiter
conn-source = Quelle
conn-source-mac = MAC der Quelle
conn-destination = Ziel
conn-destination-mac = MAC des Ziels
conn-protocol = Protokoll
conn-direction = Richtung
conn-service = Dienst
conn-program = Programm
conn-host = Gegenstelle
conn-packets = Pakete
conn-data = Daten
conn-first = Erstes Paket
conn-last = Letztes Paket
conn-vlan = VLAN
conn-exporter = Exporter
conn-message-types = Nachrichtentypen
direction-in = eingehend
direction-out = ausgehend
connection-title = Verbindung
connection-remote = Gegenstelle
connection-host-connections = Verbindungen des Hosts
connection-address-connections = Verbindungen der Adresse
connection-packets = Pakete
connection-statistics = Statistik

# Pakete
packets-subtitle = Die gespeicherten Pakete der Aufzeichnung mit Schichten und Bytes.
packets-download = Als PCAP herunterladen
packets-download-filtered = Auswahl als PCAP herunterladen
packets-unavailable-source = Für IPFIX-Aufzeichnungen gibt es keine einzelnen Pakete.
packets-disabled = Der Paketspeicher ist ausgeschaltet. In den Einstellungen lässt er sich für die nächste Aufzeichnung einschalten.
packets-none-filtered = Kein Paket passt zum Filter.
packets-empty = Noch keine Pakete.
packets-loading = Pakete werden geladen …
packets-follow-hint = Am Ende der Liste folgt sie den neuen Paketen; hochscrollen hält sie an.
packets-noscript = Die Paketliste braucht JavaScript.
packets-goto = Gehe zu Paket
packets-goto-placeholder = Nr.
packets-goto-key = Paketnummer eingeben und Enter drücken (Strg+G)
packets-kept = { $count } Pakete gespeichert
packets-dropped = { $count } ältere verworfen
packets-headers-only = nur Kopfdaten
packets-filter = Anzeigefilter
packets-filter-placeholder = Anzeigefilter, z. B. dns oder tls.sni contains amazon oder ip.addr == 192.168.1.20
packets-filter-help = Hilfe zum Anzeigefilter
packets-filter-help-text = Ein Protokoll allein (dns, tcp) wählt die Pakete, die es enthalten; ein Feld allein (tls.sni, tcp.flags.syn) die Pakete, die es haben. Vergleiche: == != < <= > >= contains und in {"{"}80 443{"}"}. Verknüpfen mit and, or, not und Klammern. Text ohne Groß-/Kleinschreibung, Adressen auch als Netz (ip.addr == 192.168.1.0/24). domain sucht in DNS-Namen, TLS-SNI und HTTP-Host. Die TCP-Analyse liefert tcp.analysis.* (z. B. tcp.analysis.flags für alle Probleme, tcp.analysis.retransmission); Zeiten wie dns.time oder tcp.analysis.ack_rtt in Sekunden (0.1) oder mit Einheit (100ms, 800us). Beispiele:
packets-filter-protocols = Protokolle:
packets-filter-fields = Felder:
packets-colors = Farben
packets-color-bad = Fehler/Reset/TCP-Probleme
packets-color-tcp = TCP-Auf-/Abbau
packets-color-local = ARP/DHCP/SSDP/NTP
packets-live = Live
filter-error-at = Fehler an Stelle { $pos }: { $message }
filter-error-unknown = unbekanntes Feld oder Protokoll „{ $name }“
filter-error-missing = hier fehlt ein Feld oder Wert
filter-error-value = „{ $value }“ passt nicht zu { $field }
filter-error-operator = „{ $op }“ geht nicht mit { $field }
filter-error-unexpected = „{ $text }“ ist hier nicht erwartet (fehlt and/or?)
filter-error-close = die Klammer wird nicht geschlossen
filter-error-string = das Anführungszeichen wird nicht geschlossen
filter-error-complex = der Filter ist zu lang oder zu tief verschachtelt
packet-number = Nr.
packet-time = Zeit (s)
packet-length = Länge
packet-info = Info
packet-title = Paket { $number }
packet-prev = Vorheriges
packet-next = Nächstes
packet-connection = Zur Verbindung
stream-title = Stream folgen
stream-packets = Pakete dieser Verbindung
stream-text = Text
stream-both = Beide Richtungen
stream-counts = { $bytes } in { $packets } Paketen
stream-gap = … { $bytes } Bytes fehlen (nicht mitgeschnitten oder schon verworfen) …
stream-empty = Diese Verbindung hat in den gespeicherten Paketen keine Nutzdaten.
stream-headers-only = Es werden nur Kopfdaten gespeichert; die Nutzdaten fehlen daher.
stream-cut = Angezeigt ist das erste MiB; den ganzen Inhalt enthält der PCAP-Download.
packet-stream = Stream folgen
packet-filter-conversation = Filter: diese Verbindung
packet-truncated = Das Paket wurde nicht vollständig mitgeschnitten.
packet-truncated-headers = Von diesem Paket sind nur die Kopfdaten gespeichert (Einstellung „Nur Kopfdaten“).
packet-layers = Schichten
packet-bytes = Bytes
packet-hex-cut = Angezeigt sind die ersten { $bytes } Bytes; das ganze Paket enthält der PCAP-Download.
packet-assembled = Zusammengesetzt mit den Paketen { $packets }: Die Bytes ab { $offset } stammen aus diesen Paketen.
packet-acks = ← ACK für Paket { $packet } (nach { $time })
packet-acked-by = → bestätigt in Paket { $packet } (nach { $time })
packet-answers = ← Antwort auf Paket { $packet } (nach { $time })
packet-answered-by = → beantwortet in Paket { $packet } (nach { $time })
host-address = Adresse
host-rdns = Reverse DNS
host-asn = Autonomes System
host-country = Land
latency = Latenz
latency-measure = messen
latency-measuring = wird gemessen …
latency-ms = { $ms } ms
latency-failed = Messung fehlgeschlagen: { $error }

## Benachrichtigungen
notifications-subtitle = Ereignisse der laufenden Aufzeichnung.
notifications-settings = Benachrichtigungen einstellen
notifications-clear = Alle löschen
notifications-none-active = Es ist keine Benachrichtigung eingeschaltet.
notifications-count = { $count ->
    [one] 1 Benachrichtigung
   *[other] { $count } Benachrichtigungen
}
notifications-kept = die neuesten { $count } werden angezeigt
notifications-empty = Noch keine Benachrichtigungen.
notification-favorite = Verkehr eines Favoriten
notification-favorite-text = { $name }: { $data }

## Einstellungen
settings-subtitle = Gelten für alle Aufzeichnungen.
settings-display = Anzeige
settings-language = Sprache
language-browser = Wie der Browser
settings-page-size = Verbindungen pro Seite
settings-sound = Ton
settings-favorite-notification = Benachrichtigen, wenn ein Favorit Daten austauscht
settings-favorite-hint = Favoriten sind Hosts und Programme (Stern in den Listen) sowie einzelne IP-Adressen. Ein Hinweis kommt, sobald ein Favorit Daten austauscht.
settings-favorite-addresses = Favorisierte IP-Adressen
settings-favorite-addresses-hint = Eine Adresse je Zeile (IPv4 oder IPv6). Auch über den Stern auf der Seite einer Verbindung.
settings-addresses-invalid = Nicht gespeichert, keine IP-Adresse: { $words }
settings-volume = Lautstärke
settings-sound-hint = Der Ton wird in geöffneten openpacketmon-Seiten abgespielt, nachdem du einmal in die Seite geklickt hast.
settings-geo = Geodaten
settings-geo-country = Länder
settings-geo-asn = Netzbetreiber (AS)
settings-geo-none = keine Datenbank
settings-geo-built = Stand { $date } ({ $days ->
    [one] vor einem Tag
   *[other] vor { $days } Tagen
})
settings-geo-old = – veraltet, bitte neu herunterladen
settings-geo-download = Geodatenbanken von DB-IP herunterladen
settings-geo-downloading = Wird heruntergeladen …
settings-geo-download-hint = Lädt die kostenlosen Datenbanken „IP to Country Lite“ und „IP to ASN Lite“ (CC BY 4.0, zusammen etwa 10 MB) von db-ip.com nach { $dir }. Nur dabei fragt openpacketmon db-ip.com ab; jeden Monat erscheint ein neuer Stand. Die Daten gelten für Adressen, die danach erkannt werden; für die bisherigen die Aufzeichnung neu starten.
settings-geo-failed = Herunterladen fehlgeschlagen (keine Verbindung zu db-ip.com oder unerwartete Datei). Einzelheiten stehen im Protokoll von openpacketmon.
settings-geo-busy = Die Geodatenbanken werden bereits heruntergeladen.
settings-mmdb-country = Eigene Länderdatenbank (MMDB, optional)
settings-mmdb-asn = Eigene ASN-Datenbank (MMDB, optional)
settings-mmdb-hint = Pfad zu einer eigenen Datei, z. B. MaxMind GeoLite2 oder IPinfo Lite. Leer lassen: die heruntergeladenen Daten von DB-IP, sonst die Dateien von geoipupdate in /var/lib/GeoIP.
settings-packet-store = Paketspeicher
settings-packets-enabled = Pakete speichern (Paketliste, Paketdetails, PCAP-Download)
settings-packets-max = Höchstens Pakete
settings-packets-headers = Nur Kopfdaten speichern (die ersten 128 Bytes jedes Pakets)
settings-packets-hint = Gilt ab der nächsten Aufzeichnung (auch auf der Seite Aufzeichnung wählbar). Mit „Nur Kopfdaten“ fehlen Inhalte wie der TLS-Servername (SNI) teilweise, und Stream folgen ist unvollständig. Die Pakete liegen nur im Arbeitsspeicher, zusammen höchstens 512 MiB; die ältesten werden zuerst verworfen.
settings-desktop = Desktop
settings-autostart = Beim Anmelden im Infobereich starten
settings-autostart-hint = Startet openpacketmon mit der Anmeldung am Desktop als Symbol im Infobereich; das Fenster öffnet sich über das Symbol. Die Aufzeichnung läuft weiter, wenn das Fenster geschlossen wird.
settings-updates = Aktualisierungen
settings-update-checks = Beim Start nach einer neuen Version suchen
settings-update-hint = Fragt GitHub nach der neuesten Version von openpacketmon; es werden keine Daten über dein Netz gesendet.
sound-gulp = Gluck
sound-pop = Plopp
sound-swhoosh = Wusch
sound-none = Kein Ton

# quit
quit-title = Programm
quit-hint = Beendet openpacketmon: Die laufende Aufzeichnung endet, die Einstellungen bleiben gespeichert. Läuft openpacketmon als Dienst (systemctl --user), startet er erst nach „systemctl --user start openpacketmon“ wieder.
quit-button = openpacketmon beenden
quit-done-title = openpacketmon wurde beendet
quit-done-text = Du kannst dieses Fenster schließen. Zum erneuten Start openpacketmon über das Startmenü öffnen oder diesen Befehl ausführen:

# Fenster und Infobereich (openpacketmon-app)
app-open = openpacketmon öffnen
app-browser = Im Browser öffnen
app-quit = openpacketmon beenden
app-saved = Gespeichert unter { $path }

## Statistik
stats-subtitle = Protokolle, TCP-Probleme und Antwortzeiten der gespeicherten Pakete, für alle oder die des Anzeigefilters.
stats-show-packets = Pakete anzeigen
stats-filter-placeholder = Anzeigefilter, z. B. ip.addr == 192.168.1.20 oder tcp.port == 443 (leer: alle Pakete)
stats-packets = Pakete
stats-bytes = Bytes
stats-duration = Zeitraum
stats-packet-rate = Ø Pakete
stats-bit-rate = Ø Datenrate
stats-per-second = { $value }/s
stats-chart = Verkehr der Auswahl
stats-hierarchy = Protokollhierarchie
stats-hierarchy-hint = Jede Zeile zählt die Pakete, die das Protokoll enthalten, mit ihrer ganzen Länge. Ein Klick zeigt die Pakete.
stats-empty = Keine Pakete.
stats-findings = TCP-Analyse
stats-findings-hint = Wiederholungen, Lücken und doppelte ACKs deuten auf Paketverlust hin, ein Nullfenster auf einen überlasteten Empfänger. Ein Klick zeigt die Pakete.
stats-retransmission = Wiederholte Segmente (Retransmission)
stats-lost-segment = Lücken (vorheriges Segment fehlt)
stats-out-of-order = Segmente außer der Reihe
stats-duplicate-ack = Doppelte ACKs
stats-zero-window = Nullfenster (Empfänger voll)
stats-reset = Verbindungsabbrüche (RST)
stats-keep-alive = Keep-Alive
stats-window-update = Fenster-Aktualisierungen
stats-times = Antwortzeiten
stats-times-hint = TCP-Verbindungsaufbau: SYN bis zum ACK des Clients. ACK: Zeit bis zur Bestätigung eines Segments (gemessen an diesem Rechner). DNS: Anfrage bis Antwort.
stats-count = Anzahl
stats-average = Mittel
stats-max = Höchstens
stats-handshake = TCP-Verbindungsaufbau
stats-ack-rtt = TCP-Bestätigung (ACK)
stats-dns-time = DNS-Antwort
