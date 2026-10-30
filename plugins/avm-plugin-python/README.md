# avm-plugin-python

avm's native CPython provider, built on
[python-build-standalone](https://github.com/astral-sh/python-build-standalone)
(relocatable, SHA-256-verified builds; the same source `uv` uses).

```bash
avm plugin add python
avm python install latest
```

## Notes

- `avm python versions` lists every GA CPython that python-build-standalone
  has a relocatable build for (`avm python 13 versions` filters to 3.13). The
  index is Astral's [`astral-sh/versions`](https://github.com/astral-sh/versions)
  feed (~20 MB), cached in `~/.avm/tools/python/.index.ndjson` for 6 hours.
- Installs use the `install_only_stripped` archive (or `install_only` for older
  releases) for your platform (macOS, Linux glibc, Windows; x64 and arm64) and
  are checked against the SHA-256 in the index. Supports `avm plugin lock`.
- Set `AVM_PYTHON_INDEX` to a URL or file path of an index in the same format
  to use a mirror (skips the cache).

## Development

```bash
cargo build --package avm-plugin-python
cargo test --package avm-plugin-python
```
