# Brands

Each folder here is one brand: a `brand.toml` and, ideally, its own pictures. A build
carries exactly one brand, chosen at compile time:

| Folder | What it is |
| --- | --- |
| `default/` | Know Giver itself. Used when `KG_BRAND_DIR` is not set. |
| `example/` | Template for a client, with a 14-day demonstration. Builds as is. |

## New client

1. Copy `example/` to `branding/<client>/`.
2. Put the client's pictures in that folder and point `[assets]` at them:

   | Key | File |
   | --- | --- |
   | `icon_png` | app icon, 512 × 512 PNG, transparent corners (window, taskbar, sidebar) |
   | `icon_ico` | Windows icon (exe and installer); with `KG_BRAND_DIR` set, `cargo run -p kg-ui --example write_icon -- <out.ico>` makes one from the brand's `icon_png` |
   | `cover_png` | square cover art, corners filled (welcome and empty screens) |
   | `engraving_png` | art of the study reminder pop-up |
   | `mark_svg` | company mark, **white**, one colour (the app tints it) |
   | `watermark_svg` | company mark already **faint and coloured**, for the handout watermark |

3. Fill in `[app]` (name, `slug`, `data_dir`, a new `installer_guid` from
   `[guid]::NewGuid()`), `[company]`, `[colors]`, `[updates]` and `[demo]`.
   `colors.company` must differ from the handout's own colours; the build says so if not.
4. Build: `scripts\package.ps1 -Brand branding\<client>`, or for a quick look
   `$env:KG_BRAND_DIR = "branding/<client>"; cargo run --release`.

A wrong value stops the build with a message naming the key (`crates/brand/config.rs`
checks every field).

## What the brand changes

- Window title, sidebar, tray, every text that names the app (`%{app}` in `locales/`),
  "Made by …" credits (`%{company}`), the ChatGPT sign-in name.
- Accent and highlight colours (light and dark), the icon, cover and reminder art.
- Handout watermark and footer, lesson whiteboard credit.
- Data folder (`%LOCALAPPDATA%\<data_dir>`), database and log names, the Windows exe's
  icon and version info, the installer's name, folder, GUID and file names.
- Updates: only from `[updates] repository`; empty means no update check and no
  source-code link.

## Demonstration builds

`[demo] enabled = true` with `days` (counted from the first start, that day included)
and/or `expires` (last day, `YYYY-MM-DD`): whichever comes first ends the demo. The
sidebar shows the days left; afterwards the app opens only the "demonstration ended"
screen, with a button to `contact_url`. Study data stays on the computer, so the full
build (same `slug` and `data_dir`, `enabled = false`) picks it up.

The first start and the latest day used are kept in the database and in
`%APPDATA%\<data_dir>\demo.json`; the earliest start and latest day win, so deleting one
or turning the clock back does not restart the demo. Someone who deletes both, or edits
the binary, can get around it: it is a commercial limit, not copy protection.

## Not branded yet

- `docs/legal/` (terms and privacy policy) still name Know Giver and Trentin Software.
  A client build needs texts reviewed for that client, with `TERMS_VERSION` bumped.
- Teacher portraits, achievement badges and other art in `crates/ui/assets`.
- macOS and Linux packages (`scripts/package-unix.sh`, `scripts/install.sh`).
- Licences: the Dii voice is CC BY-NC-ND (non-commercial). Check every item under the
  README's Credits before selling a build.
