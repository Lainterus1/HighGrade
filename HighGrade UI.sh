#!/bin/sh
# HighGrade project UI launcher, protocol 1. Resolve the active release at launch.
set -eu
fail() { printf '%s\n' "HighGrade UI: $*" >&2; exit 1; }
case "${HOME-}" in /*) ;; *) fail 'HOME must name an absolute user profile.';; esac
case "${1-}" in '') no_open=false;; --no-open) no_open=true;; *) fail 'Usage: HighGrade UI.sh [--no-open]';; esac
[ "$#" -le 1 ] || fail 'Too many arguments.'
project=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
base="$HOME/.highgrade/global"
pointer="$base/active.json"
[ -f "$pointer" ] || fail 'High Grade is not installed for this user.'
release=$(sed -n 's/.*"release"[[:space:]]*:[[:space:]]*"\([a-z0-9-]*\)".*/\1/p' "$pointer")
case "$release" in ''|*[!a-z0-9-]*) fail 'Invalid active release.';; esac
[ "${#release}" -le 64 ] || fail 'Invalid active release.'
exe="$base/releases/$release/highgrade"
[ -x "$exe" ] || fail 'Active High Grade executable is missing or not executable.'
"$exe" global-status --profile "$HOME" >/dev/null || fail 'Installation needs repair; run global-status.'
exec "$exe" ui --root "$project" --no-open "$no_open"
