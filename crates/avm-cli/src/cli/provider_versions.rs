fn print_provider_status(
    provider_name: &str,
    provider: &dyn ToolProvider,
    cfg: &ResolvedConfig,
) -> Result<()> {
    println!("Plugin: {provider_name}");
    if let Some((version, source)) = cfg.resolve_tool(provider_name, cfg) {
        println!("Selected version: {version} ({})", alias_source_label(&source));
    } else {
        println!("Selected version: none");
    }
    print_installed_versions(provider)?;
    println!();
    println!("Commands:");
    println!("  avm {provider_name} versions");
    println!("  avm {provider_name} use <version>");
    println!("  avm {provider_name} install <version>");
    println!("  avm {provider_name} uninstall <version>");
    Ok(())
}

fn print_installed_versions(provider: &dyn ToolProvider) -> Result<()> {
    let installed = provider.installed_versions()?;
    if installed.is_empty() {
        println!("Installed {} versions: none", provider.name());
    } else {
        println!(
            "Installed {} versions: {}",
            provider.name(),
            installed.join(", ")
        );
    }
    Ok(())
}

fn print_available_versions(
    provider_name: &str,
    provider: &dyn ToolProvider,
    filter: VersionFilter,
) -> Result<()> {
    let versions = provider.available_versions(provider_query(filter))?;
    if versions.is_empty() {
        println!("Available {provider_name} versions: none");
        return Ok(());
    }

    if ui::can_select() {
        return select_tool_version(provider_name, provider, versions);
    }

    println!("Available {provider_name} versions:");
    for version in &versions {
        println!("  {}", version.label);
    }

    println!();
    println!("Use:");
    println!("  avm {provider_name} install <version>");
    println!("  avm {provider_name} use <version>");
    Ok(())
}
