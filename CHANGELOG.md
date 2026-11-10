# Changelog

All notable changes to the Know Giver white-label base. Format based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/).

Earlier history (Know Giver 0.1.0 to 0.39.3) lives in the upstream Know Giver
repository.

## [Unreleased]

### Fixed
- **Voice downloads on Linux and macOS.** Piper could only be downloaded on Windows;
  elsewhere the button failed with "This is not supported on this system". The app now
  downloads the official builds for each system, checked against pinned SHA-256 sums:
  - Piper on Windows and Linux (x86_64 and aarch64);
  - sherpa-onnx v1.12.20, which runs the Kokoro voices and Parakeet, on Windows,
    Linux (x86_64 and aarch64) and macOS (Apple silicon and Intel).

  Piper's macOS archives are broken upstream, so on macOS the default voice is Kokoro,
  a saved Piper voice switches to the Kokoro voice of the same gender, and Settings
  shows "Not available on this system" instead of a button that fails. The welcome
  screen downloads the program of the chosen voice, and the lesson hint no longer
  names Piper.

## [0.2.0] - 2026-11-10

Classes in the cloud, for brands with a panel.

### Added
- Teachers and students: the panel's client area enrols each person as a teacher or a
  student. Students use what their classes give them (lessons, quizzes, handouts,
  readings) and cannot create or change subjects; teachers create plans and content.
- Classes screen for teachers: publish a plan, with its handouts and quiz questions, to
  classrooms, and see each student's topics mastered, quiz accuracy, lessons, study time
  and last activity.
- Students' progress kept in the cloud and merged across computers, so it comes back on a
  new computer; "Sync now" on the Syllabus screen.
- Shared base of AI-made material: the app looks for an equal plan, handout or quiz bank
  made by any school before asking the AI, and shares what the AI makes, to save tokens.
- The purpose of each plan (ENEM, university entrance exam, public-service exam, bar or
  certification exam, other), asked when the plan is created; prompts adapt to it.
- Bibliographic references at the end of every handout topic.
- Panel: classrooms, courses and roles in the client area; the shared base for admins;
  `/api/v1/classrooms`, `/courses`, `/progress` and `/shared-materials`.
- Signed offline licences (Ed25519, public key in the brand): activation in Settings and
  on the "demonstration ended" screen, bound to brand, customer, seats and dates;
  `license_issue` example to create keys and licences. `[licensing] required` builds stop
  without a valid licence.
- Brand options: support link, per-brand legal texts and consent version, teacher and
  sticker art folders, review flags that gate commercial packaging.
- Handouts from a demo or unlicensed build carry a "Demonstration" stamp.
- Caderna sample brand.
- CADERNA-DASHBOARD panel: a client area where schools enrol their students and set their
  plans, and charges the client pays there by Pix or credit card (up to 3 instalments
  without interest, through Asaas).
- Per-brand macOS/Linux packages, installers and release workflow; signing scripts that
  read certificates from CI secrets.

### Changed
- Caderna terms of use and privacy policy describe classes, the cloud progress and the
  shared base (version `2026-11-10.1`; users agree again).
- Handout prompts speak of any Brazilian exam, not only public-service exams.
- The Dii voice (non-commercial licence) is no longer offered or downloaded.
- READMEs in all five languages show the Caderna icon and screenshots rendered with the
  Caderna brand, the full brand.toml sections, licence issuing and the CADERNA-DASHBOARD
  panel (its integration in the app is ROADMAP Phase W6). Builds use every core again.

## [0.1.0] - 2026-11-08

First version of the white-label base, forked from Know Giver 0.39.3 with all its
features.

### Added
- **Brands.** A build carries one brand, read at compile time from
  `branding/<name>/brand.toml` (`KG_BRAND_DIR`; default `branding/default`, Know Giver
  itself) by the new `kg-brand` crate. The brand sets:
  - the product name in the window, tray, sidebar and every text (`%{app}` in the
    locale files), and the company in the credits (`%{company}`);
  - accent and highlight colours for the light and dark themes;
  - app icon, cover art, reminder art and company mark; the "Made by …" credit can be
    turned off;
  - the handout watermark, footer credit and its colour, and the whiteboard credit;
  - the data folder, database and log names, so brands never share data;
  - the Windows exe icon and version info, and the installer's name, publisher,
    folder, GUID, icon and file names (`scripts\package.ps1 -Brand branding\<client>`);
  - where updates come from; without a repository the app never checks and shows no
    source-code link.
  A wrong value stops the build with a message naming the key.
- **Demonstration builds.** `[demo]` in `brand.toml` limits use to some days from the
  first start, to a last date, or both. The sidebar shows the days left; afterwards the
  app opens only a "demonstration ended" screen with a button to the client's contact
  link, in all five languages. Plans and progress stay on the computer. The first start
  and the latest day used are kept in the database and in the system settings folder,
  so deleting one or turning the clock back does not restart the demonstration.
- `branding/example`: a client template with a 14-day demonstration, and
  `branding/README.md` with the steps for a new client.

### Changed
- The workspace version restarts at 0.1.0.
- Update checks are off in the default brand (no public releases from this
  repository); README install instructions now describe branded builds.
- Release package and installer names follow the brand's `data_dir`
  (`KnowGiver-…` for the default brand).
