use avm_plugin_api::{runner, Manifest};
use avm_plugin_node::NodeProvider;
use std::process::ExitCode;

fn main() -> ExitCode {
    let manifest = Manifest {
        name: "node".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        api_version: Some(1),
        description: Some("Built-in Node.js provider for node version resolution".to_string()),
        section_label: Some("Node".to_string()),
        homepage: Some("https://github.com/prajanova/avm".to_string()),
    };
    runner::run(manifest, &NodeProvider::new())
}
