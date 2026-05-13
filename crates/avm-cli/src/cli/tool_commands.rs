fn cmd_tool(cmd: Option<ToolCommands>) -> Result<()> {
    let cmd = cmd.unwrap_or(ToolCommands::List);
    let node = NodeProvider::new();
    let cfg = load_state()?;
    match cmd {
        ToolCommands::List => {
            print_tool_list(&cfg, &node)?;
            Ok(())
        }
        ToolCommands::Use(args) => {
            set_tool_version(&args.tool, &args.version, args.global)
        }
        ToolCommands::Install(args) => {
            if args.tool != "node" {
                return Err(anyhow!("unsupported tool '{}'", args.tool));
            }
            node.install(&args.version)?;
            println!("✓ Installed {} {}", args.tool, args.version);
            Ok(())
        }
        ToolCommands::Uninstall(args) => {
            if args.tool != "node" {
                return Err(anyhow!("unsupported tool '{}'", args.tool));
            }
            node.uninstall(&args.version)?;
            println!("✓ Removed {} {}", args.tool, args.version);
            Ok(())
        }
        ToolCommands::Node(args) => cmd_node_tool(args, &cfg, &node),
    }
}

fn cmd_node_tool(
    args: NodeToolArgs,
    cfg: &ResolvedConfig,
    node: &NodeProvider,
) -> Result<()> {
    let parts = args.args;
    match parts.as_slice() {
        [] => {
            print_node_tool_status(cfg, node)?;
            Ok(())
        }
        [cmd] if cmd == "list" || cmd == "ls" => {
            print_node_tool_status(cfg, node)?;
            Ok(())
        }
        [cmd] if cmd == "versions" || cmd == "available" => {
            print_available_node_versions(node, NodeVersionFilter::Recent)?;
            Ok(())
        }
        [filter, cmd] if cmd == "versions" || cmd == "available" => {
            let filter = parse_node_version_filter(filter)?;
            print_available_node_versions(node, filter)?;
            Ok(())
        }
        [cmd, version] if cmd == "use" => set_tool_version("node", version, false),
        [cmd, version, flag] if cmd == "use" && (flag == "--global" || flag == "-g") => {
            set_tool_version("node", version, true)
        }
        [cmd, version] if cmd == "install" => {
            node.install(version)?;
            println!("✓ Installed node {version}");
            Ok(())
        }
        [cmd, version] if cmd == "uninstall" => {
            node.uninstall(version)?;
            println!("✓ Removed node {version}");
            Ok(())
        }
        [cmd] if cmd == "--help" || cmd == "-h" || cmd == "help" => {
            print_node_tool_help();
            Ok(())
        }
        _ => Err(anyhow!(
            "unknown node tool command. Try `avm tool node --help`"
        )),
    }
}
