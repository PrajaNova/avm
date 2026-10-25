use super::*;

/// `.avm.json`'s directory: `$HOME` for global, else the current directory.
pub fn config_root(global: bool) -> Result<PathBuf> {
    if global {
        home_dir()
    } else {
        std::env::current_dir().context("failed to read current directory")
    }
}

/// Load the local/global config, apply `f`, save it back. A missing file
/// starts empty when `create`, otherwise it's an error.
pub fn edit_config(
    global: bool,
    create: bool,
    f: impl FnOnce(&mut ConfigLoadResult) -> Result<()>,
) -> Result<()> {
    let root = config_root(global)?;
    let _guard = avm_plugin_api::storage::lock(&root.join(CONFIG_FILE))?;
    if !create && !root.join(CONFIG_FILE).exists() {
        return Err(if global {
            anyhow!("no {CONFIG_FILE} found in {}", root.display())
        } else {
            anyhow!("no {CONFIG_FILE} found in current directory. Run `avm init` first")
        });
    }
    // Editing through avm keeps an already-trusted (or brand new) local file
    // trusted; it never promotes an untrusted one.
    let file = root.join(CONFIG_FILE);
    let keep_trusted = !global && (!file.exists() || crate::trust::is_trusted(&file, &[]));
    let mut cfg = config::load(&root)?;
    f(&mut cfg)?;
    config::save(&root, &cfg)?;
    if keep_trusted {
        crate::trust::trust(&file)?;
    }
    Ok(())
}

fn scope(global: bool) -> &'static str {
    if global {
        "global"
    } else {
        "local"
    }
}

/// Print local ∪ global entries sorted by key, local winning.
fn print_layered(
    local: &HashMap<String, String>,
    global: &HashMap<String, String>,
    indent: &str,
    sep: &str,
    origins: &HashMap<String, String>,
) {
    let keys: BTreeSet<&String> = local.keys().chain(global.keys()).collect();
    for key in keys {
        let value = local
            .get(key)
            .or_else(|| global.get(key))
            .map(String::as_str)
            .unwrap_or("");
        let origin = origins.get(key).map(String::as_str).unwrap_or("unknown");
        let override_label = if local.contains_key(key) && global.contains_key(key) {
            " [override global]"
        } else {
            ""
        };
        println!("{indent}{key}{sep}{value} (from {origin}){override_label}");
    }
}

pub fn cmd_init() -> Result<()> {
    let root = std::env::current_dir().context("failed to read current directory")?;
    let _guard = avm_plugin_api::storage::lock(&root.join(CONFIG_FILE))?;
    if root.join(CONFIG_FILE).exists() {
        return Err(anyhow!("{CONFIG_FILE} already exists"));
    }
    let ignore = root.join(".gitignore");
    let mut raw = match fs::read_to_string(&ignore) {
        Ok(raw) => raw,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).context("read .gitignore"),
    };
    if !raw
        .lines()
        .any(|line| matches!(line.trim(), ".avm.local.json" | "/.avm.local.json"))
    {
        if !raw.is_empty() && !raw.ends_with('\n') {
            raw.push('\n');
        }
        raw.push_str(".avm.local.json\n");
        avm_plugin_api::storage::atomic_write(&ignore, raw.as_bytes())
            .context("update .gitignore")?;
    }
    config::save(&root, &ConfigLoadResult::default())?;
    crate::trust::trust(&root.join(CONFIG_FILE))?;
    println!("✓ Created {CONFIG_FILE} in current directory");
    Ok(())
}

pub fn cmd_add(args: AddArgs) -> Result<()> {
    edit_config(args.global, args.global, |cfg| {
        cfg.aliases.insert(args.key.clone(), args.value.join(" "));
        Ok(())
    })?;
    println!("✓ Added {} alias '{}'", scope(args.global), args.key);
    Ok(())
}

