#!/usr/bin/env bash
# Publishes the crates of a ferrigura release to crates.io from your machine, with your own
# crates.io login (docs-port/RELEASE.md, "The first publish").
#
#   tools/release/publish-crates.sh ferrigura-v0.1.0             publish the release
#   tools/release/publish-crates.sh --dry-run ferrigura-v0.1.0   check it and publish nothing
#
# The first release is published this way, not by .github/workflows/publish.yml: crates.io's Trusted
# Publishing can be enabled only for a crate that exists, and crates.io lets an account create only
# a few new crates in a row (about five, then one about every ten minutes), so twelve new crates
# take more than an hour. Later releases are the workflow's.
#
# The script checks out the tag in a temporary worktree, so the checkout it runs from stays as it
# is. It checks that the tag is on origin/main, that every crate has the tag's version, and that the
# workflow's list of crates names every publishable crate after the crates it depends on; then it
# packages and builds them all (`cargo publish --dry-run`). After you confirm by typing the version,
# it publishes the crates one by one in that order, as the workflow does: a version already on
# crates.io is skipped, so a run that stopped can be started again with the same tag. With
# --dry-run the ref may be any commit (main, HEAD), and the script stops before publishing.
#
# cargo authenticates as always, with the token of `cargo login` or CARGO_REGISTRY_TOKEN; the token
# needs the scopes publish-new and publish-update. The script does not read it.
#
# Needs git, cargo 1.90 or newer (for packaging several crates at once), curl and python3.
# RATE_LIMIT_WAIT (600), RETRY_DELAY (30) and INDEX_TIMEOUT (300) are seconds and can be set in the
# environment.

set -euo pipefail

RATE_LIMIT_WAIT="${RATE_LIMIT_WAIT:-600}"
RETRY_DELAY="${RETRY_DELAY:-30}"
INDEX_TIMEOUT="${INDEX_TIMEOUT:-300}"
# How often one crate waits for crates.io's limit on new crates: of twelve new crates, five go at
# once and the others one by one.
RATE_LIMIT_TRIES=10
REPOSITORY="sebastian-software/ferrigura"

die() {
  echo "error: $*" >&2
  exit 1
}

usage() {
  echo "usage: $0 [--dry-run] <tag>   (a release tag such as ferrigura-v0.1.0)" >&2
  exit 2
}

dry_run=0
if [ "${1:-}" = "--dry-run" ]; then
  dry_run=1
  shift
fi
[ "$#" -eq 1 ] || usage
ref="$1"

repo="$(git rev-parse --show-toplevel)"
cd "$repo"

# A release is published from its tag on main, never from a branch.
tag_version=""
if [[ "$ref" =~ ^ferrigura-v([0-9]+\.[0-9]+\.[0-9]+.*)$ ]]; then
  tag_version="${BASH_REMATCH[1]}"
  git fetch --quiet origin main "refs/tags/${ref}:refs/tags/${ref}" ||
    die "could not fetch main and the tag ${ref} from origin"
  git merge-base --is-ancestor "$ref" origin/main || die "${ref} is not on origin/main"
elif [ "$dry_run" -eq 0 ]; then
  die "a release is published from its tag, such as ferrigura-v0.1.0, not from ${ref}"
fi
commit="$(git rev-parse --verify --quiet "${ref}^{commit}")" || die "there is no commit ${ref}"

work="$(mktemp -d "${TMPDIR:-/tmp}/ferrigura-publish.XXXXXX")"
cleanup() {
  git -C "$repo" worktree remove --force "${work}/src" >/dev/null 2>&1 || true
  rm -rf "$work"
}
trap cleanup EXIT
trap 'exit 130' INT TERM
git worktree add --quiet --detach "${work}/src" "$commit"
cd "${work}/src"
# The builds go to the repository's target directory, so that a second run does not start over.
export CARGO_TARGET_DIR="${repo}/target/publish"

# The workflow's list is the order: the script publishes what the workflow would.
[ -f .github/workflows/publish.yml ] || die "${ref} has no .github/workflows/publish.yml"
crates="$(awk '/^ *crates: \|$/ { on = 1; next } on && /^ *[a-z0-9-]+$/ { print $1; next } on { exit }' \
  .github/workflows/publish.yml)"
[ -n "$crates" ] || die "found no list of crates in .github/workflows/publish.yml"

cargo metadata --locked --no-deps --format-version 1 >"${work}/metadata.json"
cat >"${work}/check.py" <<'PY'
import json
import sys

metadata = json.load(open(sys.argv[1]))
tag_version, order = sys.argv[2], sys.argv[3:]
members = set(metadata["workspace_members"])
packages = {p["name"]: p for p in metadata["packages"] if p["id"] in members}
# `publish = false` reads as an empty list of registries.
publishable = {name for name, p in packages.items() if p["publish"] != []}

errors = []
if sorted(order) != sorted(publishable):
    errors.append(
        "the workflow lists %s, but the publishable crates are %s"
        % (" ".join(sorted(order)), " ".join(sorted(publishable)))
    )
versions = sorted({packages[name]["version"] for name in order if name in packages})
version = versions[0] if len(versions) == 1 else None
if version is None:
    errors.append("the crates have different versions: " + ", ".join(versions))
elif tag_version and version != tag_version:
    errors.append("the crates are at %s, the tag at %s" % (version, tag_version))

