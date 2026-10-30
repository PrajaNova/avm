//! Fake protocol plugins installed into a sandbox's `home/.avm/plugins`.
use crate::common::Sandbox;
use fs_err as fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

/// Writes `script` as the executable `avm-plugin-<tool>/bin/avm-plugin`; returns its path.
pub fn install_plugin(sandbox: &Sandbox, tool: &str, script: &str) -> PathBuf {
    let bin = sandbox.path(&format!("home/.avm/plugins/avm-plugin-{tool}/bin"));
    fs::create_dir_all(&bin).unwrap();
    let file = bin.join("avm-plugin");
    fs::write(&file, script).unwrap();
    fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
    file
}

/// A plugin that records each `env-vars` call in `<root>/spawns` and reports `<TOOL>_HOME`.
/// Pins every tool in `tools` to `1` in the project config. Returns the spawn log path.
pub fn counting_env_plugins(sandbox: &Sandbox, tools: &[&str]) -> PathBuf {
    let counter = sandbox.path("spawns");
    for tool in tools {
        let script = format!(
            "#!/bin/sh\nif [ \"$1\" = env-vars ]; then echo x >> '{}'; echo '{{\"env\":{{\"{}_HOME\":\"/opt/{tool}\"}}}}'; fi\n",
            counter.display(),
            tool.to_uppercase()
        );
        install_plugin(sandbox, tool, &script);
    }
    let pins: serde_json::Map<_, _> = tools
        .iter()
        .map(|t| (t.to_string(), serde_json::json!("1")))
        .collect();
    fs::write(
        sandbox.path("project/.avm.json"),
        serde_json::json!({ "tools": pins }).to_string(),
    )
    .unwrap();
    counter
}
