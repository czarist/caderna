# Changelog

All notable changes to the Know Giver white-label base. Format based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/).

Earlier history (Know Giver 0.1.0 to 0.39.3) lives in the upstream Know Giver
repository.

## [Unreleased]

### Added
- **Snapshot options for the website.** The screen snapshots take `KG_SNAPSHOT_SIZE`
  (for example `1920x1080`), `KG_SNAPSHOT_LOCALE` (one language) and
  `KG_SNAPSHOT_BOARD_SCALE` (pixel density of the whiteboard); without them nothing
  changes. A new `quiz_snapshots` renders a quiz question before and after the answer,
  and the written-answer snapshots no longer show sticker notices.
- **Build channels.** Brands with a panel build for production, homologation or
  development (`KG_CHANNEL`, from the branch in CI: `main`, `hom`, `dev`), each talking to
  its own panel (`[server] url`, `hom_url`, `dev_url`). Homologation and development
  install as programs of their own (*Caderna HOM*, *Caderna DEV*: own data folder,
  executable, installer GUID and credential entry), show *HOMOLOGAÇÃO* or
  *DESENVOLVIMENTO* in a strip at the top and in the window title, and never update
  themselves. `.github/workflows/channels.yml` builds each branch for Windows, Linux and
  macOS and replaces its download (releases `latest-prod`, `latest-hom`, `latest-dev`);
  production is published only after the brand's legal review. Caderna points at
  `https://caderna.ia.br`, `https://hom.caderna.ia.br` and `http://localhost:8080`.
- **School restrictions for students.** In the panel's client area (*Permissões dos
  alunos*), a school can stop its students from taking handouts out of the app
  (exporting them or opening the PDF in another program) and from taking screenshots or
  recording the screen. The engine refuses blocked exports, the reader disables those
  buttons and says why, and Settings lists what the school blocks. On Windows the
  window shows black in captures (`SetWindowDisplayAffinity`); macOS and Linux cannot
  block by themselves, so there the content is covered while the window is out of focus
  or a known screen recorder or screenshot tool is running (`ps` on macOS, `/proc` on
  Linux, read locally), and macOS also marks the window as not shared. With capture
  blocked, handouts carry the student's name and email as a faint diagonal watermark
  (sent by the panel only then, never saved with the handout, not selectable in the
  reader). Teachers are never restricted.
- **Picture credits and sources in handouts.** Each Wikipedia picture's author and
  license come from Wikimedia and go in its caption, and the topic's bibliography lists
  the Wikipedia article and the picture file (ABNT in Portuguese; all five languages).
  Handouts saved earlier keep their pages, so their highlights stay in place.
- **Short access codes.** Students can activate with the panel's code
  (`XXXX-XXXX-XXXX-XXXX`, typed with or without dashes) instead of the long signed key;
  the panel answers with the signed key, which the app keeps and checks offline. The
  email sends a link to the code, with a copy button, instead of the code itself.
- **Overdue payments pause the app.** When a school's payment is 2 days late, the panel
  refuses its computers with `billing_paused`: the app shows a neutral "access paused,
  contact support" screen with the support link, keeps the study data, asks again every
  10 minutes and opens by itself once the payment arrives.
- Development test accounts for both projects (`DevTestSeeder` in the panel) and a
  pt-BR MVP test plan (`docs/mvp-test-plan.md`).
- Panel (CADERNA-DASHBOARD):
  - charges with a title and a description; the client picks card instalments in its
    area: up to 3 without interest, up to 10 with 1.99% a month;
  - subscriptions: a recurring fee issued 10 days before each due date, a due day the
    client chooses, automatic card payment (Asaas card token) with the client's consent,
    Pix reminders 3 days before and on the due date, and a pause 2 days after it;
  - LLM overage: past the monthly allowance the apps keep working up to an optional cap,
    the school is emailed once a month, and last month's extra tokens become one charge
    (price per million tokens) on the 1st;
  - a `scheduler` service in Docker for the daily billing run.

### Changed
- Channel builds (`.github/workflows/channels.yml`) run only by hand
  (`gh workflow run channels.yml --ref <branch>`), no longer on every push, to save
  Actions minutes; releases still build from a `v*` tag.
- The local panel runs on port 8080 (`APP_PORT`), as the development channel expects.
- Caderna terms of use and privacy policy describe the school's restrictions, the
  watermark, the local check of open programs and the Wikimedia credits (version
  `2026-11-11.1`; users agree again).
- The brand downloaded from the panel applies as soon as it arrives (theme, title, icon,
  pictures and handout brand), without restarting the app.

### Fixed
- CI frees disk space on the runners before building.

## [0.3.0] - 2026-11-10

### Added
- **Cebraspe-style quizzes (Certo/Errado).** Settings → *Quiz question style* offers
  *Like the exam* (default), *Multiple choice* and *True/false (Cebraspe)*. True/false
  quizzes ask statements to judge, written by the model with a clear content error in
  the wrong ones, and keep Certo before Errado. The result also shows Cebraspe's net
  score, where each wrong answer cancels a right one. *Like the exam* asks Certo/Errado
  when the exam format read from the notice is true/false, when the plan names
  Cebraspe/CESPE, or when most of the plan's past exam questions are Certo/Errado.
  Past exam questions follow the chosen style; the multiple-choice question bank
  prepared in advance is skipped for true/false quizzes.

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
- Panel integration (`[server]` in the brand): online licence activation with a device
  token in the system's credential store, renewed silently, a check at start and every
  hour, 14 days offline, refusals that lock the app with their reason, and "Sign out
  this computer"; the client's brand (names, colours, links, pictures, texts) downloaded
  and kept; AI through the panel, with the AI choice hidden in setup and Settings.
- Panel address per build: `[server] dev_url` (localhost) for development, `[server] url`
  or `KG_SERVER_URL` for releases.
- Licence modules from the panel (*Videoaula*, *Criação de conteúdos*): closed modules
  leave the sidebar and the tour, and the engine refuses them.
- General exam terms (*Programa da prova*, *Cargo ou área*, *Provas anteriores*) instead
  of terms used only by public-service exams.
- `scripts/package.ps1` takes the Inno Setup compiler from `KG_ISCC`.

### Changed
- Caderna terms of use and privacy policy describe classes, the cloud progress and the
  shared base (version `2026-11-10.1`; users agree again).
- Handout prompts speak of any Brazilian exam, not only public-service exams.
- The Dii voice (non-commercial licence) is no longer offered or downloaded.
- READMEs in all five languages show the Caderna icon and screenshots rendered with the
  Caderna brand, the full brand.toml sections, licence issuing and the CADERNA-DASHBOARD
  panel (its integration in the app is ROADMAP Phase W6). Builds use every core again.
- The program file follows the brand (`Caderna.exe`).

### Fixed
- A licence works from the day before its start date, so a key issued at night in Brazil
  is not refused on a computer still on the previous day.
- A brand picture that cannot be used no longer holds back the rest of the brand.

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
