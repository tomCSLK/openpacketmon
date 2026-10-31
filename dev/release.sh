#!/usr/bin/env bash
# Builds, checks and (optionally) publishes a GitHub release of openpacketmon: the .deb package, the license list and
# checksums.
#
#   dev/release.sh                                  # build and check only; result in target/dist/
#   dev/release.sh --publish --notes notes.md       # additionally tag vX.Y.Z, push the tag, create the release
#   dev/release.sh --publish --notes notes.md --prerelease   # the same, marked as pre-release
#
# The version comes from Cargo.toml (bump it there, in app/Cargo.toml and in capture/Cargo.toml and commit first); CHANGELOG.md needs a section
# "## [X.Y.Z]". Before anything is published the script requires a public repository, a clean work tree on main that
# matches origin/main, a green CI run for this commit and a version without an existing tag. Every run checks fmt,
# clippy, tests, cargo deny, THIRD-PARTY-LICENSES.txt, the notices of changed Sniffnet files and the contents of the .deb.
# Release notes are in English.
#
# The release is called vX.Y.Z and not "openpacketmon X.Y.Z": the update check reads this name as a version
# (src/utils/check_updates.rs) and would never report a release with another one.
set -euo pipefail

cd "$(dirname "$0")/.."
REPO=tomCSLK/openpacketmon
PUBLISH=no
NOTES=""
PRERELEASE=()
while [ $# -gt 0 ]; do
  case "$1" in
    --publish) PUBLISH=yes; shift ;;
    --notes) NOTES="${2:?--notes needs a file}"; shift 2 ;;
    --prerelease) PRERELEASE=(--prerelease); shift ;;
    -h|--help) sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
done

step() { printf '==> %s\n' "$*"; }
fail() { printf 'error: %s\n' "$*" >&2; exit 1; }

VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
APP_VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' app/Cargo.toml | head -n 1)
CAPTURE_VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' capture/Cargo.toml | head -n 1)
TAG="v$VERSION"
DEB="openpacketmon_$VERSION-1_amd64.deb"
DIST="target/dist"

step "checking the repository for $TAG"
[ "$APP_VERSION" = "$VERSION" ] || fail "app/Cargo.toml has version $APP_VERSION, Cargo.toml $VERSION"
[ "$CAPTURE_VERSION" = "$VERSION" ] || fail "capture/Cargo.toml has version $CAPTURE_VERSION, Cargo.toml $VERSION"
grep -qF "## [$VERSION]" CHANGELOG.md || fail "CHANGELOG.md has no section ## [$VERSION]"
if [ "$PUBLISH" = yes ]; then
  [ -n "$NOTES" ] && [ -f "$NOTES" ] || fail "--publish needs --notes FILE with the release notes"
  [ "$(gh repo view "$REPO" --json visibility --jq .visibility)" = PUBLIC ] || fail "$REPO is not public yet"
  [ -z "$(git status --porcelain)" ] || fail "work tree is not clean"
  [ "$(git rev-parse --abbrev-ref HEAD)" = main ] || fail "not on main"
  git fetch -q origin main
  [ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] || fail "main differs from origin/main (push or pull first)"
  [ "$(gh run list -R "$REPO" --workflow CI --commit "$(git rev-parse HEAD)" --json conclusion --jq '.[0].conclusion')" = success ] \
    || fail "no successful CI run for this commit (wait for it or check it)"
  git rev-parse -q --verify "refs/tags/$TAG" >/dev/null && fail "tag $TAG already exists locally (bump the version in Cargo.toml)"
  if git ls-remote --exit-code --tags origin "refs/tags/$TAG" >/dev/null; then fail "tag $TAG already exists on GitHub"; fi
elif [ -n "$(git status --porcelain)" ]; then
  echo "note: uncommitted changes; they are part of this build"
fi

step "checking code and licenses"
cargo fmt --all --check
cargo clippy --locked -- -D warnings
cargo test --locked --workspace --quiet
python3 -I dev/third-party-licenses.py --check
command -v cargo-deny >/dev/null || fail "cargo-deny is missing (cargo install cargo-deny --locked --version 0.20.2)"
cargo deny --workspace --locked check
dev/check-notices.sh

step "building the release binaries"
cargo build --release --locked --workspace   # cargo deb only builds openpacketmon itself
for program in openpacketmon openpacketmon-app; do
  [ "$("target/release/$program" --version)" = "$program $VERSION" ] \
    || fail "$program reports $("target/release/$program" --version), expected $VERSION"
done

step "packaging $DIST/$DEB"
rm -rf "$DIST" && mkdir -p "$DIST"
cargo deb --no-build --quiet >/dev/null
cp "target/debian/$DEB" THIRD-PARTY-LICENSES.txt "$DIST/"
CONTENTS=$(dpkg-deb --contents "$DIST/$DEB")
for f in usr/bin/openpacketmon usr/bin/openpacketmon-app usr/share/applications/openpacketmon.desktop \
         usr/lib/systemd/user/openpacketmon.service usr/share/doc/openpacketmon/copyright \
         usr/share/doc/openpacketmon/NOTICE usr/share/doc/openpacketmon/THIRD-PARTY-LICENSES.txt; do
  grep -q " \./$f\$" <<<"$CONTENTS" || fail "$DEB does not contain /$f"
done
[ "$(dpkg-deb --field "$DIST/$DEB" Version)" = "$VERSION-1" ] || fail "$DEB has the wrong version"
DEPENDS=$(dpkg-deb --field "$DIST/$DEB" Depends)
for package in "libc6 (>=" libcap2-bin libwebkit2gtk-4.1-0; do
  grep -qF "$package" <<<"$DEPENDS" || fail "$DEB does not depend on $package"
done
(cd "$DIST" && sha256sum "$DEB" THIRD-PARTY-LICENSES.txt > SHA256SUMS)
cat "$DIST/SHA256SUMS"

if [ "$PUBLISH" = no ]; then
  step "done (not published; add --publish --notes FILE to tag and release $TAG)"
  exit 0
fi

step "tagging $TAG and creating the GitHub release"
git tag -a "$TAG" -m "openpacketmon $VERSION"
git push -q origin "$TAG"
gh release create "$TAG" --verify-tag "${PRERELEASE[@]}" --title "$TAG" --notes-file "$NOTES" \
  "$DIST/$DEB" "$DIST/THIRD-PARTY-LICENSES.txt" "$DIST/SHA256SUMS"

step "verifying the uploaded files"
VERIFY=$(mktemp -d)
trap 'rm -rf "$VERIFY"' EXIT
gh release download "$TAG" --dir "$VERIFY"
(cd "$VERIFY" && sha256sum -c SHA256SUMS)
step "published $TAG"