pub fn cmd_remove(args: RemoveArgs) -> Result<()> {
    edit_config(args.global, false, |cfg| {
        match cfg.aliases.remove(&args.key) {
            Some(_) => Ok(()),
            None => Err(anyhow!("alias '{}' not found", args.key)),
        }
    })?;
    println!("✓ Removed {} alias '{}'", scope(args.global), args.key);
    Ok(())
}

/// `avm alias add/remove/list` — same shape as `avm plugin`. `add`/`remove`
/// reuse the exact same handlers as the top-level `avm add`/`avm remove`.
pub fn cmd_alias(command: AliasCommands) -> Result<()> {
    match command {
        AliasCommands::Add(args) => cmd_add(args),
        AliasCommands::Remove(args) => cmd_remove(args),
        AliasCommands::List => cmd_alias_list(),
    }
}

fn cmd_alias_list() -> Result<()> {
    let cfg = load_state()?;
    if cfg.local_aliases.is_empty() && cfg.global_aliases.is_empty() {
        println!("No aliases configured.");
        return Ok(());
    }
    print_layered(
        &cfg.local_aliases,
        &cfg.global_aliases,
        "",
        " → ",
        &cfg.alias_origins,
    );
    Ok(())
}

/// `avm env add/remove/list` — same shape as `avm plugin`, targeting the
/// custom `env` map in `.avm.json` rather than `aliases`.
pub fn cmd_env_add(key: String, value: String, global: bool) -> Result<()> {
    edit_config(global, global, |cfg| {
        cfg.env.insert(key.clone(), value.clone());
        Ok(())
    })?;
    println!("✓ Added {} env var '{key}={value}'", scope(global));
    Ok(())
}

pub fn cmd_env_remove(key: String, global: bool) -> Result<()> {
    edit_config(global, false, |cfg| match cfg.env.remove(&key) {
        Some(_) => Ok(()),
        None => Err(anyhow!("env var '{key}' not found")),
    })?;
    println!("✓ Removed {} env var '{key}'", scope(global));
    Ok(())
}

pub fn cmd_env_list() -> Result<()> {
    let cfg = load_state()?;
    let merged = merge_env(&cfg);
    if merged.is_empty() {
        println!("No env vars configured.");
        return Ok(());
    }
    let mut keys: Vec<_> = merged.keys().collect();
    keys.sort();
    for key in keys {
        println!(
            "{key}={} (from {})",
            merged[key],
            cfg.env_origins
                .get(key)
                .map(String::as_str)
                .unwrap_or("unknown")
        );
    }
    Ok(())
}

pub fn cmd_trust(args: TrustArgs) -> Result<()> {
    if args.list {
        for path in crate::trust::list().keys() {
            println!("{path}");
        }
        return Ok(());
    }
    let target = match args.path {
        Some(path) => path,
        None => std::env::current_dir().context("failed to read current directory")?,
    };
    let files: Vec<PathBuf> = if target.is_dir() {
        let names = config::file_names(&std::env::var("AVM_ENV").unwrap_or_default())?;
        names
            .iter()
            .map(|f| target.join(f))
            .chain([target.join(".env")])
            .filter(|f| f.exists())
            .collect()
    } else {
        vec![target.clone()]
    };
    if files.is_empty() {
        return Err(anyhow!(
            "no active avm config or .env in {}",
            target.display()
        ));
    }
    for file in files {
        if args.revoke {
            let removed = crate::trust::revoke(&file)?;
            println!(
                "{} {}",
                if removed {
                    "✓ Revoked trust for"
                } else {
                    "Was not trusted:"
                },
                file.display()
            );
            continue;
        }
        // Show what's being trusted: the commands and env it will run with.
        if file
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with(".avm.") && n.ends_with(".json"))
        {
            let cfg = config::load_file(&file)?;
            for (key, value) in cfg.aliases.iter().collect::<BTreeMap<_, _>>() {
                println!("  alias {key} → {value}");
            }
            for (key, value) in cfg.env.iter().collect::<BTreeMap<_, _>>() {
                println!("  env   {key}={value}");
            }
        } else {
            for line in fs::read_to_string(&file)?
                .lines()
                .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
            {
                println!("  {line}");
            }
        }
        crate::trust::trust(&file)?;
        println!("✓ Trusted {}", file.display());
    }
    Ok(())
}

