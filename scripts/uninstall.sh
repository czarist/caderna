#!/usr/bin/env bash
# Removes the application as installed by install.sh: the app, its command, menu
# entry, icon and desktop shortcut. Study data (plans, progress, speech and
# voice models) is kept unless you say otherwise.
#
# macOS: "~/Applications/${data_dir}.app/Contents/Resources/uninstall.sh"
# Linux: ~/.local/opt/${data_dir}/uninstall.sh
set -euo pipefail

# Brand values come from the package or the invoking updater, never shell-evaluated.
data_dir="${KG_INSTALL_DATA_DIR:-}"
slug="${KG_INSTALL_SLUG:-}"
repo="${KG_INSTALL_REPOSITORY:-}"
app_name="${KG_INSTALL_NAME:-}"
config_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" 2>/dev/null && pwd || true)"
if [[ -n "$config_dir" && -f "$config_dir/brand.conf" ]]; then
  { IFS= read -r data_dir; IFS= read -r slug; IFS= read -r repo; IFS= read -r app_name; } < "$config_dir/brand.conf"
fi
[[ "$data_dir" =~ ^[A-Za-z0-9_-]+$ && "$slug" =~ ^[a-z0-9-]+$ ]] || { printf 'Set valid KG_INSTALL_DATA_DIR and KG_INSTALL_SLUG\n' >&2; exit 1; }
[[ -n "$app_name" ]] || app_name="$data_dir"
[[ "$app_name" != *$'\n'* && "$app_name" != *$'\r'* ]] || exit 1


# Yes or no from the terminal; no answer means no.
ask() {
  local answer=""
  if { exec 3</dev/tty; } 2>/dev/null; then
    read -r -p "$1 [y/N] " answer <&3 || true
    exec 3<&-
  fi
  [[ "$answer" =~ ^[yYsS] ]]
}

if [[ "$(uname -s)" == Darwin ]]; then
  launch_agent="$HOME/Library/LaunchAgents/org.study.${slug}.desktop.plist"
  if launchctl print "gui/$(id -u)/org.study.${slug}.desktop" >/dev/null 2>&1; then
    if ! launchctl bootout "gui/$(id -u)" "$launch_agent"; then
      printf 'Could not stop the the application login item; remove it from Login Items.\n' >&2
    fi
  fi
  rm -f -- "$launch_agent"
  rm -rf -- "$HOME/Applications/${data_dir}.app"
  if [[ -L "$HOME/Desktop/${data_dir}" ]]; then
    rm -f -- "$HOME/Desktop/${data_dir}"
  fi
  data="$HOME/Library/Application Support/${data_dir}"
else
  rm -rf -- "$HOME/.local/opt/${data_dir}"
  rm -f -- "$HOME/.local/bin/${slug}" \
    "$HOME/.local/share/applications/${slug}.desktop" \
    "$HOME/.config/autostart/${slug}.desktop" \
    "$HOME/.local/share/icons/hicolor/256x256/apps/${slug}.png"
  desktop="$(xdg-user-dir DESKTOP 2>/dev/null || printf '%s' "$HOME/Desktop")"
  rm -f -- "$desktop/${slug}.desktop"
  data="${XDG_DATA_HOME:-$HOME/.local/share}/${data_dir}"
fi
printf 'the application was removed.\n'

if [[ -d "$data" ]]; then
  if ask "Also delete your study data (plans, progress, voice and speech models) in $data?"; then
    rm -rf -- "$data"
    printf 'Study data deleted.\n'
  else
    printf 'Your study data stays in %s.\n' "$data"
  fi
fi
