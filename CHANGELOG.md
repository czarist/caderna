# Changelog

All notable changes to the Know Giver white-label base. Format based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/).

Earlier history (Know Giver 0.1.0 to 0.39.3) lives in the upstream Know Giver
repository.

## [Unreleased]

## [0.5.1] - 2026-11-24

### Added
- Speech recognition: **Nemotron 3.5** (NVIDIA Nemotron 3.5 ASR Streaming 0.6B,
  GGUF q8_0) in Settings → Voice → Speech recognition, next to Parakeet and Whisper.
  It is the default from 8 GB of memory (the minimum); below that, Parakeet stays the
  default. Recommended: 16 GB of memory and a dedicated graphics card with 4 GB.
  Settings shows which one fits the computer, and a recognizer picked there is kept.
  - The app reads the installed memory and the graphics cards from the system
    (registry on Windows, `/sys` on Linux; Macs count as having no dedicated card).
    A dedicated card needs 2 GB of its own memory: NVIDIA cards, AMD cards not named
    "… Graphics", and Intel Arc A and B cards.
  - Dedicated cards pick the build directly: CUDA on NVIDIA, Vulkan on AMD and Intel,
    with the CPU build as fallback if it fails to start. Without one, the builds are
    still timed as below.
  - README: system requirements (8 GB minimum, 16 GB and a 4 GB card recommended) and
    up-to-date badges.
  - It runs on the official NeMo-Speech.cpp server (`nemo-speech serve`, only on
    127.0.0.1) and is told the language (pt-BR) instead of guessing it, so it gets no
    Whisper second opinion. Live captions work with it too.
  - First results (17 phrases, four synthetic voices and a noisy set, two machines):
    about half of Parakeet's word errors, and "Sim / Não / Isso" no longer come out in
    English.
  - CPU or GPU, per computer: CUDA for NVIDIA GPUs (Windows and Linux x86_64; about
    500 MB, downloaded only with an NVIDIA driver installed and counted in the size
    shown before downloading), Vulkan for AMD and Intel GPUs (Windows, Linux), Metal on
    Apple silicon, and the CPU build everywhere (Intel Macs have no GPU build). A GPU
    build is used only when `nemo-speech doctor` reports a real GPU with room for the
    model and a compatible driver; one that crashed is skipped afterwards.
  - Without a dedicated card, right after installing (or on first use, for earlier
    installs) the app times each usable build on four seconds of audio and uses the
    fastest from then on; a GPU build (Metal on Apple silicon) must beat the CPU build
    by 20%.
  - Known gap: the v0.3.0 Vulkan build drops some one-word answers ("Sim" comes back
    empty) and adds no punctuation; AMD and Intel cards use it anyway.
  - The server's own warmup is off (it aborts the v0.3.0 Vulkan build); the app warms
    it up with its first request.
  - The runtime (v0.3.0) and the model are pinned with checksums in `local-ai.toml` and
    downloaded from the model list.
  - `cargo run --release -p kg-audio --example nemotron_check -- <models-dir> <wavs or
    folders>` compares Nemotron and Parakeet on the same recordings (word error rate
    against a `.txt` next to each WAV, time, peak memory, fastest build);
    `docs/nemotron-evaluation.md` has the phrases, the procedure and the results.

- **Sign in with email and password.** With a brand's panel, students and teachers sign in
  with the email their school enrolled and a password they create from the emailed
  invitation link; *Forgot my password* emails a new link. After two wrong attempts the
  panel asks for the code in a picture it draws itself (a captcha). The password goes only
  to the panel and is never kept on the computer. Typing a licence key or access code is
  now only for developer mode and development builds; computers already activated with a
  key keep working until they are signed out. The Caderna privacy policy and terms list the
  sign-in data (legal version `2026-11-23.1`, so everyone agrees again).
- Settings → *Developer mode* (with a valid developer token): *Open the student's view*
  and *Open the teacher's view*, and back to the licence's. In the teacher's view the
  sidebar says *Check lessons* and *Check quizzes*.
- **Developer area: views and presentation mode (Phase W9).** With a valid developer
  token, the area forces the student or teacher view (every module open, whatever the
  licence) and lists the panel's clients with a brand; *Present* shows the app with
  that client's name, colours and pictures and hides the development strip, until
  *Stop presenting*. Nothing is saved; the next start is as before. The end-to-end
  test covers it (`KG_E2E_DEV_CODE`).
