# Changelog

All notable changes to the Know Giver white-label base. Format based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/).

Earlier history (Know Giver 0.1.0 to 0.39.3) lives in the upstream Know Giver
repository.

## [Unreleased]

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
