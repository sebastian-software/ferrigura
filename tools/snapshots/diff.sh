#!/usr/bin/env bash
# Shows which renders of the corpus moved between a base revision and the working tree
# (docs/decisions/0005-quality-gates.md). The files are those whose line in the snapshot manifest,
# crates/ferrigura/tests/snapshots/svg.tsv, differs between the base and the working tree, so run it
# after `UPDATE_SNAPSHOTS=1 cargo test --release --test snapshots`:
#
#   tools/snapshots/diff.sh [base]      the base defaults to main
#
# Each changed file is rendered under the test's three configurations and four color schemes, with
# the base's renderer and with the working tree's, into target/snapshots/{base,head}/<file>/
# <configuration>-<scheme>.svg, and the renders that differ are listed. A file of a type the design mode
# draws, a flowchart or a state diagram, also has its design renders, named
# design-<configuration>-<scheme>.svg. Compare them side by side, or check the new output
# against the reference with `pnpm -C tools/port port:geometry <type> --filter <file>`.
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
if ! git show "$base:$manifest" >"$work/base.tsv" 2>/dev/null; then
  : >"$work/base.tsv"
fi

echo "building the renderer of $base and of the working tree" >&2
CARGO_TARGET_DIR="$root/target/snapshots/build-base" \
  cargo build --quiet --release --example render --manifest-path "$work/base/Cargo.toml"
cargo build --quiet --release --example render
base_bin="$root/target/snapshots/build-base/release/examples/render"
head_bin="$root/target/release/examples/render"

echo '{}' >"$work/none.json"
echo '{"theme": "redux-dark-color"}' >"$work/dark.json"
cp crates/ferrigura/tests/snapshots/override.json "$work/override.json"

# Whether a renderer draws a file in the design mode: it does not refuse the file's type with the
# mode's option error. A file that is missing on that side has no renders there.
draws_design() {
  local bin="$1" src="$2" error
  [ -f "$src" ] || return 1
  error="$("$bin" --design true <"$src" 2>&1 >/dev/null || true)"
  case "$error" in
  *"design mode is not supported for this diagram type"*) return 1 ;;
  esac
}

out="$root/target/snapshots"
rm -rf "$out/base" "$out/head"
count=0
while IFS= read -r file; do
  [ -n "$file" ] || continue
  count=$((count + 1))
  moved=""
  # The design renders exist for a type the design mode draws on either side. The number of columns
  # does not tell: a state diagram's line and a sequence's both have two hashes.
  modes=(default)
  if draws_design "$base_bin" "$work/base/$file" || draws_design "$head_bin" "$file"; then
    modes+=(design)
  fi
  for mode in "${modes[@]}"; do
    prefix=""
    design=false
    if [ "$mode" = design ]; then
      prefix="design-"
      design=true
    fi
    for config in none override dark; do
      for scheme in none light dark auto; do
        args=(--now "$now" --config "$work/$config.json" --design "$design")
        if [ "$scheme" != none ]; then
          args+=(--scheme "$scheme")
        fi
        for side in base head; do
          mkdir -p "$out/$side/$file"
          if [ "$side" = base ]; then bin="$base_bin" src="$work/base/$file"; else bin="$head_bin" src="$file"; fi
          target="$out/$side/$file/$prefix$config-$scheme.svg"
          if [ -f "$src" ]; then
            "$bin" "${args[@]}" <"$src" >"$target" 2>&1 || true
          else
            echo "(no such file at $side)" >"$target"
          fi
        done
        if ! cmp -s "$out/base/$file/$prefix$config-$scheme.svg" "$out/head/$file/$prefix$config-$scheme.svg"; then
          moved="$moved $prefix$config-$scheme"
        fi
      done
    done
  done
  echo "$file:${moved:- no render differs (only the hash input changed)}"
done <<<"$changed"
echo "$count files; the renders are in $out/base and $out/head" >&2