- **"Sou aluno / Sou professor" in the first run.** The name step also asks how the
  person uses the app. A role the school set on the licence in the panel shows
  chosen and locked; otherwise the choice sets the view (`Engine::view_role`): the
  student view has no lesson planning or classrooms. Settings saved before ask it
  once. Stored only on the computer.
- End-to-end UI test (`crates/ui/src/e2e.rs`, ignored by default): activation with
  a test licence on the local panel, the first run by clicking, a handout, a lesson
  with the panel's AI and the chat, then sign-out.
- **Lesson planning (Phase W8), module "Planejamento de aulas".** A Planejamento screen
  for teachers builds a classroom's class calendar from a plan, in code and without AI:
  class days and times, Brazilian holidays (Carnaval, Sexta-feira Santa and Corpus
  Christi from Easter) and blocked days left out, topics spread by size, a review every
  few classes and a closing mock exam, with the shortfall said up front. Each class can
  be marked given or cancelled, fixed, and gets objectives, materials, an online link, a
  note to students and a private note; replan from today keeps given and fixed classes;
  undo; `.ics` export. The AI plans the empty classes through the panel (objectives,
  activities with minutes, homework), checked in code. Publishing sends the calendar to
  the school's panel without private notes; students see their next classes on Today and,
  when the teacher chooses, topics open on the day of their first class.
- Planejamento: edit a class's day, time, length, kind and topics; split a class in two or
  merge it with the next; when marking a class given, untick what was left for later and
  replanning puts it back into the next classes; **Imprimir planejamento** writes a PDF
  (Typst) with the schedule as a table and a page per class plan, without private notes.
- Students' work per class: the progress sync carries, per class of the classroom's
  schedule, whether the student studied its topics by its day and how much homework they
  did, computed from saved study; the school's area shows the totals per class.
- Class reminders for students: the day before a class while the last homework is still
  open, and an hour before an online class (with a button to join); both in Settings.
- Today shows a quiet note when a teacher republishes a schedule that moved, cancelled or
  added classes.
- **Mock exams, stage 2: photo of the answer card.** The printed card carries the exam's
  number as a row of cells; "Ler foto do cartão" (answer-card mode) reads a JPG or PNG in
  pure Rust, without AI: the four corner markers, the perspective, each bubble's darkness.
  The marks fill the sitting on screen and doubtful ones (two marks, a faint mark) are
  highlighted; a card of another exam is refused. The student checks and hands in.
- Progress: mock exams per subject (average, the last sittings as bars, going up or down)
  and the topics that lost most points in the last three sittings.
- Mock exams: the notice's cut-off score, read only when its number is written in the
  notice (in percent or points), editable, and shown beside the result.
- **Lesson types (Phase 10).** Straight to the point, Balanced and In depth replace Quick,
  Normal and Full (saved values unchanged) and change what the teacher does, not only the
  length. The ▾ beside a new lesson picks the type, with a suggestion from the student's data;
  "Go deeper"/"Make it shorter" (or saying so) change the rest of a lesson; the last type per
  topic is remembered and the quiz size follows it.
- **Goals and profile (Phase 11).** Weekly goals per plan with suggested values and the week
  on Today; a Profile tab with a level from study and results, pinned badges, a calendar of
  study days and history per subject; a streak that forgives a rest day a week and days off;
  four secret achievements; a soft sound with each badge, which Settings can turn off.
- **Developer mode (Phase W9).** A hidden area (Settings → About, seven clicks on the
  version, or `--developer`) takes a code made by an admin in the panel and switches a
  production build to homologation or development: only the brand's compiled panel
  addresses, each environment with its own data folder and activation, production data
  never opened elsewhere; the token is checked offline and every hour with the panel.
- Reader: **One-page summary** of a handout (rules, one example, traps and summary) for
  lessons straight to the point; highlights stay in the full handout.
- Reader: **Report** a handout that came from the shared base (wrong, inappropriate,
  broken or other, with an optional note); the school's panel hides material three
  schools reported until an admin decides.
