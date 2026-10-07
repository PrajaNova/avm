use avm_plugin_api::{runner, Manifest};
use avm_plugin_node::NodeProvider;
use std::process::ExitCode;

fn main() -> ExitCode {
    let manifest = Manifest::new(
        "node",
        env!("CARGO_PKG_VERSION"),
        "Built-in Node.js provider for node version resolution",
        "Node",
    );
    runner::run(manifest, &NodeProvider)
}
