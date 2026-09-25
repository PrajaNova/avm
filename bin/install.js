#!/usr/bin/env node
// Post-install script: downloads the correct avm binary for the current platform

const fs = require('fs');
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

function main() {
  try {
    const { osName, archName } = getPlatform();
    const archiveName = `avm_${osName}_${archName}.tar.gz`;
    const url = `https://github.com/${REPO}/releases/download/${VERSION}/${archiveName}`;

    const binDir = __dirname;
    const finalBinary = path.join(binDir, 'avm-bin');

    console.log(`Downloading avm ${VERSION} for ${osName}/${archName}...`);
    execSync(`curl -fsSL "${url}" | tar -xz -C "${binDir}"`, { stdio: 'inherit' });
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
