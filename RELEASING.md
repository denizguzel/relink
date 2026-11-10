# Releasing Re-Link

Releases are GitHub Releases, built by GitHub Actions
(`.github/workflows/release.yml`) when a version tag is pushed. Nothing is
published on an ordinary commit: `ci.yml` only checks (format, clippy, tests,
a Windows build).

## When to release

Re-Link follows [Semantic Versioning](https://semver.org). While it is
`0.x`, a minor bump (`0.3.0`) is for new features or changed behaviour and a
patch bump (`0.2.1`) for fixes only. Release when there is something users
should get: a fix they are waiting for, or a set of finished features. Not every
merge. Use a pre-release tag (`0.3.0-rc.1`) to try the files on real machines
first; pre-releases are never offered by the in-app updater.

## How

```sh
scripts/release.sh 0.3.0     # checks, bumps Cargo.toml/Cargo.lock, commits, tags
git show v0.3.0              # look at it
git push origin master v0.3.0
```

The tag starts the workflow, which refuses a tag that differs from the
`Cargo.toml` version, then builds, tests and publishes the release with
automatically generated notes (grouped by pull request label, see
`.github/release.yml`). A tag whose version has a `-` suffix becomes a
pre-release. To rehearse without publishing, run the workflow by hand from
the Actions tab ("Run workflow"): it builds everything and attaches the files
to the run.

## What a release contains

| File | For |
| --- | --- |
| `relink-<v>-windows-x64-setup.exe` | Windows installer. Per user (`%LOCALAPPDATA%\Programs\Re-Link`), no administrator rights, Start menu entry, uninstaller. |
| `relink-<v>-windows-x64-portable.zip` | Windows, no install: unzip anywhere. The `relink.json` inside keeps all settings next to the program. |
| `relink-<v>-linux-x86_64.tar.gz` | Linux, portable: the program, the root helper and `install-linux.sh`. Needs glibc 2.31 or newer (Ubuntu 20.04 and newer; built with `cargo-zigbuild` against glibc 2.31). |
| `relink-<v>-windows-x64.exe`, `relink-<v>-linux-x86_64` | The bare programs the in-app updater downloads. |
| `SHA256SUMS` | Checksums of every file above. |

Every file also has a build provenance attestation:
`gh attestation verify <file> --repo denizguzel/relink`.

## How installed copies update

Re-Link asks the GitHub API for the latest release at start-up (after 30
seconds) and every 12 hours (setting "Check for updates"). A newer, non-draft,
non-pre-release version shows a banner; **Update and restart** downloads the
bare program for the platform, checks it against `SHA256SUMS`, renames the running
file to `.old`, puts the new one in its place and starts it. The new process waits for
the old one to exit and then deletes the `.old` file. If the folder is not writable or
a step fails, the banner offers the release page instead.

- The updater only fetches files of this repository's releases over HTTPS.
- The Linux root helper is never replaced by the updater (it is root-owned on
  purpose). When a release changes it, the notes say so: rerun
  `sudo ./install-linux.sh` from the new archive.

## Troubleshooting a release

- *Tag does not match Cargo.toml*: delete the tag (`git push --delete origin vX`,
  `git tag -d vX`), fix, tag again. Never move a published tag.
- *The release exists but a file is wrong*: publish a new patch version. The
  updater and users trust the published files and their checksums.
