# Archive the Legacy Website

- Status: Accepted
- Date: 2026-08-11

## Context

The `Website/` directory contains the historical PHP website inherited from RatioMaster.NET. The current project is distributed through GitHub Releases, checks its version from the repository root `version.txt`, and documents usage in the root README. No deployment workflow or supported PHP runtime exists in this repository.

## Decision

The legacy PHP website is retained as a read-only historical archive and is not a maintained or deployed product surface. RatioForge will not migrate it to GitHub Pages at this time.

The canonical project surfaces are:

- `README.md` for product and usage documentation.
- `CHANGELOG.md` for release history.
- `version.txt` for the application update check.
- GitHub Releases for binaries and checksums.
- GitHub Issues and Discussions for support.

## Consequences

- Releases no longer update `Website/version.html` or the archived PHP pages.
- PHP is not required to build, test, package, or release RatioForge.
- Historical files and attribution remain available in `Website/`.
- A future public website requires a new decision, a supported static implementation, and an explicit deployment workflow.
