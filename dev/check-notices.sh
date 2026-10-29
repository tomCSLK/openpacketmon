#!/bin/sh
# Checks the notices required by section 4(b) of the Apache License (see NOTICE): every file that also exists in
# Sniffnet at the commit openpacketmon started from and differs from it must say "Modified for openpacketmon" near
# its top or be listed in NOTICE. Renamed files and files with code copied from Sniffnet are not found; mark them
# by hand. Usage: dev/check-notices.sh [sniffnet-directory]
# Without an argument Sniffnet is cloned into a temporary directory (needs the network).
set -eu
cd "$(dirname "$0")/.."

base=ab4bf739d464
if [ $# -ge 1 ]; then
  upstream=$1
else
  upstream=$(mktemp -d)
  trap 'rm -rf "$upstream"' EXIT
  git clone --quiet --filter=blob:none --no-checkout https://github.com/GyulyVGC/sniffnet "$upstream"
  git -C "$upstream" checkout --quiet "$base"
fi

# Files at the same paths as Sniffnet's that were written anew for openpacketmon (after those of opennetmon), not
# derived from Sniffnet's: they share only the keys of GitHub's issue form format
replaced="
SECURITY.md
CONTRIBUTING.md
.github/ISSUE_TEMPLATE/bug_report.yml
.github/ISSUE_TEMPLATE/config.yml
.github/ISSUE_TEMPLATE/feature_request.yml
"

missing=$(git ls-files | while read -r f; do
  [ -f "$upstream/$f" ] || continue
  cmp -s "$f" "$upstream/$f" && continue
  printf '%s\n' "$replaced" | grep -qx "$f" && continue
  head -n 5 "$f" | grep -q "Modified for openpacketmon" && continue
  grep -qx "  $f" NOTICE && continue
  echo "$f"
done)

if [ -n "$missing" ]; then
  echo "error: changed files of Sniffnet without a notice (add the header or list them in NOTICE):" >&2
  echo "$missing" >&2
  exit 1
fi
echo "All changed files of Sniffnet carry a notice."
