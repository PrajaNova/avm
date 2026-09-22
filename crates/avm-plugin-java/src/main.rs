use avm_plugin_api::{runner, Manifest};
use avm_plugin_java::JavaProvider;
use std::process::ExitCode;

fn main() -> ExitCode {
    let manifest = Manifest {
        name: "java".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        api_version: Some(1),
        description: Some(
            "Built-in OpenJDK provider (Eclipse Temurin builds via the foojay Disco API)"
                .to_string(),
        ),
        section_label: Some("OpenJDK".to_string()),
        homepage: Some("https://github.com/prajanova/avm".to_string()),
    };
    runner::run(manifest, &JavaProvider::new())
}
