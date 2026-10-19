#!/bin/zsh
set -eu
pgm_root="${0:A:h:h:h}"
pgm_font="$pgm_root/assets/fonts/PxPlus_IBM_VGA_8x16.ttf"
pgm_installed="$HOME/Library/Fonts/PxPlus_IBM_VGA_8x16.ttf"
mkdir -p "$HOME/Library/Fonts"
if [[ -e "$pgm_installed" ]]; then
  if ! cmp -s "$pgm_font" "$pgm_installed"; then
    print -u2 "The installed VGA font differs from the bundled font."
    exit 1
  fi
else
  cp "$pgm_font" "$pgm_installed"
fi
cd "$pgm_root"
cargo build --offline --bin pgm-data
pgm_profile="${PGM_DATA_HOME:-$HOME/Library/Application Support/pgm-data}/ghostty-ansi.config"
if [[ ! -f "$pgm_profile" ]]; then
  pgm_profile="$pgm_root/packaging/ghostty/pgm-ansi.ghostty"
fi
pgm_args=(--config-default-files=false "--config-file=$pgm_profile" "--working-directory=$pgm_root")
if [[ -n "${PGM_DATA_HOME:-}" ]]; then
  pgm_args+=("--env=PGM_DATA_HOME=$PGM_DATA_HOME")
fi
open -na Ghostty --args "${pgm_args[@]}" -e "$pgm_root/target/debug/pgm-data" tui
