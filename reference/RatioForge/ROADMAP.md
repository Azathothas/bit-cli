# Roadmap

This roadmap is intentionally practical: it separates near-term maintenance from larger product and architecture work.

## Short Term - Done

Completed on 2026-08-11.

- [x] **Done - Release automation:** Build and tagged-release workflows run tests, create self-contained Windows artifacts, launch the executable, generate SHA256 checksums, upload artifacts, and re-download release assets for checksum verification. See [build.yml](.github/workflows/build.yml) and [release.yml](.github/workflows/release.yml).
- [x] **Done - Dependency maintenance:** Direct NuGet dependencies are current, Dependabot checks NuGet and GitHub Actions weekly, and CI rejects known direct or transitive NuGet vulnerabilities. See [dependabot.yml](.github/dependabot.yml) and [Test-NuGetSecurity.ps1](build/Test-NuGetSecurity.ps1).
- [x] **Done - Parser and URL edge cases:** Automated fixtures cover single-file, multi-file, missing metadata, damaged piece hashes, truncated strings, and unterminated integers; tracker tests cover announce events, scrape rewriting, query preservation, and malformed hashes. See [RatioForge.Tests](Source/RatioForge.Tests).
- [x] **Done - Reproducible releases:** Versioning, local verification, packaging, tagging, GitHub asset checks, and rollback are documented in [RELEASE_CHECKLIST.md](RELEASE_CHECKLIST.md).
- [x] **Done - Legacy website decision:** The PHP website is retained as an undeployed historical archive; README, changelog, `version.txt`, and GitHub Releases are canonical. See [ADR 0001](docs/decisions/0001-archive-legacy-website.md).

## Mid Term

- Add a dedicated .NET CLI project if command-line automation is needed.
- Add structured logging for tracker communication, version checks, and proxy failures.
- Improve error handling around network, proxy, and malformed torrent files.
- Completed early: sample torrent fixtures now cover parser and tracker behavior; continue extending them when regressions are found.
- Active: CI enforces startup checks and size budgets for the compressed self-contained and Lite single-file releases.
- Add a signed release path if code-signing certificates become available.

## Long Term

- Move the extracted tracker URL builder and torrent parser into a dedicated reusable core library.
- Consider a modern UI refresh while preserving the existing lightweight workflow.
- Add a documented plugin or profile system for torrent client emulation data.
- Build a dedicated .NET CLI executable if command-line workflows become part of the product.
- Add automated compatibility testing across supported Windows runner images.
- Define a security and disclosure policy for tracker, proxy, and release-distribution issues.
