# Brands

Each folder here is one brand: a `brand.toml` and, ideally, its own pictures. A build
carries exactly one brand, chosen at compile time:

| Folder | What it is |
| --- | --- |
| `default/` | Know Giver itself. Used when `KG_BRAND_DIR` is not set. |
| `example/` | Template for a client, with a 14-day demonstration. Builds as is. |
| `caderna/` | Sample client brand (paper, graphite, olive) with its own art and draft legal texts. |

Real client brands (contact data, keys) belong in a private repository checked out
under `branding/private/<client>`, which git ignores.

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

3. Fill in `[app]` (name, `slug`, `data_dir`, `executable` for the program's file name
   such as `Caderna` → `Caderna.exe` (default `know-giver`), a new `installer_guid` from
   `[guid]::NewGuid()`), `[company]`, `[colors]`, `[updates]`, `[demo]`, `[legal]` and `[licensing]`,
   plus `[server]` and `[locale]` when the client has a panel or sells in fewer countries.
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

## Support, art and legal texts

- `[company] support_url` (`https://` or `mailto:`): the support link in Settings → About.
- `[company] teachers_dir` / `stickers_dir`: optional folders with the same file names as
  the bundled teacher portraits and stickers; any file found there replaces the default.
- `[legal] directory`: a folder with the documents of `docs/legal/` the brand can show:
  `terms-of-use` and `privacy-policy` for each language in `[locale] languages`, and
  `partner-access` only when `[locale] regions` includes `cn` or `other`. It needs
  `version` (becomes the consent version: change it and every student agrees again),
  `provider` (the client, who serves the students), `licensor` and `contact`. Keep the
  rules of the root `CLAUDE.md`: every language the brand offers, and the texts still say
  the AI can be wrong.
- `[legal] reviewed` and `commercial_reviewed` stay `false` until a lawyer reviewed the
  client's texts and the third-party licences. Packaging for sale refuses to run while
  either is `false`; a passing build is not a review.

## Licences

`[licensing] public_key` holds the base64 Ed25519 **public** key; the private key never
goes into a brand, the repository or CI artifacts. With a key set, Settings and the
"demonstration ended" screen accept a licence pasted by the student. A licence names the
brand (`slug`), the customer, the seats and its first and last day; a wrong brand, a
changed byte or a date outside the period is refused, and a refused licence never
replaces an accepted one or touches study data. `required = true` makes the build stop
without a valid licence, like an ended demo.

```sh
# once per brand, on the licensor's machine (keep private.pk8 offline and backed up)
cargo run -p kg-engine --example license_issue -- keygen private.pk8 public.txt
# claims.json: {"version":1,"id":"acme-2027-01","brand":"acme","customer":"Acme",
#               "seats":50,"not_before":"2027-01-01","expires":"2027-12-31"}
cargo run -p kg-engine --example license_issue -- sign private.pk8 claims.json license.txt
```

Without a panel, seats are a contractual limit: the check is offline, so the app cannot
count computers.

## Panel (`[server]`)

`[server]` points the brand at its panel ([CADERNA-DASHBOARD](https://github.com/czarist/caderna-dashboard)).
The address is chosen when the app is built, never read on the student's computer, so
nobody can point an installed app at another server:

- `url`: release builds (the installer). Must be `https://`; a release build of a brand
  with a panel stops without it.
- `dev_url`: `cargo run` and tests, e.g. `http://localhost:8000`.
- `KG_SERVER_URL` overrides both, from the shell or CI; in development builds it can also
  come from the repository's `.env` (see `.env.example`).

With a panel:

- **Licences are activated online.** The key is still checked offline, then the panel
  reserves a seat for this computer and returns a device token valid for 7 days, kept in
  the operating system's credential store (memory only when there is none). The app
  checks in when it opens and every hour, renewing the token before it expires
  (`renew: true`, so a computer signed out in the panel stays out until the student
  activates it again). Without the panel the app keeps working for 14 days after the last
  contact (`OFFLINE_GRACE_DAYS`). The panel's refusals (revoked, expired, no seats, signed
  out, blocked) lock the app with the reason; study data stays. Settings has "Sign out
  this computer" to free the seat.
- **The computer's id** sent to the panel is the operating system's machine id hashed with
  the brand's `slug`; the computer's name is never sent.
- **Modules per licence.** The panel's licence says which paid areas it opens:
  *Videoaula* (voice lessons) and *Criação de conteúdos* (handouts, quizzes, reviews,
  mock exams, written answers, exam board). The sidebar and the tour leave the others
  out, a closed screen says so with the contact link, and the engine refuses their
  commands (`EngineError::ModuleDisabled`). Changes in the panel arrive at the next check.
- **AI goes through the panel** with the device token. `text_model` and `vision_model`
  are the panel's model aliases; the AI choice disappears from the setup and Settings.
- **The brand is downloaded** from the panel (ETag cache): app and company names, accent
  and highlight colours, support and contact links. The app checks with the panel when it
  opens and every hour; a changed brand applies at once (names, colours, pictures, window
  title and icon, handouts made from then on) and is kept for the next start. Pictures (icon, cover, reminder art, mark, handout
  watermark) are downloaded too and kept only when safe: square PNGs of 64–2048 px up to
  2 MB, SVGs up to 512 KB without scripts or outside references, from `https://` or the
  panel's own address. Texts replace the app's by locale key, in the brand's first
  language only. Everything lives in `<data dir>/panel-brand/`, which the reminder pop-up
  also reads; the handout uses the panel's company name, colour and watermark.
- `[licensing] public_key` is required; set `required = true` and `[demo] enabled = false`
  so evaluations are licences issued by the panel.
- The privacy policy must name the panel and what it receives; see
  `branding/caderna/legal/privacy-policy.pt-BR.md`.

## Languages and regions (`[locale]`)

`languages` (from `pt-BR`, `en`, `ja`, `zh-Hans`, `zh-Hant`) and `regions` (from `br`,
`us`, `eu`, `uk`, `jp`, `tw`, `hk`, `sg`, `cn`, `other`) limit what the language and
region pickers offer; empty offers everything. A brand with one of each skips both
choices in the setup. Only the legal texts of the offered languages are needed. Caderna
offers only `pt-BR` and `br`.

## Still open

- Lawyer review of each client's legal texts, and the licence agreement with the client.
- Signing certificates under the client's publisher name.
- Licences: check every item under the README's Credits before selling a build (the Dii
  voice, non-commercial, is no longer offered).
