# Security Policy

## Supported Versions

Only the latest version of **avm** is currently supported for security updates.

| Version | Supported          |
| ------- | ------------------ |
| v0.2.x  | :white_check_mark: |
| < v0.2  | :x:                |

## Download verification

- `avm plugin add` / `update` verify each plugin archive's sha256 against
  the release's `checksums.txt` before extracting; a mismatch aborts with
  nothing installed.
- First-party plugins verify the runtimes they download: Node.js against
  `SHASUMS256.txt`, Temurin JDKs against foojay's sha256, and the Android
  cmdline-tools zip against a pinned sha256 (`sdkmanager` verifies the rest).
- `install.sh` and the npm installer verify `avm-bin` the same way.
- Releases carry GitHub build provenance attestations
  (`gh attestation verify <archive> -R <owner>/<repo>`).
- `AVM_ALLOW_UNVERIFIED=1` is the only way to skip verification.

## Reporting a Vulnerability

If you discover a potential security vulnerability in **avm**, please do not open a public issue. Instead, please report it privately by emailing mahendra.connect@outlook.com or using GitHub's private vulnerability reporting feature.

We will acknowledge your report within 48 hours and provide a timeline for a fix if necessary.
