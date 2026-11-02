# openpacketmon unter Windows testen

Anleitung für Tester der Windows-Fassung (Stand 2026-11-02). Es ist eine **Testfassung**: Bitte die Installer-Datei nicht
weitergeben. Rückmeldungen gehen direkt an Thomas.

## Was gebraucht wird

- Windows 10 oder 11, 64 Bit.
- **Npcap**, das Programm, über das openpacketmon den Netzwerkverkehr mitliest. Es gehört nicht zu openpacketmon und
  muss selbst installiert werden: https://npcap.com/#download, dort „Npcap … installer“. Beim Installieren
  **„Install Npcap in WinPcap API-compatible Mode“ angehakt lassen**. Die kostenlose Ausgabe darf auf bis zu fünf
  Rechnern für den eigenen Gebrauch eingesetzt werden; die genauen Bedingungen stehen auf npcap.com.
- Die Installer-Datei `openpacketmon-<version>-setup.exe`.

## Vorher prüfen: Smart App Control

Die Testfassung ist noch **nicht signiert**. Windows 11 blockiert unsignierte Programme ganz, wenn Smart App Control
eingeschaltet ist. Nachsehen unter **Windows-Sicherheit → App- & Browsersteuerung → Smart App Control**:

- **Aus** oder **Auswertung**: Es geht weiter wie unten beschrieben.
- **Ein**: Der Installer wird mit „Eine Anwendungssteuerungsrichtlinie hat diese Datei blockiert“ abgewiesen. Dann bitte
  auf eine signierte Fassung warten. Smart App Control lässt sich zwar ausschalten, aber danach nicht ohne Weiteres wieder
  einschalten (nur durch Zurücksetzen von Windows); das sollte niemand für einen Test tun.

## Installieren

1. Die Installer-Datei starten. Windows SmartScreen meldet „Der Computer wurde durch Windows geschützt“, weil die Datei
   unsigniert und neu ist: **Weitere Informationen → Trotzdem ausführen**.
2. Standardmäßig wird openpacketmon nur für den eigenen Benutzer installiert, ohne Administratorrechte. Wer es für alle
   Benutzer will, wählt das in der ersten Frage des Installers (braucht dann Administratorrechte).
3. Fehlt Npcap noch, sagt das der Installer auf einer eigenen Seite, mit einem Knopf zur Downloadseite. openpacketmon wird
   trotzdem installiert; nach dem Installieren von Npcap einfach starten.
4. Gestartet wird openpacketmon über das Startmenü.

openpacketmon selbst braucht nie Administratorrechte.

## Was ausprobieren

- **Aufzeichnung**: einen Adapter wählen (WLAN, Ethernet oder „Adapter for loopback traffic capture“) und starten.
  Pause, Fortsetzen, Stoppen. Wer mag, mit einem Filter wie `tcp port 443`.
- **Seiten**: Übersicht, Verbindungen, Pakete, Paketdetails, Statistik, Hosts, Dienste, Programme.
- **Aufzeichnungsdatei**: eine `.pcap`-Datei öffnen (Quelle „Aufzeichnungsdatei“), stoppen und danach prüfen, ob sich die
  Datei im Explorer löschen oder umbenennen lässt.
- **Einstellungen**: Sprache (anfangs Englisch, unter „Settings“ auf Deutsch umstellbar), Geodaten laden, Benachrichtigungen
  mit Ton.
- **Schließen** des Fensters während einer Aufzeichnung: Es kommt eine Rückfrage.

## Rückmeldung

Am meisten hilft: was gemacht wurde, was erwartet war, was passiert ist, dazu Windows-Version und ein Bildschirmfoto.
Stürzt openpacketmon ab oder startet nicht, bitte auch die Meldung abschreiben.

## Deinstallieren

Über **Einstellungen → Apps → Installierte Apps → openpacketmon**. Die Einstellungen bleiben dabei in
`%APPDATA%\openpacketmon` liegen und können von Hand gelöscht werden. Npcap wird getrennt deinstalliert.
