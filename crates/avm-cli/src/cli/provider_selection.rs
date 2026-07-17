fn select_tool_version(
    provider_name: &str,
    provider: &dyn ToolProvider,
    versions: Vec<avm_plugin_api::ToolVersion>,
) -> Result<()> {
    let items = versions
        .iter()
        .map(|version| ui::SelectItem {
            label: version.label.clone(),
        })
        .collect::<Vec<_>>();

    let title = format!("Available {provider_name} versions");
    match ui::select(&title, "Use Up/Down to move, Enter to select, q to cancel.", &items, 10)? {
        Some(selected) => {
            confirm_tool_version_selection(provider_name, provider, &versions[selected].version)
        }
        None => {
            println!("Cancelled.");
            Ok(())
        }
    }
}

fn confirm_tool_version_selection(
    provider_name: &str,
    provider: &dyn ToolProvider,
    version: &str,
) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let has_local_config = cwd.join(CONFIG_FILE).exists();

    if has_local_config {
        print!("Use {provider_name} {version} locally or globally? [l/g/c]: ");
        io::stdout().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        match answer.trim().to_ascii_lowercase().as_str() {
            "l" | "local" => use_provider_version(provider_name, provider, version, false),
            "g" | "global" => use_provider_version(provider_name, provider, version, true),
            _ => {
                println!("Cancelled.");
                Ok(())
            }
        }
    } else {
        print!("No local .avm.json found. Set {provider_name} {version} globally? [y/N]: ");
        io::stdout().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        match answer.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => use_provider_version(provider_name, provider, version, true),
            _ => {
                println!("Cancelled.");
                Ok(())
            }
        }
    }
}