/// `~/x`, `./x` or `../x` instead of an absolute path (a file directly in home is `~/x`).
fn short_path(path: &str) -> String {
    let path = Path::new(path);
    let home = home_dir().ok();
    let under_home = home.as_deref().and_then(|h| path.strip_prefix(h).ok());
    if let Some(rest) = under_home.filter(|r| r.components().count() == 1) {
        return format!("~/{}", rest.display());
    }
    let cwd = std::env::current_dir().ok();
    for (depth, dir) in cwd.iter().flat_map(|c| c.ancestors()).enumerate() {
        if let Ok(rest) = path.strip_prefix(dir) {
            let up = if depth == 0 {
                "./".to_string()
            } else {
                "../".repeat(depth)
            };
            return format!("{up}{}", rest.display());
        }
    }
    match under_home {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// Entries under "Global" / "Local" headings, sorted; local entries name their file.
fn print_scoped(
    title: &str,
    local: &HashMap<String, String>,
    global: &HashMap<String, String>,
    sep: &str,
    origins: &HashMap<String, String>,
) {
    if local.is_empty() && global.is_empty() {
        return;
    }
    println!("{title}:");
    if !global.is_empty() {
        println!("  Global:");
        for (key, value) in global.iter().collect::<BTreeMap<_, _>>() {
            println!("    {key}{sep}{value}");
        }
    }
    if !local.is_empty() {
        println!("  Local:");
        for (key, value) in local.iter().collect::<BTreeMap<_, _>>() {
            let origin = origins.get(key).map_or("unknown".into(), |o| short_path(o));
            let over = if global.contains_key(key) {
                ", overrides global"
            } else {
                ""
            };
            println!("    {key}{sep}{value} ({origin}{over})");
        }
    }
}

fn project_line(cfg: &ResolvedConfig) -> String {
    let files: BTreeSet<String> = cfg
        .local_aliases
        .keys()
        .filter_map(|k| cfg.alias_origins.get(k))
        .chain(cfg.local_env.keys().filter_map(|k| cfg.env_origins.get(k)))
        .chain(
            cfg.local_tools
                .keys()
                .filter_map(|k| cfg.tool_origins.get(k)),
        )
        .map(|o| short_path(o))
        .collect();
    if files.is_empty() {
        "Project: none here (no .avm.json). Showing global only.".to_string()
    } else {
        format!(
            "Project: {}",
            files.into_iter().collect::<Vec<_>>().join(", ")
        )
    }
}

/// Installed versions with the active one starred; flags a pin that is not installed.
pub(super) fn installed_cell(installed: &[String], active: Option<&str>) -> String {
    let mut cells: Vec<String> = installed
        .iter()
        .map(|v| format!("{v}{}", if Some(v.as_str()) == active { "*" } else { "" }))
        .collect();
    if active.is_some_and(|a| !installed.iter().any(|v| v == a)) {
        cells.push("(not installed)".into());
    }
    if cells.is_empty() {
        "-".into()
    } else {
        cells.join("  ")
    }
}

/// TOOL / ACTIVE / SCOPE / SET BY / INSTALLED, one row per pinned or installed tool.
fn print_tool_table(cfg: &ResolvedConfig, installed: &BTreeMap<String, Vec<String>>) {
    let names: BTreeSet<&String> = cfg
        .local_tools
        .keys()
        .chain(cfg.global_tools.keys())
        .chain(installed.keys())
        .collect();
    let mut rows = vec![["TOOL", "ACTIVE", "SCOPE", "SET BY", "INSTALLED"].map(String::from)];
    for name in names {
        let pin = cfg.resolve_tool(name);
        let active = pin.as_ref().map(|(v, _)| v.as_str());
        let versions = installed.get(name).map(Vec::as_slice).unwrap_or(&[]);
        rows.push([
            name.to_string(),
            active.unwrap_or("-").to_string(),
            pin.as_ref()
                .map_or("-", |(_, s)| alias_source_label(s))
                .to_string(),
            cfg.tool_origins
                .get(name)
                .map_or("-".to_string(), |o| short_path(o)),
            installed_cell(versions, active),
        ]);
    }
    let widths: Vec<usize> = (0..4)
        .map(|i| rows.iter().map(|r| r[i].len()).max().unwrap_or(0) + 2)
        .collect();
    for row in rows {
        let lead: String = (0..4)
            .map(|i| format!("{:<w$}", row[i], w = widths[i]))
            .collect();
        println!("{lead}{}", row[4]);
    }
}

/// Installed versions per plugin, numerically sorted; plugins with none are left out.
fn installed_by_tool() -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    let Ok(plugin_manager) = PluginManager::new() else {
        return out;
    };
    for name in plugin_manager.installed_tool_names() {
        let provider: Option<Box<dyn ToolProvider>> =
            if let Some(p) = plugin_manager.protocol_provider(&name) {
                Some(Box::new(p))
            } else {
                plugin_manager
                    .asdf_provider(&name)
                    .map(|p| Box::new(p) as Box<dyn ToolProvider>)
            };
        if let Some(Ok(mut versions)) = provider.map(|p| p.installed_versions()) {
            if !versions.is_empty() {
                crate::version_files::sort_versions(&mut versions);
                out.insert(name, versions);
            }
        }
    }
    out
}

type PluginAliasSections<'a> = BTreeMap<&'a str, BTreeMap<&'a str, (&'a str, &'a str)>>;

/// Plugin-provided aliases not shadowed by a user alias, by section then name.
fn visible_plugin_aliases(cfg: &ResolvedConfig) -> PluginAliasSections<'_> {
    let mut sections = PluginAliasSections::new();
    for (name, alias) in &cfg.plugin_aliases {
        if cfg.local_aliases.contains_key(name) || cfg.global_aliases.contains_key(name) {
            continue;
        }
        sections.entry(&alias.section_name).or_default().insert(
            name,
            (&alias.command, alias.source.as_deref().unwrap_or_default()),
        );
    }
    sections
}