- The progress export (Markdown) ends with the profile: level, study time, streak and goals.
- Teachers on a new computer get their own published courses back as plans, and their
  progress on them and study time follow them like a student's (privacy policy
  `2026-11-17.3`).
- Examples `activate`, `planning_live` and `classes_live` for live checks with two
  copies of the app.

### Fixed
- "The speech synthesizer failed" on every sentence: the Piper voices were downloaded from
  `rhasspy/piper-voices` `main`, which since 2026-11-09 retrains them for Piper 1.5 with
  configs the Piper we ship refuses. Voices now come from the 2026-10-30 revision; one
  already downloaded from the newer one counts as missing, so the lesson offers to
  download it again.
- The school's icon from the panel now shows on the Windows taskbar and in the tray, at
  start and whenever the brand changes; before, the taskbar kept the executable's own
  icon and the tray never changed. The executable's file icon (desktop and Start menu
  shortcuts) is the build's and cannot follow a school.
- Handouts of a plan without a notice (a classroom's course) failed with "The model
  answered in an unexpected format": the plan's own subjects and topics now serve as
  the syllabus.
- An AI error no longer retries with an empty fallback model (the panel's models have
  none), which the panel refused with HTTP 422.

### Changed
- The lesson screen offers what a voice lesson still lacks (speech recognition, the
  teacher's voice and its program) with *Download now* and a progress bar, before a
  lesson starts and during one, instead of only pointing to Settings.
- Caderna: no "Made by" credit (`show_credit = false`).
- Lessons no longer stop when the student talks over the tutor. The student talks by
  pressing the microphone (or holding Space), and when the tutor finishes speaking with
  a question the microphone opens by itself for the answer, skipping the echo of its
  last words; with no answer within 30 seconds it stops listening quietly. The settings
  "I can interrupt the tutor" and "Hands-free" are gone. The transcript and chat panel
  stays open for the whole lesson. (Know Giver 0.41.0 and 0.41.2.)
- Speech recognition keeps lessons in Portuguese: when Parakeet writes any English word,
  Whisper (told the language is Portuguese) reads the whole answer again. Without Whisper
  installed, the lesson says to download it. (Know Giver 0.41.0.)
- Caderna terms and privacy policy `2026-11-17.2` (users agree again): reports on shared
  handouts send the reason and the note.
- Caderna terms and privacy policy `2026-11-17.1` (users agree again): the panel also
  receives, per class, whether each student prepared and did the homework, and the school
  sees the totals.
- Caderna terms and privacy policy `2026-11-16.2` (users agree again): the panel receives
  the class calendars teachers publish; private notes never leave the computer.

### Fixed
- Windows installer: reinstalling no longer leaves a second desktop icon. Other desktop
  shortcuts to the app (another name, the local desktop from before it moved to
  OneDrive, or a OneDrive sync copy) are removed; shortcuts to other programs are
  untouched. (Know Giver 0.41.2.)
- A student's class sync failed when a course had an empty syllabus name or a class an
  empty note: the panel sends empty texts as `null`, now read as empty.

## [0.4.0] - 2026-11-16

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
- Caderna terms and privacy policy `2026-11-16.1` (users agree again): local photo
  reading uses the graphics card through Vulkan on Windows and Linux, and nothing of it
  leaves the computer.
- Channel builds (`.github/workflows/channels.yml`) run only by hand
  (`gh workflow run channels.yml --ref <branch>`), no longer on every push, to save
  Actions minutes; releases still build from a `v*` tag.
- CI jobs stop after a time limit (Checks 60 min, packages 120 min) instead of the
  6-hour default, retry apt downloads, and Checks skip documentation-only changes.
- The local panel runs on port 8080 (`APP_PORT`), as the development channel expects.
- Caderna terms of use and privacy policy describe the school's restrictions, the
  watermark, the local check of open programs and the Wikimedia credits (version
  `2026-11-11.1`; users agree again).
- The brand downloaded from the panel applies as soon as it arrives (theme, title, icon,
  pictures and handout brand), without restarting the app.

### Fixed
- Two copies of the app on one computer (`KG_DATA_DIR`), such as a teacher and a
  student, keep separate device tokens in the credential store instead of
  replacing each other's; the standard data folder keeps its entry.
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
