# avm GitHub Action

Install [avm](https://github.com/PrajaNova/avm), install your project's tools, and put them on `PATH`.

```yaml
- uses: actions/checkout@v4
- uses: PrajaNova/avm/action@v1
  with:
    version: latest          # avm release tag
    install: true            # `avm install --frozen` if .avm.lock exists, else `avm install`
    plugins: ""              # extra plugins for `avm plugin add` (bundled ones need none)
    cache: true              # ~/.avm/tools + plugins, keyed by .avm.lock/.avm.json hash + OS + arch
    working-directory: .
    env: ci                  # sets AVM_ENV
- run: avm-bin run build
```

Sets `AVM_TRUST_ALL=1`, adds avm's shims to `PATH`, and exports the merged `.avm.json` env via `GITHUB_ENV`.
The command is `avm-bin` (`avm` is a shell function that CI steps do not load).
Linux and macOS only (avm has no Windows shell support yet).
The logic lives in [`scripts/ci-setup.sh`](../scripts/ci-setup.sh), which other CI systems use too.
