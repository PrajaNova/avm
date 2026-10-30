### Development Instructions

- Do `@.claude/skills/unsafe-rust-review` for all the code review
- Do Write clean and smaller code, with SRP, ATOMIX principals
- Code muts use block comment for meothos doc doc and line comment for any spefic details for inside fucntion
- Do not write repetated code
- All Code must have better error handling and cleand docs
- CLI layout (`crates/avm-cli/src/`, no `cli/` folder): one folder per feature (`alias`, `env`, `project`, `tool`, `global`, `app`), plus `common/` for code every feature shares. A feature that already has a domain module merges into it (`plugins/`, `shims/`).
  - Name files by role: `<feature>_manager.rs` (clap commands + `Manager` impl), `<feature>_handler.rs` or `<what>_handler.rs` (the `cmd_*` functions), `<feature>_utils.rs` (small helpers), `<name>_service.rs` (domain logic with no CLI types).
  - A feature's `mod.rs` re-exports its manager and handlers; other features import from the feature module, never from a sibling's private file.
  - Adding a feature: new folder, a `*Manager` enum, one variant in `Commands` (`args.rs`) and one arm in `run/dispatch.rs`.
