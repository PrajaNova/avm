# avm — Any Version Manager

Manage Node.js, Java and Android SDK versions per project, with aliases and
environment variables. Built in Rust for macOS, Linux and Windows.

**[Website & documentation](https://getavm.dev/)** ·
**[npm package](https://www.npmjs.com/package/@prajanova/avm)**

## Install

```bash
npm install -g @prajanova/avm
```

Prefer Homebrew or a standalone installer? See [all installation methods](https://getavm.dev/docs/guide/getting-started).

Set up your shell using the [shell setup guide](https://getavm.dev/docs/guide/shell-setup).
For Bash or zsh, add this to `~/.bashrc` or `~/.zshrc`:

```bash
eval "$(avm-bin shell-init)"
```

## Quick start

```bash
avm plugin add node
avm node use 22
avm add dev "npm run dev"
avm dev
```

## Learn more

- [Getting started](https://getavm.dev/docs/guide/getting-started)
- [Commands & configuration](https://getavm.dev/docs)
- [Plugin marketplace](https://getavm.dev/marketplace)
- [Changelog](https://getavm.dev/changelog)

Contributions welcome: [Contributing](https://github.com/PrajaNova/avm/blob/main/CONTRIBUTING.md) ·
[Report an issue](https://github.com/PrajaNova/avm/issues) ·
[Security](https://github.com/PrajaNova/avm/blob/main/SECURITY.md).

[MIT License](https://github.com/PrajaNova/avm/blob/main/LICENSE)
