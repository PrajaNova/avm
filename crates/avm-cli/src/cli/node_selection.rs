fn select_available_node_version(versions: Vec<NodeVersion>) -> Result<()> {
    let items = versions
        .iter()
        .map(|version| ui::SelectItem {
            label: format_node_version(version),
        })
        .collect::<Vec<_>>();

    match ui::select(
        "Available node versions",
        "Use Up/Down to move, Enter to select, q to cancel.",
        &items,
        10,
    )? {
        Some(selected) => {
            let version = versions[selected].version.trim_start_matches('v').to_string();
            confirm_node_version_selection(&version)
        }
        None => {
            println!("Cancelled.");
            Ok(())
        }
    }
}

fn confirm_node_version_selection(version: &str) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let has_local_config = cwd.join(CONFIG_FILE).exists();

    if has_local_config {
        print!("Use node {version} locally or globally? [l/g/c]: ");
        io::stdout().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        match answer.trim().to_ascii_lowercase().as_str() {
            "l" | "local" => set_tool_version("node", version, false),
            "g" | "global" => set_tool_version("node", version, true),
            _ => {
                println!("Cancelled.");
                Ok(())
            }
        }
    } else {
        print!("No local .avm.json found. Set node {version} globally? [y/N]: ");
        io::stdout().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        match answer.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => set_tool_version("node", version, true),
            _ => {
                println!("Cancelled.");
                Ok(())
            }
        }
    }
}
