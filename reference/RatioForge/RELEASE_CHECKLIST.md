# Release Checklist

Use this checklist before creating and pushing a release tag.

## Versioning

- [ ] Decide the next SemVer version.
- [ ] Update `version.txt`.
- [ ] Update `Source/Directory.Build.props`.
- [ ] Update version assertions in tests.
- [ ] Update `CHANGELOG.md` with the release date and notable changes.
- [ ] Update `README.md` if the current release highlights changed.
- [ ] Do not update the archived `Website/` PHP pages; see `docs/decisions/0001-archive-legacy-website.md`.

## Local Verification

- [ ] Confirm the worktree is clean before starting release edits.
- [ ] Run `dotnet restore Source\RatioForge.sln`.
- [ ] Run `build\Test-NuGetSecurity.ps1`.
- [ ] Run `dotnet build Source\RatioForge.sln --configuration Release --no-restore`.
- [ ] Run `dotnet test Source\RatioForge.sln --configuration Release --no-build`.
- [ ] Publish the compressed self-contained executable with the command used by `.github/workflows/release.yml`.
- [ ] Publish the framework-dependent Lite executable with the command used by `.github/workflows/release.yml`.
- [ ] Confirm `artifacts\RatioForge-<version>-win-x64\RatioForge.exe` exists.
- [ ] Confirm both executable file versions match `<version>.0` and both pass the startup smoke check.
- [ ] Confirm the self-contained executable is at most 100 MiB and starts without a separately installed .NET runtime.
- [ ] Confirm the Lite executable is at most 10 MiB and starts with the .NET 8 Desktop Runtime installed.
- [ ] Generate and verify SHA256 checksums for the zip archive and both raw GUI executables.
- [ ] Remove local generated `artifacts\` output before committing.

## Windows Executable Build

- [ ] Build the Windows Forms executable with `dotnet publish`.
- [ ] Publish the compressed self-contained GUI executable as `RatioForge-<version>-win-x64.exe`.
- [ ] Publish the framework-dependent GUI executable as `RatioForge-<version>-win-x64-lite.exe`.
- [ ] Publish the self-contained single-file archive as `RatioForge-<version>-win-x64.zip`.
- [ ] Do not add packaging tools for languages that are not used by the application.

## Commit And Tag

- [ ] Verify Git identity is `tsautier <tsautier@users.noreply.github.com>`.
- [ ] Commit release changes with author `tsautier <tsautier@users.noreply.github.com>`.
- [ ] Create an annotated tag: `git tag -a v<version> -m "RatioForge <version>"`.
- [ ] Push `master`.
- [ ] Push the tag: `git push origin v<version>`.

## GitHub Actions Verification

- [ ] Confirm the Build workflow passes.
- [ ] Confirm the Release workflow passes.
- [ ] Confirm the release page exists.
- [ ] Confirm these release assets exist and are non-empty:
  - `RatioForge-<version>-win-x64.zip`
  - `RatioForge-<version>-win-x64.exe`
  - `RatioForge-<version>-win-x64-lite.exe`
  - `RatioForge-<version>-win-x64.sha256`
- [ ] Confirm the checksum file includes the zip archive and both raw GUI executables.
- [ ] Confirm no CLI executable is expected unless a dedicated .NET CLI project exists.

## Rollback

- [ ] If the tag was pushed but release validation failed, delete the GitHub release first.
- [ ] Delete the remote tag: `git push origin :refs/tags/v<version>`.
- [ ] Delete the local tag: `git tag -d v<version>`.
- [ ] Revert or fix the release commit on `master`.
- [ ] Re-run local verification before tagging again.
