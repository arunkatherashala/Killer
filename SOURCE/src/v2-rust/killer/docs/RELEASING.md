# Releasing Killer

The release workflow is `.github/workflows/release.yml`. It builds `killer` (the driver) and
`killer_super` for linux-x64, windows-x64, macos-x64 and macos-arm64, packages each as
`killer_super-<tag>-<target>.tar.gz` (`.zip` on Windows) with a `.sha256` file, and a final job
publishes a GitHub release with all archives plus a combined `SHA256SUMS`.

## Version rules

- The tag must be `v` plus the `version` in `SOURCE/src/v2-rust/killer/Cargo.toml` (now `1.0.0`,
  so tag `v1.0.0`). A suffix such as `v1.0.0-rc1` is allowed and makes the release a pre-release.
  The workflow fails early if the tag and Cargo.toml disagree.
- `killer --version` prints `killer <Cargo.toml version>`; the smoke test checks this.
- `killer_super --version` reports its own compiler version (4.0.0, covered by a test); it is
  intentionally independent of the crate version.

## Steps

1. Bump `version` in Cargo.toml if needed and commit it on the branch you release from.
2. Run the CI checks locally (`cargo test --tests --no-fail-fast`, parity, docs checks).
3. Dry run (publishes nothing): GitHub > Actions > release > Run workflow, keep the default
   version label. All four targets build, the smoke test runs, and the archives appear as
   workflow artifacts. Download one and run `sha256sum -c` on it.
4. Owner step, pushing the tag (this publishes a public release):

       git tag v1.0.0
       git push origin v1.0.0

5. Watch the run. Then test the installers against the real release:

       curl -fsSL https://raw.githubusercontent.com/arunkatherashala/Killer/main/install.sh | sh -s -- --version v1.0.0
       irm https://raw.githubusercontent.com/arunkatherashala/Killer/main/install.ps1 | iex

## Testing an installer without a release

Serve a directory holding the archive and `.sha256` (for example `python -m http.server`) and
point the installer at it: `KILLER_BASE_URL=http://127.0.0.1:8000 sh install.sh --version v1.0.0
--prefix /tmp/killer-test`. `KILLER_OS` and `KILLER_ARCH` override platform detection in
`install.sh` for cross-checking.

## Not verified until a real tag run

- The GitHub-hosted runners, the macOS builds (macos-x64 is cross-compiled and never executed),
  and the upload/download-artifact and release publishing steps.
- `install.ps1` and the Windows zip packaging step were not executed locally.

## Dry run by branch push

GitHub only offers "Run workflow" for a workflow that exists on the default branch. Until `release.yml` is on
`main`, run the dry run by pushing the commit to a branch named `dryrun-v<version>`:

```text
git push origin HEAD:refs/heads/dryrun-v1.0.0-beta
```

The workflow builds every target, runs the smoke test and uploads the archives as workflow artifacts, labelled with
the branch name. The publish job only runs for a pushed tag, so this never creates a release. Delete the branch
afterwards: `git push origin --delete dryrun-v1.0.0-beta`.

## Release notes

The text of the GitHub release comes from `docs/RELEASE_NOTES.md` (the publish job checks it out and passes it as
`body_path`; GitHub appends its generated change list). Update that file before tagging: it carries the security
warning about the old fake `encryption` module, the behaviour changes and the "Not tested" list.