before = set()
for name in order:
    for dep in packages.get(name, {}).get("dependencies", []):
        if dep["name"] not in packages:
            continue
        # A development dependency with only a path is left out of the package.
        if dep["kind"] == "dev" and dep["req"] == "*":
            continue
        if dep["name"] not in before:
            errors.append("%s comes before %s, which it depends on" % (name, dep["name"]))
        if version and dep["req"] != "^" + version:
            errors.append("%s requires %s %s, not %s" % (name, dep["name"], dep["req"], version))
    before.add(name)

if errors:
    sys.exit("\n".join("error: " + e for e in errors))
print(version)
PY
# shellcheck disable=SC2086 # the crate names are single words
version="$(python3 -I "${work}/check.py" "${work}/metadata.json" "$tag_version" $crates)" || exit 1

crate_index_path() {
  # The sparse index's layout: 1/name, 2/name, 3/{first character}/name, and
  # {first two}/{next two}/name for longer names, always in lower case.
  local crate
  crate="$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')"
  case "${#crate}" in
    1) echo "1/${crate}" ;;
    2) echo "2/${crate}" ;;
    3) echo "3/${crate:0:1}/${crate}" ;;
    *) echo "${crate:0:2}/${crate:2:2}/${crate}" ;;
  esac
}

# 0: the version is on crates.io, 1: it is not, 2: the lookup failed. A failed lookup is never read
# as "not there", which would publish again.
index_status() {
  local crate="$1" version="$2" response body http_code
  response="$(curl -sSL --retry 3 -w $'\n%{http_code}' "https://index.crates.io/$(crate_index_path "$crate")")" ||
    return 2
  http_code="${response##*$'\n'}"
  body="${response%$'\n'*}"
  case "$http_code" in
    200) grep -qF "\"vers\":\"${version}\"" <<<"$body" && return 0 || return 1 ;;
    404) return 1 ;;
    *)
      echo "warning: the crates.io index answered HTTP ${http_code} for ${crate}" >&2
      return 2
      ;;
  esac
}

await_index() {
  local crate="$1" version="$2" waited=0
  while [ "$waited" -lt "$INDEX_TIMEOUT" ]; do
    if index_status "$crate" "$version"; then
      return 0
    fi
    sleep 10
    waited=$((waited + 10))
  done
  echo "warning: ${crate} ${version} is not in the crates.io index after ${INDEX_TIMEOUT}s; going on" >&2
}

echo "ferrigura ${version} from ${ref} (${commit:0:12}), in this order:"
for crate in $crates; do
  status=0
  index_status "$crate" "$version" || status=$?
  case "$status" in
    0) echo "  ${crate}: ${version} is on crates.io, skipped" ;;
    1) echo "  ${crate}: to publish" ;;
    *) die "the crates.io index lookup for ${crate} failed; not guessing whether ${version} is there" ;;
  esac
done

echo
echo "Packaging and building every crate from its package (cargo publish --dry-run):"
args=()
for crate in $crates; do
  args+=(-p "$crate")
done
cargo publish --dry-run --locked "${args[@]}"

if [ "$dry_run" -eq 1 ]; then
  echo
  echo "Dry run: nothing is published."
  exit 0
fi

echo
echo "Publishing cannot be undone: crates.io keeps every version (yanking only hides it)."
printf 'Type %s to publish ferrigura %s: ' "$version" "$version"
read -r answer || die "not confirmed; nothing is published"
[ "$answer" = "$version" ] || die "not confirmed; nothing is published"

publish() {
  local crate="$1" log="${work}/publish-${1}.log" retried=0 waits=0
  while :; do
    if cargo publish --locked -p "$crate" 2>&1 | tee "$log"; then
      return 0
    fi
    if index_status "$crate" "$version"; then
      echo "${crate} ${version} is on crates.io after all."
      return 0
    fi
    if grep -qE 'no token found|cargo login|401 Unauthorized|403 Forbidden' "$log"; then
      die "crates.io did not accept the login for ${crate}; run \`cargo login\` with a token with the scopes publish-new and publish-update, then this script again"
    fi
    if grep -qiE '429|too many' "$log" && [ "$waits" -lt "$RATE_LIMIT_TRIES" ]; then
      waits=$((waits + 1))
      echo "crates.io's limit on new crates: trying ${crate} again in ${RATE_LIMIT_WAIT}s."
      sleep "$RATE_LIMIT_WAIT"
      continue
    fi
    # The usual cause is that the crate before has not reached the index yet.
    if [ "$retried" -eq 0 ]; then
      retried=1
      echo "Trying ${crate} again in ${RETRY_DELAY}s."
      sleep "$RETRY_DELAY"
      continue
    fi
    die "could not publish ${crate} ${version}; the crates before it are on crates.io, and running this script again with ${ref} goes on from ${crate}"
  done
}

for crate in $crates; do
  status=0
  index_status "$crate" "$version" || status=$?
  [ "$status" -ne 2 ] || die "the crates.io index lookup for ${crate} failed; not guessing whether ${version} is there"
  if [ "$status" -eq 0 ]; then
    echo "Skipping ${crate} ${version}: it is on crates.io."
    continue
  fi
  publish "$crate"
  await_index "$crate" "$version"
done

echo
echo "ferrigura ${version} is on crates.io: https://crates.io/crates/ferrigura/${version}"
echo
echo "So that the workflow publishes the next release (docs-port/RELEASE.md):"
echo "1. Enable Trusted Publishing for each crate, under \"Trusted Publishing\" in its settings:"
echo "   GitHub, owner ${REPOSITORY%/*}, repository ${REPOSITORY#*/}, workflow publish.yml, no environment."
for crate in $crates; do
  echo "   https://crates.io/crates/${crate}/settings"
done
echo "2. gh variable set PUBLISH_ENABLED --body true --repo ${REPOSITORY}"
