# Releases and Automatic Updates

The GitHub Actions workflow `.github/workflows/release.yml` publishes a release
on every push to `main` or `master`, and it can also be run manually.

The workflow uses `tauri-apps/tauri-action@v0.6.2`. This is intentional: the
current Tauri v2 documentation shows the action line as `@v0`, and the
`tauri-apps/tauri-action` repository publishes `v0`, `v0.6`, and `v0.6.2`
tags, not `v1`.

## Built platforms

- macOS Apple Silicon (`aarch64-apple-darwin`)
- Linux x64 (`ubuntu-22.04`)
- Linux arm64 (`ubuntu-22.04-arm`)
- Windows x64 (`windows-latest`)

macOS Intel is intentionally excluded.

## Required GitHub secrets

Generate a Tauri updater key pair:

```bash
deno task tauri signer generate -w ~/.tauri/echo.key
```

Then add these secrets in GitHub:

- `TAURI_SIGNING_PRIVATE_KEY`: the generated private key contents
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: the key password, or empty if none
- `TAURI_UPDATER_PUBLIC_KEY`: the public key printed by the command

The public key is injected into `tauri.conf.json` only during CI builds. The
private key signs updater bundles and must never be committed.

## Date-based versioning

To ensure a real new release on every push, CI computes one SemVer-compatible
UTC version at the start of the workflow, and every matrix build reuses that
same version:

```text
YYYY.M.DDHHMMSS
```

For example, a build created on June 13, 2026 at 09:45:30 UTC becomes
`2026.6.13094530`.

The Tauri updater compares these as SemVer versions. Without a version bump, an
installed app would not detect an update.

## Updater behavior

In release builds, the app checks this endpoint at startup:

```text
https://github.com/LeoMartinDev/echo/releases/latest/download/latest.json
```

If a newer version exists, it is downloaded, installed, and the app restarts
automatically.
