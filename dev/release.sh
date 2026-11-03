#!/usr/bin/env bash
# Builds, checks and (optionally) publishes a GitHub release of openpacketmon: the .deb package, the license list and
# checksums, and the signed Windows installer when it is given.
#
#   dev/release.sh                                  # build and check only; result in target/dist/
#   dev/release.sh --publish --notes notes.md       # additionally tag vX.Y.Z, push the tag, create the release
#   dev/release.sh --publish --notes notes.md --prerelease   # the same, marked as pre-release
#   dev/release.sh --windows openpacketmon-X.Y.Z-setup.exe [--publish --notes notes.md]   # the release with the installer
#   dev/release.sh --add-windows target/installer/openpacketmon-X.Y.Z-setup.exe [--publish] # add it to the existing release
#
# The Windows installer is built and signed on the Windows computer (dev\build-installer.ps1 -CertSha1 <thumbprint>,
# docs/windows.md 6.1); this script does not build it. --windows takes it into a new release built here (copy the file
# over first). --add-windows adds it to the existing release vX.Y.Z and rewrites SHA256SUMS without building anything; it
# also runs in Git Bash on the Windows computer. Without --publish both only check: the file name has to carry the
# version, the signature has to be valid (signtool, or osslsigncode under Linux), and for --add-windows the program code
# must not have changed since the tag, so that the installer matches the tagged source.
#
# The version comes from Cargo.toml (bump it there and in capture/Cargo.toml and commit first); CHANGELOG.md needs a section
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
WINDOWS=""
ADD_WINDOWS=no
while [ $# -gt 0 ]; do
  case "$1" in
    --publish) PUBLISH=yes; shift ;;
    --notes) NOTES="${2:?--notes needs a file}"; shift 2 ;;
    --prerelease) PRERELEASE=(--prerelease); shift ;;
    --windows) WINDOWS="${2:?--windows needs the installer}"; shift 2 ;;
    --add-windows) WINDOWS="${2:?--add-windows needs the installer}"; ADD_WINDOWS=yes; shift 2 ;;
    -h|--help) sed -n '2,25p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
done

step() { printf '==> %s\n' "$*"; }
fail() { printf 'error: %s\n' "$*" >&2; exit 1; }

VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
CAPTURE_VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' capture/Cargo.toml | head -n 1)
TAG="v$VERSION"
DEB="openpacketmon_$VERSION-1_amd64.deb"
DIST="target/dist"
SETUP="openpacketmon-$VERSION-setup.exe"

# signtool from PATH or the newest Windows SDK (Git Bash under Windows), else osslsigncode (Linux: apt install osslsigncode)
check_signature() {
  local file="$1" signtool=""
  if command -v signtool.exe >/dev/null; then
    signtool=signtool.exe
  else
    signtool=$(find "/c/Program Files (x86)/Windows Kits/10/bin" -path '*/x64/signtool.exe' 2>/dev/null | sort -V | tail -n 1 || true)
  fi
  if [ -n "$signtool" ]; then
    # MSYS2_ARG_CONV_EXCL: Git Bash would turn /pa into a path
    MSYS2_ARG_CONV_EXCL='*' "$signtool" verify /pa /all "$file" || fail "$file has no valid signature"
  elif command -v osslsigncode >/dev/null; then
    osslsigncode verify -in "$file" || fail "$file has no valid signature"
  else
    fail "cannot check the signature of $file: install osslsigncode (Linux) or the Windows SDK (signtool)"
  fi
}

check_installer() {
  [ -f "$WINDOWS" ] || fail "no file $WINDOWS"
  [ "$(basename "$WINDOWS")" = "$SETUP" ] || fail "$WINDOWS is not $SETUP (built for another version?)"
  step "checking the signature of $SETUP"
  check_signature "$WINDOWS"
}

