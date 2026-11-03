# openpacketmon unter Windows testen

Anleitung für Tester der Windows-Fassung (Stand 2026-11-03). Der Installer liegt seit 0.4.0 beim
[Release auf GitHub](https://github.com/tomCSLK/openpacketmon/releases/latest) und ist signiert. Rückmeldungen gehen direkt an
Thomas oder als Issue auf GitHub.

## Was gebraucht wird

- Windows 10 oder 11, 64 Bit.
- **Npcap**, das Programm, über das openpacketmon den Netzwerkverkehr mitliest. Es gehört nicht zu openpacketmon und
  muss selbst installiert werden: https://npcap.com/#download, dort „Npcap … installer“. Beim Installieren
  **„Install Npcap in WinPcap API-compatible Mode“ angehakt lassen**. Die kostenlose Ausgabe darf auf bis zu fünf
  Rechnern für den eigenen Gebrauch eingesetzt werden; die genauen Bedingungen stehen auf npcap.com.
- Die Installer-Datei `openpacketmon-<version>-setup.exe` vom Release.

## Smart App Control

Installer und Programme sind signiert (Certum, „Open Source Developer Thomas Waldemar Cieslik“). Smart App Control lässt sie
deshalb laufen, es muss nicht ausgeschaltet werden. Getestet am 2026-11-03 mit 0.4.0. Wer prüfen will: Rechtsklick auf die
Datei → Eigenschaften → Digitale Signaturen.

## Installieren

1. Die Installer-Datei starten. Solange eine Version neu ist, kann Windows SmartScreen trotz Signatur „Der Computer wurde
   durch Windows geschützt“ melden: **Weitere Informationen → Trotzdem ausführen**. Als Herausgeber muss dort der Name oben stehen.
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
- **Einstellungen**: Sprache (anfangs die Anzeigesprache von Windows, Deutsch oder sonst Englisch; in den Einstellungen
  umstellbar), Geodaten laden, Benachrichtigungen mit Ton.
- **Schließen** des Fensters während einer Aufzeichnung: Es kommt eine Rückfrage.

## Rückmeldung

Am meisten hilft: was gemacht wurde, was erwartet war, was passiert ist, dazu Windows-Version und ein Bildschirmfoto.
Stürzt openpacketmon ab oder startet nicht, bitte auch die Meldung abschreiben.

## Deinstallieren

Über **Einstellungen → Apps → Installierte Apps → openpacketmon**. Die Einstellungen bleiben dabei in
`%APPDATA%\openpacketmon` liegen und können von Hand gelöscht werden. Npcap wird getrennt deinstalliert.
