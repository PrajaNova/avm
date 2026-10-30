# Using avm in CI

Commit `.avm.json` and `.avm.lock` (see [Lockfiles](LOCKFILES.md)). `scripts/ci-setup.sh`
does the work on any CI: it installs avm, runs `avm plugin install` for each name in
`AVM_PLUGINS`, then installs tools with `--frozen` when a lock exists (plain
`avm -p install` otherwise). CI builds a checkout you chose, so it sets `AVM_TRUST_ALL=1`
instead of `avm trust`. `avm -p install` does not fetch plugins itself, so list one per
tool in `AVM_PLUGINS` (for example `node java`). Linux and macOS only. The command is `avm-bin`; `avm` is a shell function.

## GitHub Actions

```yaml
- uses: actions/checkout@v4
- uses: PrajaNova/avm/action@v1
  with:
    plugins: node       # one plugin per tool in .avm.json, e.g. "node java"
    cache: true         # tools + plugins keyed by lock/config hash
    env: ci             # AVM_ENV
- run: avm-bin run build
```

## GitLab CI

```yaml
build:
  variables: { AVM_TRUST_ALL: "1", AVM_ENV: ci, AVM_PLUGINS: node }
  cache:
    key: { files: [.avm.lock, .avm.json] }
    paths: [.avm-home/tools, .avm-home/plugins]
  before_script:
    - export AVM_HOME="$CI_PROJECT_DIR/.avm-home"
    - curl -fsSL https://raw.githubusercontent.com/PrajaNova/avm/main/scripts/ci-setup.sh | bash
    - export PATH="$HOME/.local/bin:$AVM_HOME/shims:$PATH"
    - eval "$(avm-bin env)"
  script: [avm-bin run build]
```

## CircleCI

```yaml
steps:
  - checkout
  - restore_cache: { keys: ["avm-{{ arch }}-{{ checksum \".avm.lock\" }}-{{ checksum \".avm.json\" }}"] }
  - run:
      name: Install avm and tools
      command: |
        curl -fsSL https://raw.githubusercontent.com/PrajaNova/avm/main/scripts/ci-setup.sh | AVM_TRUST_ALL=1 AVM_ENV=ci AVM_PLUGINS=node bash
        echo 'export AVM_TRUST_ALL=1 AVM_ENV=ci PATH="$HOME/.local/bin:$HOME/.avm/shims:$PATH"; eval "$(avm-bin env)"' >> "$BASH_ENV"
  - save_cache: { key: "avm-{{ arch }}-{{ checksum \".avm.lock\" }}-{{ checksum \".avm.json\" }}", paths: [~/.avm/tools, ~/.avm/plugins] }
  - run: avm-bin run build
```
