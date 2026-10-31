# Changelog

All notable changes to openpacketmon are documented in this file.

openpacketmon starts from Sniffnet `main` at commit `ab4bf739d464` (2026-10-20, after Sniffnet v1.5.1);
the history of this repository starts with one initial commit of the cleaned-up fork.
The history of Sniffnet itself is in its own changelog: <https://github.com/GyulyVGC/sniffnet/blob/main/CHANGELOG.md>.

## [Unreleased]

First public version. This list describes what differs from Sniffnet.

### Interface

- New browser UI in the style of opennetmon on 127.0.0.1 (default port 7380) instead of Sniffnet's desktop UI: capture
  page (adapter with activity curves, capture file, IPFIX collector, BPF filter, PCAP export), overview with tiles, live
  chart and top lists, connections, hosts, services, programs, notifications with badge and sounds, and settings; light
  and dark design with a violet sidebar; English and German; desktop layout only
- New program `openpacketmon-app`: the UI in its own window (system web view) with a tray icon (open, browser, quit). It
  starts the server when needed and runs without capture rights; closing the window keeps the capture running, downloads
  go to the download folder, and links to other sites open in the browser. A setting starts it in the tray at sign-in
- A back button on every page, also for the window without a browser bar. Statistics, packet list and PCAP download can
  take the connection filter, so the Connections page opens the statistics of everything its search selects
- The filter fields suggest fields, operators and connectors while typing, with short explanations, and in the
  connection filter the values of the capture
- Sign-in with a start link (random access key, readable only by the user), session cookie, CSRF tokens, `Host`/`Origin`
  checks and security headers
- A second start only opens the browser for the running instance, with one start link file per port; options `--port`,
  `--no-browser`, `--adapter`, `--import` and `--list-adapters`
- Debian package with a start menu entry and a systemd user service (`systemctl --user enable --now openpacketmon`)
- Connections are filtered with one field in the grammar of the display filter, with fields of their own (`ip.addr`,
  `port`, `proto`, `direction`, `service`, `domain`, `as.name`, `country`, `program`, `bytes`, `favorite` …), checked
  while typing and explained in a help section; pages with page numbers and a field to type the page
- The connection page shows the jumps to its packets, statistics and connections next to the values they belong to;
  latency is measured from its line in the remote side card
- Favorites are hosts, programs and IP addresses (typed in the settings or starred on the connection page); services
  can no longer be favorites
- Notifications only report favorites with new traffic
- The top lists of the overview stay inside their cards; program names cut by Linux to 15 characters are completed from
  the path (`evolution-calendar-factory` instead of `evolution-calen`)

### Packet analysis

- Packet store: the packets of a capture are kept in memory (10,000 to 500,000 packets, at most 512 MiB), whole or as
  headers only, chosen on the capture page or in the settings; live captures then take whole packets
- New page *Packets*: one continuous list, as in Wireshark, that scrolls through all packets matching the display filter,
  loads only the rows in view and follows new packets at the end during a capture; "Go to packet" (Ctrl+G), Ctrl+Home/End,
  Page Up/Down and the arrow keys; back from the packet details it returns to the same packet
- Packet details: layer tree and hex dump, a field marks its bytes and a byte selects its field. Layers: Ethernet, VLAN,
  Linux cooked capture, ARP, IPv4, IPv6, TCP, UDP, ICMP, ICMPv6, IGMP, DNS, mDNS, LLMNR, DHCP, DHCPv6, TLS (with server
  name, ALPN and versions, also over several TCP segments), HTTP/1, SSDP, NTP and the QUIC header
- Display filter with about 90 fields, e.g. `dns`, `tls.sni contains amazon`, `ip.addr == 192.168.1.0/24 and not arp`,
  `dns.time > 100ms`; checked while typing and explained when wrong
- Coloring rules in the packet list (errors and resets, TCP setup and teardown, DNS, local services, TLS/QUIC, HTTP …)
- Follow stream: the payload of a TCP or UDP connection in both directions as text or hex
- Download of the stored packets (all or the filtered ones) as a PCAP file
- TCP analysis of the stored packets: retransmissions, gaps, segments out of order, duplicate ACKs, zero windows,
  keep-alives and window updates are marked and can be filtered (`tcp.analysis.flags`, `tcp.analysis.retransmission` …);
  round trip times of ACKs, handshake times and DNS response times, with links between a packet and the one it
  acknowledges or answers
- New page *Statistics*: for all stored packets, a display filter or one connection a traffic chart, the protocol hierarchy,
  the findings of the TCP analysis and the response times, each linked to its packets

### Changed from Sniffnet

- Service names come from the IANA Service Name and Transport Protocol Port Number Registry (public domain) instead of
  nmap; `dev/update-services.py` regenerates the list. Some names changed (`bootps`/`bootpc` instead of `dhcps`/`dhcpc`,
  `ssdp` instead of `upnp`, `submissions` for port 465), and nmap's names for trojans and unofficial ports are gone:
  such connections show the service `?` or the name IANA registered for the port
- No built-in MaxMind GeoLite2 databases (the binary is about 20 MB smaller): *Settings → Geolocation* downloads the free
  DB-IP Lite databases (country and ASN, CC BY 4.0) on request, shows their date and warns after 60 days; MaxMind files of
  `geoipupdate` in `/var/lib/GeoIP` and own MMDB files still work. New databases apply without a restart to the addresses
  found afterwards. Without a database, overview and hosts point to the settings; with DB-IP data the sidebar links to
  db-ip.com, as its license asks
- The update check looks at the releases of this repository
- New name and logo; package, binary and configuration directory are called `openpacketmon`, independent of Sniffnet

### Removed from Sniffnet

- Sniffnet's desktop UI (iced), charts (plotters), sound playback (rodio), file dialogs, fonts, icons, flags, themes,
  sound files and the 14 UI translations; the sounds are now generated in the browser, and dependencies went from 701 to
  about 300 crates
- The IP blacklist, and the data threshold and blacklist notifications and their forwarding to a URL
- Windows and macOS packaging: Linux only for now
- Files that only apply to the Sniffnet project (funding, website, audits, upstream CI)
- Program icons are not looked up (the icon crate depended on the old UI toolkit)

### Project

- `NOTICE` lists the files of Sniffnet changed for openpacketmon, and each of them says so at its top (Apache License
  section 4(b)); `dev/check-notices.sh` finds changed files of Sniffnet without such a notice
- `THIRD-PARTY-LICENSES.txt` lists the licenses of all compiled-in crates, uPlot, the service names and the geolocation and
  test data (generated by `dev/third-party-licenses.py`); the package installs it with NOTICE to
  `/usr/share/doc/openpacketmon/`, and the sidebar links to both (*Licenses*)
- `cargo deny` checks licenses, sources and RustSec advisories of all dependencies (`deny.toml`), also in the CI
- `SECURITY.md`, `CONTRIBUTING.md`, templates for pull requests and issues, and a DCO sign-off check for pull requests
- `dev/build-deb.sh` builds the package; `dev/release.sh` checks, builds and publishes a release
