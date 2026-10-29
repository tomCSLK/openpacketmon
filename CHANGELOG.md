# Changelog

All notable changes to openpacketmon are documented in this file.

openpacketmon starts from Sniffnet `main` at commit `ab4bf739d464` (2026-10-20, after Sniffnet v1.5.1);
the history of this repository starts with one initial commit of the cleaned-up fork.
The history of Sniffnet itself is in its own changelog: <https://github.com/GyulyVGC/sniffnet/blob/main/CHANGELOG.md>.

## [Unreleased]

- New name and logo; package, binary and configuration directory are called `openpacketmon`,
  so it can be installed next to Sniffnet
- Removed files that only apply to the Sniffnet project (funding, website, audits, Windows and macOS packaging, upstream CI)
- Update check looks at the releases of this repository
- Removed Sniffnet's desktop UI (iced), charts (plotters), sound playback (rodio), file dialogs, fonts, icons,
  flags, themes, sounds and the 14 UI translations; dependencies went from 701 to 304 crates
- New `engine` module: the traffic monitor without a user interface (capture lifecycle, traffic data,
  per-second history, adapter previews, notifications, favorites, search), to be used by the browser UI
- Temporary command line front end: `--adapter`, `--import`, `--list-adapters`
- Program icons are not looked up for now (the icon crate depended on the old UI toolkit)
- New browser UI in the style of opennetmon, served on 127.0.0.1 (default port 7380): capture page (adapter with
  activity curves, capture file, IPFIX collector, BPF filter, PCAP export), overview with tiles, live chart and top lists,
  mini view, connections with search, filters, pages and details, hosts, services and programs with favorites,
  notifications with badge and sounds, settings; light and dark design; English and German
- Sign-in with a start link (random access key, readable only by the user), session cookie, CSRF tokens,
  `Host`/`Origin` checks and security headers
- A second start only opens the browser for the running instance; new options `--port` and `--no-browser`
- Debian package with a systemd user service (`systemctl --user enable --now openpacketmon`)
- The capture page shows the link type, the addresses an IPFIX collector listens on and dropped packets
- Removed code that only the old UI used
- Desktop layout only (no phone layout): tables use the width of the window, compact connection table,
  notification badge as a rounded rectangle with at most "99+", unused styles from opennetmon removed
- One start link file per port, so instances on different ports do not overwrite each other's link
- Packet store: the packets of a capture are kept in memory (10,000 to 500,000 packets, at most 512 MiB, optionally
  headers only; settings section "Packet store")
- New page *Packets*: packet list with number, time, addresses, protocol, length and info, filtered by connection
  (optionally both directions), in pages or following the newest packets live
- Packet details: layer tree (Ethernet, VLAN, Linux cooked capture, ARP, IPv4, IPv6, TCP, UDP, ICMP, ICMPv6, IGMP) and
  hex dump; a field marks its bytes and a byte selects its field
- Download of the stored packets (all or the filtered ones) as a PCAP file; the connection page links to its packets
- New program `openpacketmon-app`: the UI in its own window (system web view) with a tray icon (open, mini view,
  browser, quit); it starts the server when needed and runs without capture rights. The start menu entry opens it.
  Closing the window keeps the capture running; downloads go to the download folder; links to other sites open in the
  browser. New setting to start it in the tray at sign-in. The package now depends on WebKitGTK 4.1.
- Application protocols in the packet list and details: DNS, mDNS and LLMNR (questions, answers and their records),
  DHCP and DHCPv6, TLS (records, Client Hello with server name, ALPN and versions, Server Hello, alerts), HTTP/1,
  SSDP, NTP and the QUIC header; a TLS Client Hello split over TCP segments is completed from the following packets
- Live packet list: new packets appear several times a second and the list follows them while it is scrolled to the end
  (pause button); other live parts of the pages update every two seconds instead of five
- Display filter for the packet list, e.g. `dns`, `tls.sni contains amazon`, `ip.addr == 192.168.1.0/24 and not arp`,
  with about 90 fields, checked while typing and explained when wrong; also used for the PCAP download and by
  "Packets of this connection". Replaces the source/destination/port fields
- Coloring rules in the packet list (errors and resets, TCP setup and teardown, DNS, local services, TLS/QUIC, HTTP …)
- Follow stream: the payload of a TCP or UDP connection in both directions as text or hex
- Live captures take whole packets when the packet store keeps them (before, packets were cut to 200 bytes unless they
  were exported, so server names, Follow stream and the PCAP download were incomplete); larger capture buffer then
