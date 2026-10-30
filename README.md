<p align="center"><img src="docs/assets/openpacketmon-logo.jpg" alt="openpacketmon – packet monitoring" width="600"></p>

# openpacketmon

openpacketmon shows what your computer is talking to on the network: hosts, countries, services, programs and live traffic
charts. It runs as a small local service on your own computer and is operated from the browser, in the style of
[opennetmon](https://github.com/tomCSLK/opennetmon). The web server only listens on 127.0.0.1.

> **Status: early version.** Expect rough edges, and please report what you find.

Platform: Linux on x86-64 for now. The package is made for Ubuntu 24.04 or newer, Debian 13 or newer and systems based on
them; it needs glibc 2.39 or newer.

## Installation

Download the `.deb` from the [latest release](https://github.com/tomCSLK/openpacketmon/releases/latest) and install it:

```sh
sudo apt install ./openpacketmon_*.deb
```

The package also grants the capture capabilities, see [Security](#security). A new version is installed the same way;
openpacketmon shows when one is available.

## Based on Sniffnet

openpacketmon is a fork of [Sniffnet](https://github.com/GyulyVGC/sniffnet) by Giuliano Bellini and the Sniffnet contributors.
The packet capture, parsing and analysis come from Sniffnet. openpacketmon replaces the user interface and goes its own way
from here on; it is not affiliated with or endorsed by the Sniffnet project.

## Building

```sh
sudo apt install libpcap-dev libwebkit2gtk-4.1-dev
cargo build --release
```

This builds two programs: `openpacketmon` (the capture and the web server) and `openpacketmon-app` (the window
with the tray icon, without special rights).

Capturing needs raw socket rights. Either run as root or grant the capabilities to the binary:

```sh
sudo setcap cap_net_raw,cap_net_admin=eip target/release/openpacketmon
```

To build the Debian package (with start menu entry, icons and a systemd user service):

```sh
cargo install cargo-deb
cargo build --release --workspace   # cargo deb only builds openpacketmon itself
cargo deb
sudo apt install ./target/debian/openpacketmon_*.deb   # also sets the capture capabilities
```

`dev/build-deb.sh` does the same and stops if the version in `Cargo.toml` is not newer than the installed one, because apt
skips a package whose version is already installed, or if `THIRD-PARTY-LICENSES.txt` no longer matches `Cargo.lock`. Bump the version first (`0.x.y`: `y` for small changes, `x` for larger
steps).

## Usage

Start openpacketmon from the application menu: it opens in its own window with an icon in the tray
(`openpacketmon-app`). The window starts the local server (`http://127.0.0.1:7380`) if it is not running yet
and signs in with the server's start link. Closing the window keeps the capture running; the tray menu opens the
window again, opens the browser, or quits openpacketmon. *Settings → Desktop* can start it in the
tray at sign-in.

Without the window, `openpacketmon` starts the server and opens the browser with a start link that signs it in.
Starting it a second time only opens the browser again. Choose a network adapter, a capture file or an IPFIX collector on the first page and start
the capture; overview, connections, hosts, services, programs and notifications follow it live. The *Connections* page filters with one field in the same grammar as the display filter
(`port == 443 and country == US`, `program contains firefox`, `favorite`). Hosts, programs and IP addresses can be
favorites; a notification reports new traffic of a favorite. The *Packets* page
lists the single packets in one continuous list that follows the capture live (with "Go to packet"), with a display filter (`dns`, `tls.sni contains amazon`, `ip.addr == 192.168.1.0/24`)
and coloring rules; it shows their layers (including DNS, DHCP, TLS server names, HTTP, SSDP, NTP and QUIC) and bytes,
follows TCP and UDP streams and downloads the packets as a PCAP file for Wireshark. A TCP analysis marks
retransmissions, gaps, duplicate ACKs and zero windows and measures round trip, handshake and DNS response times; the
*Statistics* page shows the protocol hierarchy, these findings and a traffic chart for any display filter. The capture page also chooses
whether the packets are kept whole, as headers only (less memory) or not at all.

```sh
openpacketmon-app                    # window and tray icon (start menu entry)
openpacketmon-app --hidden           # tray icon only
openpacketmon                        # start the server and open the browser
openpacketmon --no-browser           # only start the server (prints the start link)
openpacketmon --adapter enp0s31f6    # start capturing on this adapter right away
openpacketmon --import capture.pcap  # analyze a PCAP file right away
openpacketmon --port 7381            # use another port
openpacketmon --list-adapters        # show the network adapters and exit
```

To keep it running in the background, enable the user service that comes with the package:

```sh
systemctl --user enable --now openpacketmon
```

openpacketmon can be stopped with Ctrl+C, with *Settings → Quit openpacketmon*, or with
`systemctl --user stop openpacketmon`.

### Countries and network operators

openpacketmon ships no geolocation data. *Settings → Geolocation → Download geolocation databases from DB-IP* fetches the
free [DB-IP](https://db-ip.com) databases "IP to Country Lite" and "IP to ASN Lite" (CC BY 4.0, about 10 MB) into
`~/.local/share/openpacketmon/geo`; this is the only time openpacketmon contacts db-ip.com, and a new edition comes out
every month. Alternatively openpacketmon uses MaxMind GeoLite2 files kept up to date by `geoipupdate` in `/var/lib/GeoIP`,
or any MMDB file chosen in the settings (MaxMind, DB-IP or IPinfo format). Without a database, countries and network
operators stay empty.

### Network access of openpacketmon itself

At startup openpacketmon asks the GitHub API for the latest version; no data about your network is sent, and *Settings →
Updates* switches this off. Apart from that it only contacts the network for the download above, for reverse DNS lookups of
the addresses it sees, and for the latency measurement you start on a connection page.

### Security

The pages are only reachable from this computer (127.0.0.1). Signing in uses a random access key in the start link,
which is stored in `$XDG_RUNTIME_DIR/openpacketmon/start-link-<port>` and readable only by your user; the session cookie is
`HttpOnly` and `SameSite=Strict`, forms carry a CSRF token, and requests with a foreign `Host` or `Origin` are
rejected (protection against DNS rebinding and cross-site requests). Please report vulnerabilities privately, see
[SECURITY.md](SECURITY.md).

## Contributing

Bug reports, tests on other distributions and small pull requests are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md).
openpacketmon was largely written with the help of an AI assistant (Claude).

## License and data sources

Dual licensed under the [MIT License](LICENSE-MIT) or the [Apache License 2.0](LICENSE-APACHE), at your option,
like Sniffnet. Copyright (c) 2022 Giuliano Bellini (Sniffnet), Copyright (c) 2026 Thomas Cieslik (openpacketmon).
[NOTICE](NOTICE) lists the files of Sniffnet that were changed for openpacketmon; each of them says so at its top.

The service names (`resources/services/services.txt`) come from the IANA Service Name and Transport Protocol Port
Number Registry, which is in the public domain (CC0 1.0); `dev/update-services.py` regenerates them.
The sample databases of IPinfo used by the tests are licensed under CC BY 4.0 (see `resources/test/README.md`).
The geolocation data is not part of openpacketmon; data downloaded from DB-IP is licensed under
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) ([IP Geolocation by DB-IP](https://db-ip.com)).
[THIRD-PARTY-LICENSES.txt](THIRD-PARTY-LICENSES.txt) lists the licenses of all crates compiled into openpacketmon and
openpacketmon-app, of the bundled uPlot and of the data above, with the license texts; `dev/third-party-licenses.py`
regenerates it from `Cargo.lock`. The package installs it to `/usr/share/doc/openpacketmon/`, and the link *Licenses* at
the bottom of the sidebar shows it together with NOTICE.
