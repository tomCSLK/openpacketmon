# Contributing to openpacketmon

Thanks for your interest! openpacketmon is a fork of [Sniffnet](https://github.com/GyulyVGC/sniffnet) with a browser interface and a
small packet analyzer. It runs on Linux for now. Testing it on your machine and telling us what breaks is a valuable contribution.

## Reporting bugs and ideas

- **Bugs:** open an issue with the bug report form. Include the version, how you installed it, your distribution and, for capture
  problems, the kind of adapter or file. Never attach captures with private traffic; reduce them to the packets that show the problem.
- **Ideas:** use the feature request form.
- **Security vulnerabilities:** please do **not** open a public issue, see [`SECURITY.md`](SECURITY.md).
- A bug that also happens in Sniffnet belongs to [Sniffnet's issues](https://github.com/GyulyVGC/sniffnet/issues) as well.

For a larger change, open an issue first so we can agree on the approach before you invest the time. Small pull requests are easier
to review than large ones.

## Development

Requirements: a current stable Rust toolchain (edition 2024), `libpcap-dev`, `libasound2-dev` (sound of the native window) and, for the window program, `libwebkit2gtk-4.1-dev`.
Building is described in the README under [Building](README.md#building). Before you open a pull request, all of these must pass:

```bash
cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo deny --workspace check
```

The latency tests need unprivileged ICMP sockets (`sudo sysctl -w net.ipv4.ping_group_range="0 2147483647"`). CI runs the same
checks on every pull request; `cargo deny` (licenses, sources and RustSec advisories, rules in `deny.toml`) also runs weekly.
Install it with `cargo install cargo-deny --locked`.

## Conventions

- **Language:** code, identifiers, code comments, commit messages, the README and the changelog are English; only the internal
  planning files `PLAN.md` and `CLAUDE.md` are German.
- **UI texts** only through the Fluent catalogs in `locales/<code>/app.ftl`, never as fixed text in templates, Rust or JavaScript.
  Add every new message in German (`de`) and English (`en`).
- **No inline styles or scripts:** the Content Security Policy forbids them. Styles go into `static/app.css`, scripts into `static/`.
- **No code from Wireshark.** Wireshark is GPL-licensed; do not copy its code, dissectors or display filter grammar, and do not
  translate them. Write dissectors from the protocol specifications (RFCs). Test packets are written by hand or generated in the tests.
- **Files of Sniffnet:** keep their license and copyright notices. When you change a file that is still unchanged from Sniffnet
  (or copy code from one), add the two lines "Modified for openpacketmon (2026), see NOTICE. Based on …" at its top, as in the other
  changed files, and list it in `NOTICE`. `dev/check-notices.sh` finds changed files without such a notice.
- **Dependencies:** only maintained crates under a permissive license (see `deny.toml`). Give a short reason in the commit message.
  After any change to `Cargo.lock`, run `python3 -I dev/third-party-licenses.py` and commit `THIRD-PARTY-LICENSES.txt`.
- **No third-party assets** (images, sounds, fonts, data) without a clear license that allows redistribution.
- Small, self-contained commits; every commit should build and pass the tests.

## Use of AI tools

Large parts of openpacketmon were written with the help of an AI assistant (Claude). If you use AI tools for a contribution, say so
in the pull request, review the result yourself and make sure you can explain every change. Contributions to Sniffnet itself follow
[its own guidelines](https://github.com/GyulyVGC/sniffnet/blob/main/CONTRIBUTING.md).

## Developer Certificate of Origin (DCO)

openpacketmon uses the [Developer Certificate of Origin 1.1](https://developercertificate.org/) instead of a contributor license
agreement. By signing off a commit you certify that you wrote the change, or otherwise have the right to submit it under the
project's license.

Sign off every commit by adding a line with your name and the e-mail address of the commit author:

```
Signed-off-by: Jane Doe <jane@example.org>
```

`git commit -s` adds it for you. To sign off commits you already made: `git commit --amend -s` for the last one, or
`git rebase --signoff main` for all commits of your branch, then force-push the branch. CI checks every commit of a pull request.

## License

openpacketmon is licensed under the [MIT License](LICENSE-MIT) or the [Apache License 2.0](LICENSE-APACHE), at your option. Unless
you explicitly state otherwise, any contribution you submit is licensed the same way, without any additional terms or conditions.
