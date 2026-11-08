#!/usr/bin/env bash
set -euo pipefail
: "${RUNNER_TEMP:?CI runner required}"
: "${KG_CERT_P12_B64:?Certificate secret required}"
: "${KG_CERT_PASSWORD:?Certificate password required}"
: "${KG_APPLE_ID:?Apple ID required}"
: "${KG_APPLE_TEAM:?Apple team required}"
: "${KG_APPLE_PASSWORD:?App-specific password required}"
keychain="$RUNNER_TEMP/client-signing.keychain-db"
cert="$RUNNER_TEMP/client-signing.p12"
umask 077
printf '%s' "$KG_CERT_P12_B64" | base64 --decode > "$cert"
security create-keychain -p "$KG_CERT_PASSWORD" "$keychain"
security set-keychain-settings -lut 21600 "$keychain"
security unlock-keychain -p "$KG_CERT_PASSWORD" "$keychain"
security import "$cert" -P "$KG_CERT_PASSWORD" -A -t cert -f pkcs12 -k "$keychain" >/dev/null
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$KG_CERT_PASSWORD" "$keychain" >/dev/null
security list-keychains -d user -s "$keychain"
xcrun notarytool store-credentials client-notary --apple-id "$KG_APPLE_ID" --team-id "$KG_APPLE_TEAM" --password "$KG_APPLE_PASSWORD" --keychain "$keychain" >/dev/null
rm -f -- "$cert"
printf '%s\n' 'KG_NOTARY_PROFILE=client-notary' "KG_SIGN_KEYCHAIN=$keychain" >> "$GITHUB_ENV"
