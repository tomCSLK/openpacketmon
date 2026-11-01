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
another local user, crafted packets on the captured network, a crafted capture file). Do not include captures with
private traffic; reduce them to the packets that trigger the problem.

openpacketmon is maintained in spare time. You will get an answer as soon as possible; once a fix is released, the advisory is
published and you are credited if you want.

Vulnerabilities in code that openpacketmon took over from [Sniffnet](https://github.com/GyulyVGC/sniffnet) (see `NOTICE`) may
affect Sniffnet as well; we pass them on to its maintainer.

## Scope and known limitations

In scope: the parsing of captured packets (dissectors, display filter, TCP analysis) and how the window shows them, the import of
capture and MMDB files, the download of the geolocation databases, the capture helper `openpacketmon-capture`, the access for AI
assistants (`openpacketmon mcp` and the listener of the window on `127.0.0.1`) and the Debian package.

Known and documented, not vulnerabilities in themselves:

- The package gives the helper `/usr/bin/openpacketmon-capture` the capabilities `cap_net_raw` and `cap_net_admin`, so every local
  user can capture traffic with it, as with Wireshark's `dumpcap` in the group `wireshark`. Remove them with `setcap -r` if that is not
  wanted. The helper drops them once the adapter is open and does not look into the packets; openpacketmon itself has no special rights.
- The access for AI assistants is off by default. Turned on, it is reachable by every local user on `127.0.0.1` and answers only to a
  token in a file readable by the user alone. What an assistant queries goes to its AI service, and text from a capture can come from
  other devices and may try to instruct the assistant (prompt injection); openpacketmon marks it as data, but only the assistant and
  its permission prompts can act on that.
- The packet store keeps the captured packets in memory, including their contents, until the capture is stopped or the program ends.
