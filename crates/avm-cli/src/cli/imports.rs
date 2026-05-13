use anyhow::{anyhow, Context, Result};
use avm_core::{load_with_env, AliasSource, ConfigLoadResult, Resolver, ResolvedConfig};
use avm_plugin_api::ResolvedAlias;
use avm_plugin_api::ToolProvider;
use avm_plugin_node::{NodeAlias, NodeProvider};
use avm_runtime::PluginManager;
use avm_shims::{install_shims, remove_shim, shim_path_env};
use clap::{Args, Parser, Subcommand};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const CONFIG_FILE: &str = ".avm.json";
const BUILTIN_PLUGIN_DIR: &str = ".builtins";
const BUILTIN_NODE_PLUGIN_MARKER: &str = "node";
