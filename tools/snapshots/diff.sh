#!/usr/bin/env bash
# Shows which renders of the corpus moved between a base revision and the working tree
# (docs-port/ORACLE-PLAN.md, step 4). The files are those whose line in the snapshot manifest,
# crates/ferrigura/tests/snapshots/svg.tsv, differs between the base and the working tree, so run it
# after `UPDATE_SNAPSHOTS=1 cargo test --release --test snapshots`:
#
#   tools/snapshots/diff.sh [base]      the base defaults to main
#
# Each changed file is rendered under the test's three configurations and four colour schemes, with
# the base's renderer and with the working tree's, into target/snapshots/{base,head}/<file>/
# <configuration>-<scheme>.svg, and the renders that differ are listed. Compare them side by side,
# or check the new output against the reference with `pnpm port:geometry <type> --filter <file>`.
set -euo pipefail

base="${1:-main}"
root="$(git rev-parse --show-toplevel)"
cd "$root"
manifest=crates/ferrigura/tests/snapshots/svg.tsv
now=1791000000000

changed="$(diff <(git show "$base:$manifest" 2>/dev/null || true) "$manifest" |
  sed -n 's/^> \([^	]*\)	.*/\1/p' || true)"
if [ -z "$changed" ]; then
  echo "no corpus file changed against $base"
  exit 0
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/ferrigura-snapshots.XXXXXX")"
cleanup() {
  git worktree remove --force "$work/base" >/dev/null 2>&1 || true
  rm -rf "$work"
}
trap cleanup EXIT
git worktree add --quiet --detach "$work/base" "$base"

echo "building the renderer of $base and of the working tree" >&2
CARGO_TARGET_DIR="$root/target/snapshots/build-base" \
  cargo build --quiet --release --example render --manifest-path "$work/base/Cargo.toml"
cargo build --quiet --release --example render
base_bin="$root/target/snapshots/build-base/release/examples/render"
head_bin="$root/target/release/examples/render"

echo '{}' >"$work/none.json"
echo '{"theme": "redux-dark-color"}' >"$work/dark.json"
cp crates/ferrigura/tests/snapshots/override.json "$work/override.json"

out="$root/target/snapshots"
rm -rf "$out/base" "$out/head"
count=0
while IFS= read -r file; do
  [ -n "$file" ] || continue
  count=$((count + 1))
  moved=""
  for config in none override dark; do
    for scheme in none light dark auto; do
      args=(--now "$now" --config "$work/$config.json")
      if [ "$scheme" != none ]; then
        args+=(--scheme "$scheme")
      fi
      for side in base head; do
        mkdir -p "$out/$side/$file"
        if [ "$side" = base ]; then bin="$base_bin" src="$work/base/$file"; else bin="$head_bin" src="$file"; fi
        target="$out/$side/$file/$config-$scheme.svg"
        if [ -f "$src" ]; then
          "$bin" "${args[@]}" <"$src" >"$target" 2>&1 || true
        else
          echo "(no such file at $side)" >"$target"
        fi
      done
      if ! cmp -s "$out/base/$file/$config-$scheme.svg" "$out/head/$file/$config-$scheme.svg"; then
        moved="$moved $config-$scheme"
      fi
    done
  done
  echo "$file:${moved:- no render differs (only the hash input changed)}"
done <<<"$changed"
echo "$count files; the renders are in $out/base and $out/head" >&2
