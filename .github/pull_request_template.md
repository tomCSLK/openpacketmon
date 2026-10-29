<!-- Thanks for contributing! Please read CONTRIBUTING.md first. -->

## What and why

## Checklist

- [ ] Every commit is signed off (`git commit -s`, see the DCO section in CONTRIBUTING.md)
- [ ] `cargo fmt --all`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace` and `cargo deny --workspace check` pass
- [ ] UI texts only through `locales/de/app.ftl` and `locales/en/app.ftl` (both languages)
- [ ] No code, dissectors or filter grammar from Wireshark; changed files of Sniffnet carry the notice (`dev/check-notices.sh`)
- [ ] `THIRD-PARTY-LICENSES.txt` regenerated if `Cargo.lock` changed (`python3 -I dev/third-party-licenses.py`)
- [ ] `README.md` and `CHANGELOG.md` updated where behaviour changed
- [ ] Use of AI tools, if any, is mentioned above