if [ "$ADD_WINDOWS" = yes ]; then
  # adds the installer to the existing release $TAG; nothing is built
  step "adding $SETUP to the release $TAG"
  [ "$CAPTURE_VERSION" = "$VERSION" ] || fail "capture/Cargo.toml has version $CAPTURE_VERSION, Cargo.toml $VERSION"
  git rev-parse -q --verify "refs/tags/$TAG" >/dev/null || git fetch -q origin "refs/tags/$TAG:refs/tags/$TAG" \
    || fail "no tag $TAG"
  gh release view "$TAG" -R "$REPO" >/dev/null || fail "no release $TAG on GitHub"
  # the installer comes from the work tree, so everything but docs and packaging scripts has to be as in the tag
  CHANGED=$(git diff --name-only "$TAG" -- . ':!*.md' ':!docs' ':!dev' ':!resources/packaging' ':!.github')
  [ -z "$CHANGED" ] || fail "the program code changed since $TAG, the installer would not match the release: $CHANGED"
  check_installer
  ADD=$(mktemp -d)
  trap 'rm -rf "$ADD"' EXIT
  gh release download "$TAG" -R "$REPO" --pattern SHA256SUMS --dir "$ADD"
  HASH=$(sha256sum "$WINDOWS" | cut -d ' ' -f 1)
  { grep -v "  $SETUP\$" "$ADD/SHA256SUMS" || true; printf '%s  %s\n' "$HASH" "$SETUP"; } > "$ADD/SHA256SUMS.new"
  mv "$ADD/SHA256SUMS.new" "$ADD/SHA256SUMS"
  cat "$ADD/SHA256SUMS"
  if [ "$PUBLISH" = no ]; then
    step "done (nothing uploaded; add --publish to add $SETUP and these checksums to $TAG)"
    exit 0
  fi
  cp "$WINDOWS" "$ADD/$SETUP"
  gh release upload "$TAG" -R "$REPO" --clobber "$ADD/$SETUP" "$ADD/SHA256SUMS"
  step "verifying the uploaded files"
  mkdir "$ADD/verify"
  gh release download "$TAG" -R "$REPO" --dir "$ADD/verify"
  (cd "$ADD/verify" && sha256sum -c SHA256SUMS)
  step "added $SETUP to $TAG; mention Windows in the release notes (gh release edit $TAG --notes-file FILE)"
  exit 0
fi

step "checking the repository for $TAG"
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
[ -z "$WINDOWS" ] || check_installer

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
[ "$(target/release/openpacketmon --version)" = "openpacketmon $VERSION" ] \
  || fail "openpacketmon reports $(target/release/openpacketmon --version), expected $VERSION"

step "packaging $DIST/$DEB"
rm -rf "$DIST" && mkdir -p "$DIST"
cargo deb --no-build --quiet >/dev/null
cp "target/debian/$DEB" THIRD-PARTY-LICENSES.txt "$DIST/"
CONTENTS=$(dpkg-deb --contents "$DIST/$DEB")
for f in usr/bin/openpacketmon usr/bin/openpacketmon-capture usr/share/applications/openpacketmon.desktop \
         usr/share/doc/openpacketmon/copyright \
         usr/share/doc/openpacketmon/NOTICE usr/share/doc/openpacketmon/THIRD-PARTY-LICENSES.txt; do
  grep -q " \./$f\$" <<<"$CONTENTS" || fail "$DEB does not contain /$f"
done
[ "$(dpkg-deb --field "$DIST/$DEB" Version)" = "$VERSION-1" ] || fail "$DEB has the wrong version"
DEPENDS=$(dpkg-deb --field "$DIST/$DEB" Depends)
for package in "libc6 (>=" libcap2-bin libpcap libasound2 libxkbcommon0; do
  grep -qF "$package" <<<"$DEPENDS" || fail "$DEB does not depend on $package"
done
FILES=("$DEB")
if [ -n "$WINDOWS" ]; then
  cp "$WINDOWS" "$DIST/$SETUP"
  FILES+=("$SETUP")
fi
FILES+=(THIRD-PARTY-LICENSES.txt)
(cd "$DIST" && sha256sum "${FILES[@]}" > SHA256SUMS)
cat "$DIST/SHA256SUMS"

if [ "$PUBLISH" = no ]; then
  step "done (not published; add --publish --notes FILE to tag and release $TAG)"
  exit 0
fi

step "tagging $TAG and creating the GitHub release"
git tag -a "$TAG" -m "openpacketmon $VERSION"
git push -q origin "$TAG"
gh release create "$TAG" --verify-tag "${PRERELEASE[@]}" --title "$TAG" --notes-file "$NOTES" \
  "${FILES[@]/#/$DIST/}" "$DIST/SHA256SUMS"

step "verifying the uploaded files"
VERIFY=$(mktemp -d)
trap 'rm -rf "$VERIFY"' EXIT
gh release download "$TAG" --dir "$VERIFY"
(cd "$VERIFY" && sha256sum -c SHA256SUMS)
step "published $TAG"
