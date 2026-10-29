#!/bin/sh
# Builds the Debian package target/debian/openpacketmon_<version>-1_amd64.deb with the version from Cargo.toml.
# Versions as in opennetmon: 0.x.y, bump y for small changes and x for larger steps, in Cargo.toml before a new package.
# apt skips a package whose version is already installed, so the script stops if Cargo.toml was not bumped
# (`--force` builds anyway; then install with `apt install --reinstall`).
set -eu
cd "$(dirname "$0")/.."

version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
installed=$(dpkg-query -W -f '${Version}' openpacketmon 2>/dev/null || true)
if [ "${1:-}" != --force ] && [ -n "$installed" ] && dpkg --compare-versions "$version-1" le "$installed"; then
  echo "error: version $version is not newer than the installed $installed; bump it in Cargo.toml first" >&2
  echo "       (or build anyway with --force and install with apt install --reinstall)" >&2
  exit 1
fi

# the licenses of the dependencies must match Cargo.lock and be committed (they are built in and packaged)
python3 -I dev/third-party-licenses.py
if ! git diff --quiet -- THIRD-PARTY-LICENSES.txt || [ -n "$(git ls-files --others -- THIRD-PARTY-LICENSES.txt)" ]; then
  echo "error: THIRD-PARTY-LICENSES.txt changed; check and commit it, then build again" >&2
  exit 1
fi

cargo build --release --workspace   # cargo deb only builds openpacketmon itself
rm -f target/debian/openpacketmon_*.deb
cargo deb --no-build

echo
echo "Install with: sudo apt install ./target/debian/openpacketmon_${version}-1_amd64.deb"
