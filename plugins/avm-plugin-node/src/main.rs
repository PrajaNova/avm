use avm_plugin_node::NodeProvider;
use avm_plugin_sdk::{runner, Manifest};
use std::process::ExitCode;

/// Plugin entry point: runs the Node provider through the plugin protocol.
fn main() -> ExitCode {
    let manifest = Manifest::new(
        "node",
        env!("CARGO_PKG_VERSION"),
        "Built-in Node.js provider for node version resolution",
        "Node",
    );
    runner::run(manifest, &NodeProvider)
}
