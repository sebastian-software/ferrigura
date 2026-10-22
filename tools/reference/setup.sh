#!/usr/bin/env bash
# Builds the reference that the port's harness checks ferrigura against: upstream Mermaid at the
# commit of tools/reference/pin, with the patches of tools/reference/patches applied in the order of
# their names (docs-port/ORACLE-PLAN.md). The build lands in target/reference/<commit>; its path is
# the only output on standard output, for the harness's REFERENCE_ROOT:
#
#   REFERENCE_ROOT="$(tools/reference/setup.sh)" pnpm port:diff flowchart
#
# A second run reuses the build while the pin and the patches are unchanged. Needs git, network
# access to the repository of the pin, and pnpm.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
dir="$root/tools/reference"
repository="$(sed -n 's/^repository //p' "$dir/pin")"
commit="$(sed -n 's/^commit //p' "$dir/pin")"
out="$root/target/reference/$commit"
stamp="$(cat "$dir/pin" "$dir"/patches/*.patch | shasum -a 256 | cut -d ' ' -f 1)"

if [ -f "$out/.reference-stamp" ] && [ "$(cat "$out/.reference-stamp")" = "$stamp" ]; then
  echo "$out"
  exit 0
fi

rm -rf "$out"
mkdir -p "$out"
git -C "$out" init --quiet
git -C "$out" fetch --quiet --depth 1 "$repository" "$commit"
git -C "$out" -c advice.detachedHead=false checkout --quiet FETCH_HEAD
for patch in "$dir"/patches/*.patch; do
  echo "applying $(basename "$patch")" >&2
  git -C "$out" apply "$patch"
done
(
  cd "$out"
  pnpm install --frozen-lockfile --ignore-scripts >&2
  pnpm --silent build:esbuild >&2
)
echo "$stamp" >"$out/.reference-stamp"
echo "$out"