- The capture page chooses what the packet store keeps: whole packets, headers only or nothing, and how many packets
- TCP analysis of the stored packets: retransmissions, gaps (previous segment not captured), segments out of order,
  duplicate ACKs, zero windows, keep-alives and window updates are marked in the packet list and details and can be
  filtered (`tcp.analysis.flags`, `tcp.analysis.retransmission` …); round trip times of ACKs, handshake times and DNS
  response times, with links between a packet and the one it acknowledges or answers
- Time values in the display filter, in seconds or with a unit (`dns.time > 100ms`, `tcp.analysis.ack_rtt > 0.2`)
- New page *Statistics*: for all stored packets or those of a display filter a traffic chart, the protocol hierarchy,
  the findings of the TCP analysis and the response times, each linked to its packets
- The TCP payload length counts the bytes on the wire also for packets kept with headers only
- Packet list and connections: first, previous, nearby page numbers, next and last page, and a field to type the page;
  the packet list has "Go to packet" (Ctrl+G), which opens the page with that packet (or the next one that matches the
  filter) and marks it
- The statistics page no longer jumps back to the top while it updates during a capture: its parts reload one by one,
  the chart takes the new values and keeps a zoom, and live pages keep their scroll position
- The top lists of the overview stay inside their cards: long names and paths end in "…" with the full text in the
  tooltip; the full lists wrap long paths. Program names cut by Linux to 15 characters are completed from the path
  (`evolution-calendar-factory` instead of `evolution-calen`)
- Violet sidebar: a gradient with the current page as a light tile in the light design; in the dark design a much darker
  violet sidebar and surfaces with a slight violet tint
- The packet list is one continuous list instead of pages, as in Wireshark: it scrolls through all packets that match the
  display filter, loads only the rows in view, follows new packets at the end during a capture, and jumps with
  "Go to packet" (Ctrl+G), Ctrl+Home/End, Page Up/Down and the arrow keys; back from the packet details it returns to the
  same packet
- Connections are filtered with one filter field in the grammar of the display filter instead of eleven separate fields,
  with fields of their own (`ip.addr`, `port`, `proto`, `direction`, `service`, `domain`, `as.name`, `country`,
  `program`, `bytes`, `favorite` …), checked while typing and explained in a help section
- Favorites are hosts, programs and now IP addresses (typed in the settings or starred on the connection page);
  services can no longer be favorites
- Notifications only report favorites with new traffic; the data threshold and blacklist notifications and the
  forwarding to a URL were removed. Old settings files still load
- The IP blacklist was removed (setting, marking of connections, filter field); favorite addresses take its place
- The mini view was removed, also from the tray menu of `openpacketmon-app`
- Fields, choices and buttons have one height per bar (36 px in filter bars and page heads, 30 px in card heads), choices
  look like the fields instead of the grey box of the system, and card heads are lower
- `NOTICE` lists the files of Sniffnet changed for openpacketmon, and each of them says so at its top (Apache License
  section 4(b)); `dev/check-notices.sh` finds changed files of Sniffnet without such a notice
- The connection page has a bar under its title with Packets, Statistics (new: the statistics of this connection),
  Connections of the address and Connections of the host; latency is measured from its line in the remote side card
- Service names come from the IANA Service Name and Transport Protocol Port Number Registry (public domain) instead of
  nmap; `dev/update-services.py` regenerates the list. Some names changed (`bootps`/`bootpc` instead of `dhcps`/`dhcpc`,
  `ssdp` instead of `upnp`, `submissions` for port 465), and nmap's names for trojans and unofficial ports are gone:
  such connections show the service `?` or the name IANA registered for the port
- No more built-in MaxMind GeoLite2 databases (the binary is about 20 MB smaller): *Settings → Geolocation* downloads the
  free DB-IP Lite databases (country and ASN, CC BY 4.0) on request, shows their date and warns after 60 days; MaxMind
  files of `geoipupdate` in `/var/lib/GeoIP` and own MMDB files still work. New databases apply without a restart to the
  addresses found afterwards. Overview and hosts point to the settings while there is no database; with DB-IP data the
  sidebar links to db-ip.com, as its license asks
- `THIRD-PARTY-LICENSES.txt` lists the licenses of all compiled-in crates, uPlot, the service names and the geolocation and
  test data (generated by `dev/third-party-licenses.py`); the package installs it with NOTICE to
  `/usr/share/doc/openpacketmon/`, the sidebar links to both (*Licenses*), and `dev/build-deb.sh` stops when the file
  is out of date or not committed
- `cargo deny` checks licenses, sources and RustSec advisories of all dependencies (`deny.toml`); the CI runs it
  instead of `cargo audit` and checks that `THIRD-PARTY-LICENSES.txt` is up to date
- `SECURITY.md`, `CONTRIBUTING.md`, templates for pull requests and issues, and a DCO sign-off check for pull requests
