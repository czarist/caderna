# White-label implementation contract

Requested on 2026-11-08. Brand direction approved: Caderna, paper, graphite and olive;
an original page symbol. Client serves students; Trentin licenses the software.

## Verified UI vocabulary

`crates/ui/src/theme.rs`: Palette, LIGHT, DARK, Inter and IBM Plex Mono; background
#F6F4EF, surface #FFFEFB, text #2B2A28. `widgets.rs`: header, hint, primary_button,
callout, link. `screens/settings.rs`: about_section and saved settings draft.
`screens/demo.rs`: expired and badge. `kg-brand`: NAME, COMPANY, asset bytes and
accent/highlight constants. The new license activation surface is explicitly in scope.

## Review artifact

Open `caderno-flow.svg`: existing geometry and typography with the proposed brand,
support contact, active license, trial, invalid activation and expired states.
This is a prototype, not a screenshot of implemented behavior.

## Implementation and acceptance

- Editable TOML brand, SVG masters, Markdown legal bundle, per-brand consent version.
- No private signing key in brands, app, repository or CI artifacts. Ed25519 verifies
  exact payload bytes; licenses bind the brand, customer, seats and validity.
- Local activation is transactional. Invalid, expired or wrong-brand licenses never
  replace an accepted license or remove study data. Offline seats are contractual,
  not a claim of enforcement across machines. Online checks remain opt-in and require
  a real client service contract before implementation.
- Five UI locales and legal document languages; region selection stays unchanged.
- Builds use every core by default; limit a single run with `-j N` when needed.
- Client legal review, third-party commercial review and signing credentials are
  release prerequisites, never assertions inferred from a passing build.
- Verify default/demo/neutral builds, key and prompt parity, license tampering,
  expiry, brand mismatch, demo conversion and all five-language UI captures.
