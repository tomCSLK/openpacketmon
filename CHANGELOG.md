# Changelog

All notable changes to openpacketmon are documented in this file.

openpacketmon starts from Sniffnet `main` at commit `ab4bf739d464` (2026-10-20, after Sniffnet v1.5.1);
the history of this repository starts with one initial commit of the cleaned-up fork.
The history of Sniffnet itself is in its own changelog: <https://github.com/GyulyVGC/sniffnet/blob/main/CHANGELOG.md>.

## [Unreleased]

## [0.4.0] - 2026-11-02

First public version. This list describes what differs from Sniffnet.

### Interface

- New native interface (egui, wgpu) in the style of opennetmon instead of Sniffnet's desktop UI: capture page (adapter with
  activity curves, capture file, IPFIX collector, BPF filter, PCAP export), overview with tiles, live chart and top lists,
  connections, hosts, services, programs, notifications with badge and sounds, and settings; light and dark design with a
  violet sidebar; English and German; the font Inter is built in
- The program works like Wireshark: closing the window ends it, with a question while a capture is running; there is no
  background service, tray icon or autostart. Several windows can run side by side
- A back arrow on every page, also with Alt+Left and the back button of the mouse. Statistics, packet list and saving as
  PCAP can take the connection filter, so the Connections page opens the statistics of everything its search selects
- The filter fields suggest fields, operators and connectors while typing, with short explanations, and in the
  connection filter the values of the capture
- AI assistants can read the capture over MCP (`openpacketmon mcp`): state, overview, connections, packets without their contents and statistics, off until turned on in the settings
- Packet contents for AI assistants (follow stream, bytes of a packet) with a switch of their own that ends with the window, secrets in HTTP headers, URL queries and passwords replaced by `x`; the sidebar shows while assistants have access
- The window and `openpacketmon mcp` prove to each other that they know the access token without sending it, and the filters of an assistant see hidden values as hidden
- Adapters without IP traffic (Bluetooth, NFLOG, NFQUEUE) are greyed out on the capture page with a note instead of an
  activity curve and cannot be chosen; a capture on such an adapter or file stops with the reason instead of showing nothing
- Options `--adapter`, `--import` and `--list-adapters`
- Debian package with a start menu entry
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

- Packet store: the packets of a capture are kept in memory (10,000 to 2 million packets, 256 MiB to 2 GiB), whole or as
  headers only, chosen on the capture page or in the settings; live captures then take whole packets
- New page *Packets*: one continuous list, as in Wireshark, that scrolls through all packets matching the display filter,
  loads only the rows in view and follows new packets at the end during a capture; "Go to packet" (Ctrl+G), Ctrl+Home/End,
  Page Up/Down and the arrow keys; back from the packet details the list is where it was
- Fields, layers and bytes of the packet details can be copied with Ctrl+C or the context menu; control characters from the
  packets are replaced in what is copied
- Packet details: layer tree and hex dump, a field marks its bytes and a byte selects its field. Layers: Ethernet, VLAN,
  Linux cooked capture, ARP, IPv4, IPv6, TCP, UDP, ICMP, ICMPv6, IGMP, DNS, mDNS, LLMNR, DHCP, DHCPv6, TLS (with server
  name, ALPN and versions, also over several TCP segments), HTTP/1, SSDP, NTP and the QUIC header
- Display filter with about 90 fields, e.g. `dns`, `tls.sni contains amazon`, `ip.addr == 192.168.1.0/24 and not arp`,
  `dns.time > 100ms`; checked while typing and explained when wrong
- Coloring rules in the packet list (errors and resets, TCP setup and teardown, DNS, local services, TLS/QUIC, HTTP …)
- Follow stream: the payload of a TCP or UDP connection in both directions as text or hex; for long connections it reads only the
  headers of each packet
- Saving the stored packets (all or the filtered ones) as a PCAP file
- TCP analysis of the stored packets: retransmissions, gaps, segments out of order, duplicate ACKs, zero windows,
  keep-alives and window updates are marked and can be filtered (`tcp.analysis.flags`, `tcp.analysis.retransmission` …);
  round trip times of ACKs, handshake times and DNS response times, with links between a packet and the one it
  acknowledges or answers
- New page *Statistics*: for all stored packets, a display filter or one connection a traffic chart, the protocol hierarchy,
  the findings of the TCP analysis and the response times, each linked to its packets

### Changed from Sniffnet

- Capture rights moved to the small helper `openpacketmon-capture`: it opens the adapter, drops its rights for good and
  passes the packets on without looking into them, so the program that parses foreign traffic runs without special rights.
  Even the capture filter is compiled after the rights are gone.
  The helper ends with the program. The package sets the capabilities on the helper instead of on `openpacketmon`
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

- Sniffnet's desktop UI (iced), charts (plotters), fonts, icons, flags, themes, sound files and the 14 UI translations; the
  sounds are now generated by the program
- The IP blacklist, and the data threshold and blacklist notifications and their forwarding to a URL
- Sniffnet's Windows (MSI) and macOS packaging. Windows has its own signed installer instead (Inno Setup,
  `dev/build-installer.ps1`, needs Npcap), added to the 0.4.0 release on 2026-11-03; no macOS version
- Files that only apply to the Sniffnet project (funding, website, audits, upstream CI)
- Program icons are not looked up (the icon crate depended on the old UI toolkit)

### Project

- `NOTICE` lists the files of Sniffnet changed for openpacketmon, and each of them says so at its top (Apache License
  section 4(b)); `dev/check-notices.sh` finds changed files of Sniffnet without such a notice
- `THIRD-PARTY-LICENSES.txt` lists the licenses of all compiled-in crates (with those of the fallback fonts built into
  egui), the font Inter, the service names and the geolocation and test data (generated by `dev/third-party-licenses.py`); the package installs it with NOTICE to
  `/usr/share/doc/openpacketmon/`, and the sidebar links to both (*Licenses*)
- `cargo deny` checks licenses, sources and RustSec advisories of all dependencies (`deny.toml`), also in the CI
- `SECURITY.md`, `CONTRIBUTING.md`, templates for pull requests and issues, and a DCO sign-off check for pull requests
- `dev/build-deb.sh` builds the package; `dev/release.sh` checks, builds and publishes a release