pub fn cmd_list() -> Result<()> {
    let cfg = load_state_with_aliases()?;
    let untrusted = untrusted_files(&cfg);
    if !untrusted.is_empty() {
        eprintln!("{}", untrusted_notice(&untrusted));
    }
    let installed = installed_by_tool();
    let plugin_aliases = visible_plugin_aliases(&cfg);
    let has_tools =
        !cfg.local_tools.is_empty() || !cfg.global_tools.is_empty() || !installed.is_empty();
    let printed = has_tools
        || !cfg.local_aliases.is_empty()
        || !cfg.global_aliases.is_empty()
        || !merge_env(&cfg).is_empty()
        || !plugin_aliases.is_empty();

    if printed {
        println!("{}", project_line(&cfg));
        println!();
    }
    if has_tools {
        print_tool_table(&cfg, &installed);
        println!();
    }
    print_scoped(
        "Aliases",
        &cfg.local_aliases,
        &cfg.global_aliases,
        " → ",
        &cfg.alias_origins,
    );
    print_scoped(
        "Environment",
        &cfg.local_env,
        &cfg.global_env,
        "=",
        &cfg.env_origins,
    );
    if !plugin_aliases.is_empty() {
        println!("Plugin aliases:");
        for (section, items) in plugin_aliases {
            println!("  {section}:");
            for (name, (command, source)) in items {
                if source.is_empty() {
                    println!("    {name} → {command}");
                } else {
                    println!("    {name} → {command} ({source})");
                }
            }
        }
    }

    if !printed {
        println!("No aliases configured.");
        println!();
        println!("Get started:");
        println!("  avm init");
        println!("  avm add start \"npm run dev\"");
        println!("  avm node use 20.11.1");
        println!("  avm plugin add <url-or-path>");
    }

    Ok(())
}
