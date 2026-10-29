# Security policy

## Supported versions

Security fixes go into the next release; only the latest release is supported.

Dependencies are checked against the RustSec advisory database (`cargo deny`, rules in `deny.toml`) on every change and every
week.

## Reporting a vulnerability

Please report vulnerabilities **privately** and not as a public issue:

1. Open the [Security tab](https://github.com/tomCSLK/openpacketmon/security) of the repository.
2. Choose **Report a vulnerability** and describe the problem.

Helpful details: the affected version, how to reproduce it, what an attacker can achieve and under which conditions (for example:
another local user, a web page open in the same browser, crafted packets on the captured network). Do not include captures with
private traffic; reduce them to the packets that trigger the problem.

openpacketmon is maintained in spare time. You will get an answer as soon as possible; once a fix is released, the advisory is
published and you are credited if you want.

Vulnerabilities in code that openpacketmon took over from [Sniffnet](https://github.com/GyulyVGC/sniffnet) (see `NOTICE`) may
affect Sniffnet as well; we pass them on to its maintainer.

## Scope and known limitations

In scope: the web interface on `127.0.0.1` and its sign-in (start link, session cookie, CSRF tokens, `Host`/`Origin` checks), the
parsing of captured packets (dissectors, display filter, TCP analysis), the import of capture and MMDB files, the download of the
geolocation databases, the window program `openpacketmon-app`, the Debian package and the systemd user service.

Known and documented, not vulnerabilities in themselves:

- The web interface listens on `127.0.0.1` only and uses plain HTTP; it is meant for the user who runs it, not for remote access.
- The package gives `/usr/bin/openpacketmon` the capabilities `cap_net_raw` and `cap_net_admin`, so every local user can capture
  traffic with it, as with Wireshark's `dumpcap` in the group `wireshark`. Remove them with `setcap -r` if that is not wanted.
- The packet store keeps the captured packets in memory, including their contents, until the capture is stopped.
