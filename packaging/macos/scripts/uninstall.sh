#!/bin/bash
# Removes only files that belong to com.pgm-data.cli.
# Application Support data is kept unless --delete-data is passed.
set -euo pipefail
ID="com.pgm-data.cli"
DELETE_DATA=0
ROOT="${PGM_UNINSTALL_ROOT:-/}"
for arg in "$@"; do
  if [[ "$arg" == "--delete-data" ]]; then
    DELETE_DATA=1
  fi
done

remove_path() {
  local path="$1"
  if [[ "$path" != /usr/local/pgm-data/* && "$path" != /usr/local/bin/pgm-data ]]; then
    echo "refusing path outside the package tree: $path" >&2
    exit 1
  fi
  local full="${ROOT%/}$path"
  if [[ -f "$full" || -L "$full" ]]; then
    rm -f "$full"
  fi
}

if [[ -n "${PGM_UNINSTALL_MANIFEST:-}" ]]; then
  while IFS= read -r path; do
    [[ -z "$path" ]] && continue
    remove_path "$path"
  done < "$PGM_UNINSTALL_MANIFEST"
elif [[ "$ROOT" == "/" ]] && command -v pkgutil >/dev/null; then
  pkgutil --only-files --files "$ID" | while IFS= read -r rel; do
    remove_path "/$rel"
  done
  pkgutil --forget "$ID" || true
else
  echo "set PGM_UNINSTALL_MANIFEST or run as root against the installed receipt" >&2
  exit 1
fi

if [[ "$DELETE_DATA" == "1" ]]; then
  data="${HOME}/Library/Application Support/pgm-data"
  rm -rf "$data"
fi
exit 0
