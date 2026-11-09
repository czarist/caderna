#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
export KG_BRAND_DIR="${1:-${KG_BRAND_DIR:-branding/default}}"
platform="$(uname -s)"
architecture="$(uname -m)"
case "$platform" in Linux) package_os=linux ;; Darwin) package_os=macos ;; *) exit 1 ;; esac
target_dir="${CARGO_TARGET_DIR:-$repo_root/target}"
[[ "$target_dir" == /* ]] || target_dir="$repo_root/$target_dir"
cargo build -p kg-brand --example metadata
metadata="$target_dir/debug/examples/metadata"
app_name="$("$metadata" name)"
data_dir="$("$metadata" data_dir)"
slug="$("$metadata" slug)"
executable="$("$metadata" executable)"
repository="$("$metadata" repository)"
icon="$("$metadata" icon_png)"
if [[ "${KG_COMMERCIAL:-0}" == 1 ]]; then
  "$metadata" release-ready
  if [[ "$platform" == Darwin ]]; then
    : "${KG_SIGN_IDENTITY:?Set the Apple Developer ID identity}"
    : "${KG_NOTARY_PROFILE:?Set a notarytool keychain profile}"
  fi
fi
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
bash scripts/build-llama-unix.sh
cargo build --locked --release -p know-giver
mkdir -p dist
stage="$(mktemp -d "$repo_root/dist/.package.XXXXXX")"
trap 'rm -rf -- "$stage"' EXIT
brand_config() { printf '%s\n' "$data_dir" "$slug" "${repository#https://github.com/}" "$app_name" "$executable" > "$1/brand.conf"; }
xml_escape() { printf '%s' "$1" | sed -e 's/\&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g'; }
if [[ "$platform" == Darwin ]]; then
  app="$stage/$data_dir.app"
  bundle="$app/Contents"
  mkdir -p "$bundle/MacOS" "$bundle/Resources"
  cp "$target_dir/release/know-giver" "$bundle/MacOS/$executable"
  cp .env.example "$bundle/MacOS/"
  cp -R "$target_dir/release/local-runtime" "$bundle/MacOS/"
  cp scripts/uninstall.sh "$bundle/Resources/"
  brand_config "$bundle/Resources"
  mkdir -p "$stage/app.iconset"
  for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$icon" --out "$stage/app.iconset/icon_${size}x${size}.png" >/dev/null
  done
  iconutil -c icns "$stage/app.iconset" -o "$bundle/Resources/app.icns"
  rm -rf -- "$stage/app.iconset"
  cat > "$bundle/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>org.study.$slug</string>
<key>CFBundleName</key><string>$(xml_escape "$app_name")</string>
<key>CFBundleExecutable</key><string>$executable</string>
<key>CFBundleIconFile</key><string>app.icns</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>CFBundleVersion</key><string>$version</string>
<key>NSMicrophoneUsageDescription</key><string>Voice lessons use the microphone.</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
EOF
  plutil -lint "$bundle/Info.plist"
  if [[ -n "${KG_SIGN_IDENTITY:-}" ]]; then
    while IFS= read -r -d '' item; do
      if file -b "$item" | grep -q 'Mach-O'; then
        codesign --force --options runtime --timestamp --sign "$KG_SIGN_IDENTITY" "$item"
      fi
    done < <(find "$bundle/MacOS" -type f -print0)
    codesign --force --options runtime --timestamp --sign "$KG_SIGN_IDENTITY" "$app"
    codesign --verify --deep --strict "$app"
    if [[ -n "${KG_NOTARY_PROFILE:-}" ]]; then
      ditto -c -k --keepParent "$app" "$stage/notarize.zip"
      notary_args=(--keychain-profile "$KG_NOTARY_PROFILE" --wait)
      if [[ -n "${KG_SIGN_KEYCHAIN:-}" ]]; then notary_args+=(--keychain "$KG_SIGN_KEYCHAIN"); fi
      xcrun notarytool submit "$stage/notarize.zip" "${notary_args[@]}"
      xcrun stapler staple "$app"
      rm -f -- "$stage/notarize.zip"
    fi
  fi
else
  dest="$stage/$data_dir"
  mkdir -p "$dest"
  cp "$target_dir/release/know-giver" "$dest/$executable"
  cp .env.example scripts/uninstall.sh "$dest/"
  cp packaging/unix/README.*.txt "$dest/"
  cp -R "$target_dir/release/local-runtime" "$dest/"
  cp "$icon" "$dest/$slug.png"
  brand_config "$dest"
  cat > "$dest/$slug.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=$app_name
Exec=$executable
Icon=$slug
Categories=Education;
Terminal=false
EOF
fi
cp packaging/unix/README.*.txt "$stage/"
archive="$repo_root/dist/$data_dir-$version-$package_os-$architecture.tar.gz"
tar -czf "$archive" -C "$stage" .
printf 'Package: %s\n' "$archive"
