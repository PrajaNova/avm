# avm Runtime Flow

## Alias execution

```mermaid
flowchart TD
  A["User runs avm run <alias>"] --> B["Load local .avm.json"]
  B --> C["Load global ~/.avm.json"]
  C --> D["Load provider aliases"]
  D --> E["Resolve alias local first"]
  E --> F{"Alias found?"}
  F -->|Yes| G["Expand placeholders"]
  G --> H["Merge env and tool PATH"]
  H --> I["Execute command"]
  F -->|No| J["Return alias not found"]
```

## Plain binary execution through shims

```mermaid
flowchart TD
  A["User runs node"] --> B["Shell PATH finds ~/.avm/shims/node"]
  B --> C["Shim calls avm-bin exec-shim node"]
  C --> D["Load config and resolve tools.node"]
  D --> E{"Managed node installed?"}
  E -->|Yes| F["Execute ~/.avm/tools/node/<version>/bin/node"]
  E -->|No| G["Warn and search PATH excluding avm shims"]
  G --> H["Execute system node"]
```

## Node package script provider

```mermaid
flowchart TD
  A["avm loads providers"] --> B["Node provider checks package.json"]
  B --> C{"scripts exists?"}
  C -->|No| D["No node aliases"]
  C -->|Yes| E["Detect manager by lockfile"]
  E --> F["Expose scripts as plugin aliases"]
```

Manager detection order:

1. `bun.lockb` or `bun.lock`
2. `pnpm-lock.yaml`
3. `yarn.lock`
4. `npm run`

## Plugin version selection

```mermaid
flowchart TD
  A["User runs avm node versions"] --> B["CLI resolves plugin name node"]
  B --> C["Call ToolProvider.available_versions"]
  C --> D["Node plugin fetches and filters release index"]
  D --> E["CLI renders shared selector"]
  E --> F["User picks version"]
  F --> G["Write local or global .avm.json selection"]
```

## Release flow

```mermaid
flowchart TD
  A["Push tag vX.Y.Z"] --> B["Run CI"]
  B --> C["Build platform archives"]
  C --> D["Publish GitHub release"]
  D --> E["Publish npm package @prajanova/avm"]
  D --> F["Update Prajanova Homebrew tap"]
```
