# avm Usage Skill

Use this as a compact skill file for agents helping users with avm.

## Primary workflow

```bash
avm init
avm add dev "pnpm run dev"
avm node use 20.11.1
avm shims install
eval "$(avm shell-init)"
```

## Commands to explain

- `avm init` creates local `.avm.json`.
- `avm add <name> <command>` adds a local alias.
- `avm add --global <name> <command>` adds a global alias.
- `avm run <name>` runs an alias.
- `avm which <name>` explains where a value came from.
- `avm list` shows aliases, env, tools, and provider aliases.
- `avm node use <version>` writes the selected Node version.
- `avm shims install` creates plain command shims.
- `avm shell-init` prints shell integration.

## Expected behavior

- Local config wins over global config.
- Node package scripts are exposed from `package.json`.
- Lockfile order is Bun, pnpm, Yarn, then npm.
- Missing managed Node versions fall back to system Node with a warning.
- `tool install` is a reserved provider surface; Node download is not implemented in the baseline.

## Good support answer shape

1. Identify whether the user is asking about aliases, tools, env, shims, or plugins.
2. Inspect `.avm.json`.
3. Explain local/global precedence.
4. Give exact commands to fix the issue.
