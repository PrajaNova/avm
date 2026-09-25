#!/usr/bin/env node
// Post-install script: downloads the correct avm binary for the current platform

const crypto = require('crypto');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { execSync } = require('child_process');

const pkg = require('../package.json');
const REPO = 'prajanova/avm';
const VERSION = `v${pkg.version}`;

function getPlatform() {
  const platform = process.platform;
  const arch = process.arch;

  let osName, archName;

  if (platform === 'darwin') osName = 'darwin';
  else if (platform === 'linux') osName = 'linux';
  else throw new Error(`Unsupported OS: ${platform}`);

  if (arch === 'x64') archName = 'amd64';
  else if (arch === 'arm64') archName = 'arm64';
  else throw new Error(`Unsupported architecture: ${arch}`);

  // macOS ships Apple Silicon builds only; Intel Macs are no longer supported.
  if (osName === 'darwin' && archName === 'amd64') {
    throw new Error('Intel macOS is not supported; avm provides Apple Silicon (arm64) macOS builds only');
  }

  return { osName, archName };
}

// Check the archive against the release's checksums.txt before extracting.
function verify(archive, archiveName, base) {
  let sums;
  try {
    sums = execSync(`curl -fsSL "${base}/checksums.txt"`, { stdio: ["ignore", "pipe", "ignore"] }).toString();
  } catch {
    if (process.env.AVM_ALLOW_UNVERIFIED === '1') {
      console.warn('warning: release has no checksums.txt; installing UNVERIFIED (AVM_ALLOW_UNVERIFIED=1)');
      return;
    }
    throw new Error(`release has no checksums.txt, so ${archiveName} can't be verified (set AVM_ALLOW_UNVERIFIED=1 to install anyway)`);
  }
  const line = sums.split('\n').map((l) => l.trim().split(/\s+/)).find(([, f]) => f && f.replace(/^\*/, '') === archiveName);
  const actual = crypto.createHash('sha256').update(fs.readFileSync(archive)).digest('hex');
  if (!line || line[0].toLowerCase() !== actual) {
    throw new Error(`checksum mismatch for ${archiveName}\n  expected ${line ? line[0] : '<not listed>'}\n  got      ${actual}\n  refusing to install`);
  }
}

function main() {
  try {
    const { osName, archName } = getPlatform();
    const archiveName = `avm_${osName}_${archName}.tar.gz`;
    const base = `https://github.com/${REPO}/releases/download/${VERSION}`;

    const binDir = __dirname;
    const finalBinary = path.join(binDir, 'avm-bin');

    console.log(`Downloading avm ${VERSION} for ${osName}/${archName}...`);
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'avm-'));
    const archive = path.join(tmp, archiveName);
    try {
      execSync(`curl -fsSL "${base}/${archiveName}" -o "${archive}"`, { stdio: 'inherit' });
      verify(archive, archiveName, base);
      execSync(`tar -xzf "${archive}" -C "${binDir}"`, { stdio: 'inherit' });
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
    if (!fs.existsSync(finalBinary)) {
      throw new Error('release archive did not contain avm-bin');
    }
    fs.chmodSync(finalBinary, 0o755);

    console.log('✓ avm installed successfully');
    console.log('');
    console.log('To enable avm in your shell, add this to ~/.zshrc or ~/.bashrc:');
    console.log('  eval "$(avm-bin shell-init)"');
    console.log('');
    console.log('Then reload: source ~/.zshrc  # or source ~/.bashrc');
  } catch (err) {
    console.error('avm install failed:', err.message);
    console.error('You can install manually from: https://github.com/prajanova/avm/releases');
    process.exit(1);
  }
}

main();
