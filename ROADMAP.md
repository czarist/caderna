# Roadmap — Know Giver White-Label

## Current Status

Version 0.3.0 of the white-label base, forked from Know Giver 0.39.3 on 2026-11-08; 0.2.0
added classes in the cloud (Phase W7); 0.3.0 adds voice downloads on Linux and macOS and
Cebraspe-style true/false quizzes.
Everything Know Giver does is here (edital → plan, voice lessons with whiteboard,
handouts and reader, quizzes and reviews, mock exams, written answers with local OCR,
challenges, five UI languages, account sign-ins, tray and reminders). Know Giver's own
phase history (Phases 1–7, all done) stays in the upstream repository; this file keeps
the white-label work and the phases still open.

A build carries one brand, read at compile time from `branding/<name>/brand.toml`
(`KG_BRAND_DIR`; default `branding/default`, Know Giver itself). See
[branding/README.md](branding/README.md).

**Builds** use every core (Cargo's default), including the llama.cpp and shader builds.
If a machine overheats, limit it for that run with `-j N` or `CARGO_BUILD_JOBS`.

## Phase W1 — Brand and demonstration builds (done in 0.1.0)

- [x] `kg-brand` crate: `brand.toml` read and checked at compile time
      (`crates/brand/config.rs`), exposed as constants; a bad value stops the build with
      the key's name.
- [x] Name everywhere: window, tray, sidebar, every text through `%{app}` /
      `%{company}` / `%{license}` in `locales/*.yml` (the UI's `t!` fills them in), the
      ChatGPT sign-in name, HTTP user agents.
- [x] Accent and highlight colours (light and dark), app icon, cover and reminder art,
      company mark; "Made by …" credit can be turned off.
- [x] Handouts: watermark, footer credit and its colour from the brand; the reader still
      leaves them out of selectable text.
- [x] Data folder, database and log named by the brand, so two brands on one computer
      never share data.
- [x] Windows exe icon and version info (company, product) from the brand.
- [x] Installer: name, publisher, folder, GUID, icon and file names from the brand
      (`scripts\package.ps1 -Brand branding\<client>`).
- [x] Updates only from the brand's own repository; none, and no source link, when empty.
- [x] Demonstration builds: days from the first start and/or a last date; days left in
      the sidebar; afterwards only the "demonstration ended" screen with the client's
      contact link; study data kept. Clock kept in the database and in the settings
      folder; earliest start and latest day win, so a clock turned back does not help.
- [x] `branding/default` (Know Giver) and `branding/example` (template with a demo).

## Phase W2 — Per-client legal texts

- [x] Roles: the client is the provider to its students and the data controller;
      Trentin licenses the software (`[legal] provider` / `licensor` in the brand).
- [x] Legal texts from the brand: a `legal/` folder inside the brand replaces
      `docs/legal/` (all fifteen files), with its own `[legal] version` used as
      `TERMS_VERSION`, so each brand asks its students to agree on its own schedule.
- [x] Release gate: `[legal] reviewed` and `commercial_reviewed` must be `true` before a
      commercial package (`metadata release-ready` in the packaging scripts and CI).
- [x] A licence agreement template between Trentin and the client (outside the app):
      drafted for Caderna in Claude Docs (parties, LLMs, allowance and overage, LGPD
      roles, Anexo I with prices), waiting for the lawyer's review below.
- [ ] Lawyer review of each client's texts before its first sale (Caderna: drafts only).

## Phase W3 — Remaining art and names

- [x] Teacher portraits and stickers optionally replaced per brand
      (`[company] teachers_dir`, `stickers_dir`, same file names as the bundled art).
- [x] Settings → About: the client's support contact (`[company] support_url`) and the
      licence status.
- [x] Prompts: no prompt names "Know Giver".
- [ ] Screenshots per brand for the client's material (`KG_SNAPSHOT_DIR` with
      `KG_BRAND_DIR`), including the licence activation in all five languages (needs a
      brand with a public key and a GPU run of the snapshot tests).

## Phase W4 — Packaging and releases per client

- [x] macOS and Linux packages from the brand (`scripts/package-unix.sh`: bundle name,
      `.desktop` file, icon, archive names).
- [x] One-line installs and the macOS/Linux updater script take the repository and
      folder from the brand.
- [x] Release workflow per brand: `brand`, `private_brand_repository`,
      `target_repository` inputs in `.github/workflows/packages.yml`; publishing is
      explicit and refuses unreviewed brands.
- [x] Code signing scripts (`scripts/sign-windows.ps1`, `ci-signing-*.{sh,ps1}`) that
      read certificates only from CI secrets.
- [x] Client brand folders can live in a private repository, checked out under
      `branding/private/` (ignored by git).
- [ ] Signing certificates under each client's publisher name (owner/client task).

## Phase W5 — Stronger demonstrations and licences

- [x] Signed licence keys (Ed25519, public key in the brand, engine `license.rs`): bound
      to brand, customer, seats and dates; verified offline; activation in Settings and
      on the "demonstration ended" screen. Keys are issued with
      `cargo run -p kg-engine --example license_issue` (private key never in the repo).
- [x] `[licensing] required = true`: a build without a valid licence stops like an
      ended demo; an expired licence stops it again, study data kept.
- [x] Demo watermark on handouts while the app is not fully licensed.
- [x] Convert a demo to the full version in place: activating a licence keeps plans and
      progress (test in `license.rs`).
- [ ] Optional online check when the client wants it (seats in use), off by default and
      stated in the privacy policy. Needs a real client service contract first.

## Phase W6 — Caderna dashboard (CADERNA-DASHBOARD)

A separate Laravel 13 + Filament 5 project. Its README holds the API contract.

- [x] Panel: clients with validation status, per-client brand (colours, images, texts,
      versioned), licences and evaluation tokens signed byte-identically to
      `license_issue`, devices and seats, AI usage per client.
- [x] OpenAI-compatible AI gateway (`/api/v1/ai`): provider keys and models only in the
      panel's `.env`, aliases for the app, streaming, monthly quota, images only to the
      vision model.
- [x] Runs with Docker (PHP 8.3 + Apache and MySQL 8.4) or a local PHP install; the
      root page is a Caderna "coming soon" site, the panel lives at `/admin`.
      Repository: <https://github.com/czarist/caderna-dashboard>.
- [x] App: after the offline check, activate online (`POST /api/v1/activations`); device
      token in the OS credential store, renewed silently before it expires (`renew`),
      check-in at start and every hour, 14 days offline, refusals lock the app with their
      reason; "Sign out this computer" frees the seat. Hashed machine id, no computer name.
- [x] App: download the client's brand (`GET /api/v1/brand`, ETag) and cache it; names,
      colours and links apply as soon as they arrive (at start and every hourly check),
      and the kept brand applies when the app opens.
- [x] App: AI through the panel with the device token (`[server] text_model` /
      `vision_model`); the AI choice is hidden in setup and Settings.
- [x] Caderna privacy policy and terms (pt-BR only, Brazil only, `[locale]`): what goes to
      the panel, with the legal version bumped.
- [x] Apply the panel's pictures (icon, cover, reminder art, mark, handout watermark) and
      texts at runtime, as soon as they arrive, after checking them.
- [ ] The device token's credential-store entry is one per brand, so two data folders
      (`KG_DATA_DIR`) on one computer share it; key it by data folder too.
- [x] Panel address per build: `[server] dev_url` (localhost) for `cargo run` and tests,
      `[server] url` or `KG_SERVER_URL` for releases; never read on the student's computer.
- [x] Modules per licence from the panel (Videoaula, Criação de conteúdos): hidden in the
      sidebar and tour, refused by the engine.
- [x] Panel: client area (`/cliente`) where a school enrols its students, each with a
      one-seat licence by email, and picks their plan, within the limits set by the admins.
- [x] Panel: charges. The admins bill a client for anything; the client pays in its area by
      Pix or credit card (up to 3 instalments without interest, up to 10 with 1.99% a month,
      chosen by the client) through Asaas, with the
      Asaas webhook and Caderna-themed emails.
- [x] Charges tested on the Asaas sandbox: Pix, card in 3 instalments, refusal, webhook,
      cancelling.
- [x] Subscriptions: a recurring fee per client, issued 10 days before each due date by a
      daily run; the client picks the due day and automatic card payment (Asaas card
      token) in its area; Pix reminders 3 days before and on the day; unpaid 2 days after
      the due date, the client's apps are refused with `billing_paused` (contact support)
      and open again by themselves once it is paid.
- [x] LLM overage: past the monthly allowance the apps keep going up to an optional cap,
      the client is emailed once a month, and last month's extra tokens become one charge
      (price per million, tied to the subscription) on the 1st.
- [x] Short access codes (`XXXX-XXXX-XXXX-XXXX`) instead of the long key: the student
      types the code, the panel answers with the signed key, which the app keeps and
      checks offline; the email carries a link to the code with a copy button.
- [x] Student restrictions per school (*Permissões dos alunos* in the client area):
      exporting handouts and screen capture, sent with the licence as `restrictions`,
      never for teachers. The engine refuses blocked exports and the reader disables them;
      on Windows the window is left out of screenshots and recordings.
- [x] Capture deterrents where the system cannot block: the student's name and email as a
      faint watermark on every handout page (all systems, sent by the panel only while
      capture is blocked); macOS marks the window as not shared; macOS and Linux cover the
      content while the window is out of focus or a known recorder or screenshot tool runs.
- [ ] Try the macOS and Linux capture deterrents on real machines: whether
      `sharingType = none` still hides the window from Cmd+Shift+3/5 and QuickTime on
      macOS 15 and later, and which screenshot tools take the focus on GNOME and KDE.
      Linux (X11, Wayland) offers no way to leave a window out of captures. The handout
      PDFs also stay readable in the data folder: the restrictions cover the app, not
      copying files by hand.
- [x] Development test accounts (`DevTestSeeder`) and the pt-BR MVP test plan
      (`docs/mvp-test-plan.md`).
- [ ] Production email for the panel (SMTP and a sender domain with SPF/DKIM) before the
      first school, so access-code links and charges do not land in spam.
- [ ] Subscriptions: card tokenization enabled on the production Asaas account, and the
      `scheduler` service (or a cron running `schedule:run`) on the production server.
- [ ] Charges: the production Asaas key and webhook token before the first real charge.
- [x] Build channels per branch (`main` production, `hom` homologation, `dev`
      development), each with its panel address, installed side by side and labelled
      in the app; CI replaces each channel's download when run by hand.
- [ ] Put the panels online at the addresses the builds use: `https://caderna.ia.br`
      (production) and `https://hom.caderna.ia.br` (homologation), with HTTPS, the same
      licence signing key pair as the app's public key, and their own database, Asaas
      and mail settings (homologation on the Asaas sandbox).
- [ ] Sign the channel builds (Windows and macOS) once the certificates exist; today
      homologation and development downloads are unsigned.

## Phase W7 — Classes in the cloud (done in 0.2.0)

Needs a brand with a panel (`[server]`); without one the app works as before.

- [x] Teachers and students: the panel's client area enrols each person as a teacher or a
      student (`students.role`), sent to the app with the licence (`license.role`).
      A licence without an enrolment keeps today's behaviour (`ClassRole::Owner`).
- [x] Students only use: the engine refuses creating, deleting or reading plans, preparing
      content and importing board material (`Command::creates_plans`); the UI hides those
      actions and the board screen, and shows "your plans come from your class".
- [x] Classrooms (turmas) in the client area: name, students and courses; a "Desempenho"
      view per classroom (topics mastered, quiz accuracy, lessons, study time, last activity).
- [x] Teachers publish a plan to classrooms from the new Classes screen (`POST`/`PUT
      /api/v1/courses`): subjects, topics with stable keys, the syllabus, handouts and quiz
      banks. Republishing bumps the version; students keep progress on topics that stay.
- [x] Students' apps download their courses at every check (read-only plans with the
      teacher's handouts and questions) and remove courses taken away from their classrooms.
- [x] Progress in the cloud (`/api/v1/progress`): topic status and review schedule,
      quizzes, lesson recaps, reviews, mistakes notebook and study time, merged without
      duplicates when two computers studied offline (`kg_study::sync`); restored on a new
      computer.
- [x] Shared base (`/api/v1/shared-materials`): before asking the AI for a plan, a
      handout or a quiz bank, the app looks up a hash of the request; what the AI makes is
      shared with every school. Only AI output; board material, edits and student data stay
      out. Admins list and remove items in the panel ("Base comum").
- [x] Purpose of the plan (ENEM, vestibular, concurso, OAB/certification, other), asked
      every time a plan is created; a note per purpose goes to every teaching prompt
      (`kg_ai::ExamPurpose`, Portuguese and English).
- [x] Handouts end with bibliographic references (all five prompt languages), checked by
      the handout reviewer and shown in the PDF.
- [x] Caderna privacy policy and terms describe classes, the cloud progress and the shared
      base; legal version `2026-11-10.1`.
- [ ] Teachers' own progress and plans across their computers (today only students sync).
- [ ] Moderation flow for the shared base beyond removal (reports from schools).
- [x] Wikipedia pictures in handouts credit their author and license (from Wikimedia) in
      the caption, and the topic's bibliography lists the article and the picture file
      (ABNT in Portuguese, all five languages); handouts saved earlier keep their pages.
- [ ] Check references against a bibliographic database before showing them.

## Phase W8 — Lesson planning for teachers (planned, not started)

**Goal:** a "Planejamento" module in the teacher's area where the teacher schedules the
classes of a classroom from the plan's content: on which days and times each class
happens, which topics it covers, what it aims at, which activities and materials it
uses and what the students do before and after. The teacher publishes it with the
course; students see their class calendar, what to prepare and their homework; the
teacher sees who did it.

Needs a brand with a panel (`[server]`) to publish; the planning itself works locally.

### Concepts

- **Schedule (cronograma):** one per published plan and classroom, with a start date, an
  end date (or the exam date), the classroom's time zone (default `America/Sao_Paulo`),
  class days and times (for example Mon and Wed 19:00–20:40), class length, blocked days
  and the topic release rule (below).
- **Class session (aula):** a stable key (like topic keys, so republishing keeps student
  progress), date, start time, minutes, kind (presencial, online with an `https` link only,
  or self-study), the topics it covers (topic keys from the plan, one topic may span
  several sessions), objectives, activities in order with minutes each, materials,
  homework, a public note for students and a private note for the teacher (never
  published), and a status (planned, given, cancelled, moved).
- **Activity kinds:** teacher explanation, voice lesson with the AI teacher (opens the
  topic's lesson in the student's app, with the lesson type of Phase 10 once it exists),
  handout reading (pages), quiz (bank and number of questions), spoken review, written
  answer (proposal), mock exam (Phase 12), discussion/exercise in class, free text.
- **Homework (tarefa):** activities the student does between two sessions, with a due
  session; done or not is computed in code from saved study (lesson taken, quiz answered
  with N questions, handout read, written answer sent), never declared by the AI.
- **Release rule:** "all topics open" (today's behaviour) or "open as the classes happen"
  (a topic opens for students on the day of its first session; reviews and the mistakes
  notebook keep working on opened topics). Chosen per schedule.

### Generating the schedule (code first, AI second)

- [ ] Session calendar computed in code (`kg_study::schedule`): class days between start
      and end, minus blocked days and holidays. Holidays per region computed in code
      (Brazil: national fixed dates plus Carnaval, Sexta-feira Santa and Corpus Christi from
      Easter); state and city holidays added by the teacher as blocked days.
- [ ] Topic distribution computed in code: topics in the plan's order, weighted by their
      size (handout length or estimated lesson minutes) and by the notice's weight when the
      plan has it (mock-exam sections, Phase 12); a review session every N sessions
      (default 6) and a mock exam before the end when the plan has an exam format.
      Too many topics for the sessions is said up front, with the shortfall in hours and
      options: more sessions, longer sessions, or mark topics as self-study.
- [ ] Per session, the AI suggests objectives, activities with minutes, materials from
      what the plan already has (handouts, quiz banks, board material) and homework
      (`lesson_planning` prompt in every `crates/ai/prompts/<lang>/`, JSON, same keys and
      placeholders; teaching culture from `culture.rs` and the plan's `ExamPurpose`).
      Checked in code: topic keys exist in the plan and belong to that session, activity
      minutes add up to at most the session length, materials exist, homework activities
      are of known kinds; a failed check asks once more, then falls back to an empty session
      with only the topics. Batches of sessions, like mock-exam questions.
- [ ] pt-BR plan-of-class conventions in the prompt note: objetivos (verbs of Bloom's
      taxonomy), conteúdo, metodologia, recursos, avaliação; the other regions follow their
      own culture note (Portuguese and English through `PromptLanguage::pick`).
- [ ] Shared base (W7) is not used for schedules: they depend on the school's calendar.

### Teacher's editing

- [ ] New "Planejamento" screen in the teacher's area (beside Classes): a calendar view
      (month and week) and a list view; select a plan and a classroom; the wizard asks
      start, end, days, times, length, blocked days and release rule, then generates.
- [ ] Edit a session: topics, objectives, activities (add, remove, reorder, minutes),
      materials, homework, notes, kind and link. Move it to another day by dragging or by a
      date field; split a session in two or merge two.
- [ ] Lock sessions; **replan**: when a session is cancelled or moved, or a topic took
      longer, the following unlocked sessions are redistributed in code; given and locked
      sessions never move. Show a summary of what changed before applying.
- [ ] Mark a session as given (with what was really covered, if different), so the
      replanning starts from the truth.
- [ ] Undo for the last changes; the schedule is saved locally as edited (SQLite tables
      `class_schedule`, `class_session`, `class_session_topic`, `class_session_activity`,
      with migrations), so nothing is lost before publishing.
- [ ] Export: printable PDF (Typst, `crates/handout/templates/lesson_plan.typ`): the
      schedule as a table and one page per session in the plan-of-class format; and an
      iCalendar file (`.ics`) of the sessions for the teacher's calendar.
- [ ] Engine commands (`GenerateSchedule`, `EditSession`, `ReplanSchedule`,
      `PublishSchedule`, `ExportSchedulePdf`, `ExportScheduleIcs`) refused for students
      (`Command::creates_plans`) and when the licence lacks the module.
- [ ] Owner role (no panel): decide whether a student studying alone may use the same
      screen as a personal study calendar (see Decisions).

### Panel (CADERNA-DASHBOARD)

- [ ] Tables for schedules and sessions tied to a course and a classroom; API
      `PUT /api/v1/courses/{course}/schedules/{classroom}` (teacher, whole schedule,
      bumps a version like courses) and the schedule inside what students download at
      every check. Private teacher notes never leave the teacher's computer.
- [ ] Students' homework status in the progress sync (`/api/v1/progress`), computed by the
      app from saved study; merged without duplicates like the rest of `kg_study::sync`.
- [ ] Client area: "Agenda" per classroom (read-only for the school), and in "Desempenho"
      per session: how many students did the preparation and the homework.
- [ ] New licence module "Planejamento de aulas" (`Module::Planning`), chosen per client
      in `allowed_modules` like Videoaula and Criação de conteúdos; README API contract and
      AGENTS.md updated in both languages.

### Students' side

- [ ] "Agenda" on Today: next class (day, time, kind, link), what to prepare before it,
      homework due and late; a calendar of the classroom's sessions in the plan screen.
- [ ] Topics follow the release rule; a closed topic shows the day it opens.
- [ ] Reminders through the existing study reminders: the day before a class with
      homework still open, and an hour before an online class (both can be turned off).
- [ ] A changed schedule (republished) shows a quiet note with what moved.
- [ ] Homework counts for challenges only if it fits existing families; no new
      pressure (no countdowns, no "late" in red), in the calm style of Phase 11.

### Legal, languages and docs

- [ ] Caderna privacy policy and terms (pt-BR, `branding/caderna/legal/`): the panel now
      keeps schedules and each student's homework status, visible to the school's
      teachers; bump the legal version and `TERMS_VERSION`. `REVISAO-JURIDICA.md` gets the
      item for the lawyer.
- [ ] Every UI text in the five `locales/*.yml`; the prompt in the five prompt folders;
      README sections (all `README*.md`) and screenshots of the Planejamento screen and
      the student's Agenda in each language.

### Tests

- [ ] `kg_study::schedule`: calendar with holidays and blocked days, Easter-based dates
      over several years, distribution by weight, review and mock-exam placement,
      shortfall report, replan never moving given or locked sessions, stable session keys
      across republishing.
- [ ] Checks of the AI's answer (wrong topic key, minutes over the length, unknown
      material) with a fake client; prompt parity (`cargo test -p kg-ai`) and locale
      parity (`cargo test -p kg-ui`).
- [ ] Engine: students and closed modules refused; release rule hides and opens topics
      by date (fixed clock); homework done computed from saved study; sync round-trip.
- [ ] Panel: feature tests for the schedule API, versioning, the student download and the
      client area pages.

### Decisions to take before starting

- Module name and whether it is sold apart (proposed: "Planejamento de aulas", its own
  module) or comes with Criação de conteúdos.
- Whether Owner (no panel) gets the screen as a personal study calendar (proposed: yes,
  later, reusing the same scheduler without publishing).
- Default release rule (proposed: all topics open, as today).
- Whether school coordinators may edit schedules in the client area or only view them
  (proposed: view only; editing stays in the app).

### Order of work

1. `kg_study::schedule` (calendar, holidays, distribution, replan) with tests.
2. Local screen, storage, editing and PDF/ICS export for teachers.
3. `lesson_planning` prompt in five languages and the checks.
4. Panel tables, API, module and client-area pages.
5. Publishing, student Agenda, release rule, reminders and homework status in the sync.
6. Legal texts, READMEs and screenshots.

**Acceptance criteria:** with a real plan, a teacher generates a semester schedule for a
classroom in a few minutes, with holidays skipped and topics spread by weight; moving or
cancelling a class replans the rest without touching given ones; the published schedule
reaches the students' apps at the next check, shows the next class and homework on Today,
and the teacher sees, per class, who did the preparation and the homework.

## Phase W9 — Developer mode instead of three channel builds (planned, not started)

**Goal:** one production executable per brand. A developer token generated in the
panel (Laravel) unlocks a developer area in the app where the owner and the team point
it at homologation or development: which panel API it talks to and which local database
it uses. The `hom` and `dev` installers stop being needed for testing.

**Today:** three builds per brand (`KG_CHANNEL`, `.github/workflows/channels.yml`), each
with its panel address chosen at build time, its own name, data folder, executable,
installer GUID and credential entry. CLAUDE.md says the panel address is "chosen at build
time, never read on the student's computer"; this phase changes that rule and needs the
owner's sign-off before it starts (see Decisions).

### Token (panel)

- [ ] Panel (production, `/admin`): "Tokens de desenvolvedor" page, admins only. A token
      names a person, the environments it opens (`hom`, `dev`, or both), an expiry
      (default 30 days, at most 90) and optionally one device (hashed machine id, as in
      activations). Shown once on creation; the panel keeps only its hash; revoking is
      one click; every use is logged (who, when, which environment, app version).
- [ ] The token is a signed claim like the licence (Ed25519, its own prefix
      `kg-dev-v1\0` + claims JSON, the same key pair or a separate developer key, see
      Decisions), so the app checks signature, brand slug and expiry offline; the app also
      asks the production panel at activation and every hourly check whether it was
      revoked, and leaves developer mode when it was.
- [ ] Short code like access codes (`DEV-XXXX-XXXX-XXXX`) that the panel exchanges for the
      signed token, so nobody types a long key.

### Developer area (app)

- [ ] Hidden entry: Settings → About, clicking the version seven times, or the
      `--developer` flag; asks for the token. Nothing about it appears in the UI, tour or
      README for students.
- [ ] Environment switch: **Production / Homologation / Development**. The addresses come
      only from the brand (`[server] url`, `hom_url`, `dev_url`, compiled in); no free URL
      field, so a token cannot point the app at an unknown server. Optional "custom
      `localhost` port" for development only (`http://127.0.0.1:<port>`).
- [ ] Database per environment: each environment uses its own data folder inside the
      brand's (`<data_dir>/env/hom`, `<data_dir>/env/dev`), with its own SQLite file, plans,
      progress, handouts, cache of the panel brand and courses; production data is never
      opened, migrated or synced while another environment is on. Options in the area:
      "start empty", "copy my production data into this environment" (one-way copy,
      never back), "reset this environment".
- [ ] Credential entry per environment (device token, licence) so hom and dev activations
      never replace the production one; this also closes the W6 item about one entry per
      brand.
- [ ] Switching restarts the app (engine, sync and AI client start from the chosen
      environment); the choice is remembered until the token expires or is revoked, then
      the app returns to production on its own and says so.
- [ ] While not in production: a coloured strip and the window title say the environment
      (reusing today's channel strip), the updater stays on production releases, and logs
      record the environment.
- [ ] Developer tools in the area (optional, after the switch): open the data folder,
      show the current licence claims and modules, force a panel check, copy the last
      panel errors.

### What stays and what goes

- [ ] Engine and UI read the environment at start (`Environment` in engine settings)
      instead of `KG_CHANNEL` at compile time; `kg-brand` keeps all three addresses.
- [ ] `cargo run` keeps starting in development without a token (debug builds only), so
      day-to-day work does not change.
- [ ] Once the developer mode is tested: retire the `hom` and `dev` installers from
      `channels.yml` (or keep `hom` only for testers outside the team, see Decisions),
      update CLAUDE.md (Git workflow and `[server]` rules), `branding/README.md`,
      `brand.toml` comments and the packaging scripts. Branches `dev`/`hom`/`main` stay as
      the code flow.
- [ ] Students: a production build with no developer token behaves exactly as today; the
      production panel never hands a developer token to a student or school account.

### Security

- [ ] Only addresses compiled into the brand; HTTPS for `url` and `hom_url`.
- [ ] Token checked offline (signature, slug, expiry, device) and online (revocation);
      expired or revoked returns to production without touching that environment's data.
- [ ] No token, key or environment data in exported files or crash messages.
- [ ] Rate limit and audit log on the panel's code exchange; tokens never in URLs.

### Legal and languages

- [ ] Privacy policy: nothing changes for students (the area only works with a developer
      token); confirm with the lawyer note in `REVISAO-JURIDICA.md`. If the panel logs the
      developer's machine id, the internal policy for the team says so.
- [ ] Developer-area texts in the five `locales/*.yml` (`cargo test -p kg-ui`), even if
      only the team sees them.

### Tests

- [ ] Token: valid, wrong signature, other brand, expired, other device, revoked online.
- [ ] Environment folders and credential entries never cross (production data untouched
      after a session in dev and in hom); copy is one-way; reset removes only that
      environment.
- [ ] Only compiled addresses are reachable; the custom port works only for development.
- [ ] Panel: token creation, hash only stored, code exchange, revocation, audit log.

### Decisions to take before starting

- Changing the "never read on the student's computer" rule: the addresses stay compiled
  in, only the choice between them moves to runtime behind the token (proposed: accept).
- Same Ed25519 key as licences or a separate developer key (proposed: separate key, so a
  leak of one does not affect the other).
- Keep a `hom` installer for outside testers (schools trying a release) or give them
  tokens too (proposed: tokens with `hom` only and a short expiry).
- Who may create tokens: admins only, or also a "Desenvolvedor" role (proposed: admins).

**Acceptance criteria:** with only the production installer, the owner types a developer
code, switches to homologation, studies with a separate database against
`hom.caderna.ia.br`, switches to development against `localhost`, and back to
production finds their data exactly as before; revoking the token in the panel brings the
app back to production at the next check.

## Voice programs per system

- [x] Voice programs download on Windows, Linux (x86_64, aarch64) and macOS, pinned
      with checksums: Piper 2023.11.14-2 and sherpa-onnx v1.12.20 (Kokoro, Parakeet)
- [ ] Piper on macOS: its official macOS archives are broken (an Intel binary without
      its libraries), so macOS uses the Kokoro voices; Settings says so. Revisit when a
      working build exists

## Licences to check before selling

- [x] Dii voice (OpenVoiceOS/pipertts_pt-BR_dii): CC BY-NC-ND 4.0, non-commercial. No
      longer offered or downloaded; old settings that chose it still load.
- [ ] Other voices, models and art under the README's Credits: confirm each allows
      commercial redistribution.
- [ ] Account sign-ins (ChatGPT, Claude Code, Copilot): their terms were accepted for
      Know Giver by Trentin; confirm they cover a product sold under another name.

## Know Giver phases still open

Carried over from Know Giver and kept because they serve schools and students; the
rest of the inherited work (managed local AI, proxy of keys, other languages and
regions, account sign-ins) was dropped from this roadmap on 2026-11-13.

## Phase 10 — Lesson types: straight to the point, balanced, in depth (planned, not started)

**Goal:** the student chooses how deep each lesson goes, at the moment they start it:
- **Straight to the point (summary):** what the exam asks about the topic, the rules
  and the usual traps, one example, short checks. For reviewing, for the days before the
  exam, or for a topic already known.
- **Balanced:** today's normal lesson; every point of the handout, one example and one
  check each.
- **In depth:** the reasoning behind each rule, exceptions, several examples, how the
  board has asked it in past exams, links to related topics, more checks and a longer
  written question at the end. For a new or weak topic.

**Status:** a global "Lesson length" exists in Settings (`LessonLength`: Quick, Normal,
Full) and only changes the number of steps and checks (`max_steps`, `teaching`). It
applies to every lesson until changed.

- [ ] Pick the type when starting a lesson (three options on the lesson start, the
      Settings value as default), not only in Settings.
- [ ] Rename the levels in every locale to the three types, keeping the saved values
      (`quick`, `normal`, `full`) so settings and old lessons still load.
- [ ] The type changes the content, not only the length: what each prompt asks for in
      every `crates/ai/prompts/<lang>/` and in `LessonLength::teaching`, following the
      region's exam culture (`culture.rs`).
- [ ] Suggest a type from the student's data: a topic never studied or with low quiz
      scores suggests in depth; a topic mastered or an exam date close suggests straight
      to the point. The student always decides.
- [ ] Change during the lesson: "go deeper" and "make it shorter" buttons (and the same
      by voice) adjust the remaining steps without restarting.
- [ ] Remember the last type per topic and show it in the topic's history.
- [ ] Handouts and quizzes follow the type: a summary handout (one page of rules and
      traps) for straight to the point, the full handout for in depth; quiz size and
      difficulty to match.
- [ ] Every UI text in the five locales; README sections and screenshots updated in each
      language.

**Acceptance criteria:** the same topic in the three types gives visibly different
lessons (length, number of examples, past-exam questions, final check), and switching
mid-lesson keeps what was already covered.

## Phase 11 — Goals and achievements (started)

**Goal:** reward steady study the way Steam rewards play, with achievements, goals and a
profile of progress, without turning the app into a game. It stays calm: no countdowns,
no loss messages, no comparison with other students unless they choose it.

**Status:** challenges and stickers shipped in 0.38.0, with levels; 0.39.0 added the mock
exam families: 164 in 36 families.
Goals the student sets, hidden achievements, the profile and streak rules below are still
to do.

- [x] Challenges with an original sticker each, computed in code from saved study
      (`kg_study::challenge`, a table of families and targets), never by the AI. Per plan:
      lessons and lesson days, quizzes, right answers, perfect (5+ questions, all right) and
      high (80 %+) quizzes, missed questions won back, a clean mistakes notebook, topics
      studied and the share of the notice, every subject started, plan weeks complete (the
      plan's check-mark rule) and the whole plan, topics, subjects and the whole notice
      mastered, spoken reviews, past exams, handouts read, highlights, handouts made,
      written answers and their estimated scores, mock exams handed in and their scores
      (70 %+ and 90 %+, since 0.39). Overall, once: study hours, study days, a
      long day, plans made, totals of lessons, quizzes, right answers and mastered topics,
      and stickers collected. Keys are stable; First Step, Comeback, Plan in Motion and Week
      on Track keep their names.
- [x] Levels inside a family with growing targets and Steam-like progress on locked
      ones ("12 / 30"): the rim's colour (bronze, silver, gold, jade) and one to three dots.
- [x] Achievements screen: one line per family with the next level, progress and a
      button to the lesson, quiz, review, board, handouts or written answers; targets a
      plan is too small for are said so; the next one on Today; a showcase of every sticker
      grouped and filtered, with details on hover. Unlocks are kept per plan (unique per
      challenge and plan, kept when the plan is deleted) or once overall, recomputed on
      start, so study saved before a grant or before this version is recognised; a day the
      history cannot tell is shown as such. The notice waits until no lesson, quiz question
      or review is open, is shown once, and many at once become one line.
  - Gap: right answers to old mistakes were not recorded before this version (the
    mistakes notebook deleted them), so won-back questions count only from now on; the
    screen says so. Reviews are dated from now on (`review_log`); an older review counts
    with an unknown day. Handouts read and topics mastered have no day on record, so a
    grant found on start says the day is unknown.

- [ ] Hidden achievements shown as "secret" until unlocked, a few only.
- [ ] Goals the student sets: hours per week, topics per week, a target quiz score by a
      date; the study plan suggests realistic values from the exam date and the
      student's pace. Progress on the home screen, a gentle note when a week is missed.
- [ ] Profile screen: level and experience from study time and results (not from clicks),
      showcase of chosen medals, calendar heat map of study days, history per subject.
- [ ] Quiet unlock notice (a small toast with the medal and a sound that can be turned
      off), never in the middle of a lesson; collected at the end.
- [ ] Streaks forgive: one rest day per week keeps the streak, and travel or illness can
      be marked so it does not break.
- [ ] Everything local in SQLite, kept per plan and in total; export with the rest of
      the data. Optional sharing later (an image of the profile), never an online
      ranking by default.
- [ ] Achievement names and descriptions in the five locales; icons in the app's visual
      identity, not stock trophies; README sections and screenshots in each language.

**Acceptance criteria:** a student who studies a week sees achievements unlocked from
real data, progress on locked ones and on their own goals, and nothing interrupts a
lesson.

## Phase 12 — Mock exams (simulados) (started)

**Goal:** a whole exam in the notice's format, against the clock, on screen or on paper,
graded in code, so the student rehearses the real sitting.

**Status:** stage 1 shipped in 0.39.0. Stage 2 (reading a photo of the printed answer card)
is next, in 0.40.

- [x] The exam's format per plan (`kg_study::mock::ExamFormat`): time, sections per plan
      subject with questions and points each, multiple choice (2 to 5 options) or
      true/false, wrong cancels right, essay. Read from the notice's rules (the text before
      the syllabus) by the AI (`exam_format` prompt, five languages) and kept only when its
      counts are written in the notice; otherwise estimated from the plan's subjects. The
      student can change every number; the source is shown (notice, estimate, student).
- [x] Questions written per subject in batches of ten, three at a time
      (`exam_questions` prompt), spread over the plan's topics, starting further down each
      subject on every new exam; past exams on the board guide the style. Checked in code:
      option count, answer index, empty and repeated prompts; options shuffled; a short
      batch asked once more. Essay proposal from the written-answer pipeline when the
      notice has a written test.
- [x] Printable PDF (Typst, `crates/handout/templates/exam.typ`): booklet in two columns,
      numbered essay sheet, answer card with fixed geometry (`CardLayout`, millimetres) and
      four corner markers; the exam's code on every page.
- [x] Timed sitting: deadline from the saved start time, so closing the app does not stop
      it; answers saved as marked; one sitting at a time; sticker notices wait. On screen
      or answer card only (for paper). Handing in after the time is allowed and shown.
- [x] Grade in code against the saved key, per subject with points and the
      wrong-cancels-right rule; missed questions to the mistakes notebook; essay to Written
      answers; answer key with explanations. Ten challenges (`mocks`, `mocks_high`,
      `mock_top`).
- [ ] Stage 2 (0.40): photograph the printed answer card; find the four markers, correct
      the perspective and read each bubble's darkness at the `CardLayout` positions, in
      pure Rust, no AI; the student checks the grid read on screen before it is graded.
      The exam code on the card picks the exam.
- [ ] Average and trend across mock exams per subject on Progress; a suggested topic
      list from the subjects that lost the most points.
- [ ] Cut-off score from the notice, when it has one, shown beside the result.

**Acceptance criteria:** with a real notice, the format read matches the notice (or the
student fixes it in a minute); a generated exam prints cleanly; a sitting survives closing
the app; the grade matches a hand count.

## Not yet verified
- Mock exams (0.39.0): checked by unit tests (format bounds, the notice's numbers,
  batches, question checks, grade with and without wrong-cancels-right), engine tests (a
  sitting graded with mistakes to the notebook, a sitting kept across a restart, the PDF
  written), PNG renders of the PDF and GPU screenshots in the five languages
  (`mock_snapshots`). Not yet run against a real model and a real notice (reading the
  format, writing 60+ questions, the essay proposal), not yet printed on paper.
- Challenges and stickers (0.38.0): checked by engine tests on a database file
  closed and reopened (partial progress, completion, two plans, the same study checked
  twice, a grant missed before closing, overall stickers won once, a 0.37 database
  migrated), by rendering every sticker in colour and grey, by GPU
  screenshots in the five languages (`challenges_snapshots`), and with the release
  build on a copy of a real 0.37 database. Not yet clicked through by hand on an
  installed build; F11 and the hover not checked on macOS and Linux.
- Brazilian exam culture (v0.34.0): a teacher who prepares students for ENEM, vestibulares
  and concursos should review the Brazil note (`crates/ai/src/culture.rs`): boards,
  question formats, written-test conventions and grading scales.
- Updates and one-line installs (v0.33.0): need the first published release.
  - The release job, its asset names and `SHA256SUMS.txt` on a real tag.
  - `scripts/install.sh` on macOS (Apple silicon) and Linux, and `scripts/install.ps1` on
    Windows, from the README commands.
  - In-app update end to end: the bar, download, checksum, silent installer on Windows (it
    closes the app and opens the new version) and the install script on macOS and Linux.
  - No Intel macOS or ARM Linux package yet: those systems get the release page.
  - Desktop shortcut question and uninstallers (v0.33.1): the Inno uninstaller's study-data
    question on Windows, and `uninstall.sh` on macOS and Linux.
- Written answers with real handwriting and the owner's vision model (v0.29.0–v0.30.1):
  transcript faithfulness, photo warning thresholds, grades against a human correction;
  see [docs/validation-discursive.md](docs/validation-discursive.md).
- Vision requests with short reasoning (v0.30.1): a dense page read on the first try.
- Lesson depth with the real model (v0.27.0): does the tutor cover every key point and
  every quiz subject before `ETAPA:`, and how long do Normal and Full lessons take?
- Reader autoscroll and dragging with a real mouse (v0.27.0).
- Long spoken answers with the owner's microphone after the turn changes (v0.26.2):
  no cut-offs mid-sentence, and fewer wrong verbs.
- Captions in step with the voice in a live lesson, for Piper and Kokoro teachers,
  including interruptions and a voice failure mid-reply (v0.26.1).
- Lesson plan with the real model: `ROTEIRO:` / `ETAPA:` / `FIM` lines (v0.26.0).
- Reader with a long whole-plan handout: memory, scrolling and render times.
- Handout pictures chosen by the live writer, and how often Wikipedia has one.
- Microphone changes of v0.25.0 with the user's microphone (quiet speech, echo).
- Kokoro Portuguese listening comparison against Piper on real tutor answers. Accented
  text now reaches the Windows runtime; subjective quality is still open.
- Handout factual correctness: the review pass reduces errors (see Phase 5) but does not
  eliminate them.
- Tutor answers in lessons are not fact-checked.
- Voice loop with a real microphone and speakers.
- Installer build/run; Inno Setup 6 compiler was prepared locally but the final release
  installer was not built before the user requested closing work.

## Before a public release
- [ ] Security audit:
  - updater (engine `update.rs`): releases are checked by SHA-256 from the same release,
    not signed; consider signing (minisign or code signing on Windows and macOS) and
    check that a download cannot write or run anything outside its temporary folder;
  - one-line installs (`scripts/install.ps1`, `scripts/install.sh`): what they run, from
    where, with which rights; the uninstaller only removes its own files;
  - keys and tokens (device token, licence, any API key typed in Settings): never in
    logs, crash messages or exported files;
  - downloads of models and runtimes: pin a SHA-256 for every file, as the voice programs
    and the OCR model already are;
  - untrusted input: editais, past exams and photos of written answers can carry prompt
    injection; check that the model's answers cannot trigger file, network or process
    actions, and that links opened from the UI are only `https`;
  - child processes (Piper, sherpa-onnx, llama.cpp for OCR): fixed paths, no shell, arguments
    never built from user or model text;
  - local data: SQLite file and model folders in `%LOCALAPPDATA%` or the platform
    equivalent, with the user's permissions only;
  - dependencies: `cargo audit` and `cargo deny` (advisories, licenses, sources) in CI;
  - CI and releases: workflow token permissions, pinned actions, who can push tags.
