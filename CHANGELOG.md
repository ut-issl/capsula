# Changelog

Release notes for Capsula. Each release section becomes the body of its GitHub Release.
Add a `## v<version>` section before running the release workflow (see `RELEASING.md`).
Earlier releases are listed on [GitHub Releases](https://github.com/ut-issl/capsula/releases).

## v0.15.1

This patch release addresses a `rustls` security advisory and refreshes Capsula's development tooling.

### Security

- Update `rustls` to 0.23.45 to address [RUSTSEC-2026-0285](https://rustsec.org/advisories/RUSTSEC-2026-0285). (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1247>_)

### Development and CI

- Update the development and CI toolchain to Rust 1.98.1, including the Rust Docker image (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/1241>, <https://github.com/ut-issl/capsula/pull/1248>_)
- Optimize the development setup for Amp orbs (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1217>_)
- Update `zizmorcore/zizmor-action` to v0.6.4 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/1249>_)
- Update `taiki-e/install-action` from v2.87.4 through v2.87.10 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/1240>, <https://github.com/ut-issl/capsula/pull/1242>, <https://github.com/ut-issl/capsula/pull/1243>, <https://github.com/ut-issl/capsula/pull/1244>, <https://github.com/ut-issl/capsula/pull/1246>, <https://github.com/ut-issl/capsula/pull/1251>, <https://github.com/ut-issl/capsula/pull/1252>_)

### Internal Changes

- Refresh the Cargo lock file (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/1245>_)
- Bump the workspace version to v0.15.1 (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1250>_)

**Full Changelog**: <https://github.com/ut-issl/capsula/compare/v0.15.0...v0.15.1>
