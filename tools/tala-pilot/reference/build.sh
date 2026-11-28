#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
  printf '%s\n' "usage: $0 D2_CHECKOUT OUTPUT_BINARY" >&2
  exit 2
fi

here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
upstream=$(CDPATH='' cd -- "$1" && pwd)
case "$2" in
  /*) output=$2 ;;
  *) output=$PWD/$2 ;;
esac

pin=b463ea5d47272f4da6e1e08012dbb15a6a4dbcde
if [ "$(git -C "$upstream" rev-parse HEAD)" != "$pin" ] ||
   [ -n "$(git -C "$upstream" status --porcelain --untracked-files=all)" ]; then
  printf '%s\n' "D2 checkout must be clean at $pin (v0.10.0)" >&2
  exit 1
fi

cd -- "$upstream"
GOWORK=off GOTOOLCHAIN=local go build -mod=readonly -trimpath -o "$output" "$here/main.go"
