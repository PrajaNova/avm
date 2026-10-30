use super::*;

#[derive(Debug)]
pub struct PluginManager {
    pub(super) plugin_dir: PathBuf,
}

impl PluginManager {
    /// Create a manager over the plugin directory, creating it if missing.
    pub fn new() -> Result<Self> {
        let dir = default_plugin_dir();
        fs::create_dir_all(&dir).context("create plugin directory")?;
        Ok(Self { plugin_dir: dir })
    }

    /// The directory user plugins are installed into.
    pub fn plugin_dir(&self) -> PathBuf {
        self.plugin_dir.clone()
    }

    /// Plugin directories as `(dir name, path)`, sorted case-insensitively.
    fn plugin_dirs(&self) -> Vec<(String, PathBuf)> {
        let mut dirs: Vec<(String, PathBuf)> = fs::read_dir(&self.plugin_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.file_type().map(|ty| ty.is_dir()).unwrap_or(false))
            .map(|entry| {
                (
                    entry.file_name().to_string_lossy().to_string(),
                    entry.path(),
                )
            })
            .collect();
        dirs.sort_by_key(|(name, _)| name.to_ascii_lowercase());
        dirs
    }

    /// First plugin dir that passes `is_kind` and maps to tool `name` once
    /// `prefix` (`avm-plugin-` / `asdf-`) is stripped.
    fn find(
        &self,
        name: &str,
        prefix: &str,
        is_kind: fn(&Path) -> bool,
    ) -> Option<(String, PathBuf)> {
        self.plugin_dirs()
            .into_iter()
            .find(|(dir, path)| is_kind(path) && tool_name(dir, prefix) == name)
    }

    /// Accepts either the literal plugin directory name or a tool name that
    /// resolves to one via the `avm-plugin-<name>` / `asdf-<name>` conventions.
    fn find_any(&self, name: &str) -> Option<(String, PathBuf)> {
        let literal = self.plugin_dir.join(name);
        if literal.exists() {
            return Some((name.to_string(), literal));
        }
        self.find(name, PROTOCOL_PREFIX, is_protocol_plugin_source)
            .or_else(|| self.find(name, ASDF_PREFIX, is_asdf_plugin_source))
    }

    /// Tool names of installed plugins, read from folder names (no plugin process is started).
    pub fn installed_tool_names(&self) -> BTreeSet<String> {
        self.plugin_dirs()
            .into_iter()
            .filter_map(|(name, path)| {
                if is_protocol_plugin_source(&path) {
                    Some(tool_name(&name, PROTOCOL_PREFIX).to_string())
                } else if is_asdf_plugin_source(&path) {
                    Some(tool_name(&name, ASDF_PREFIX).to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Install an asdf-style plugin from a git URL (shallow clone) or a local directory (copy) into the plugin directory.
    /// Errors on symlinked or unsafe sources, an existing install, or a source lacking `bin/list-all` and `bin/install`; a failed install is cleaned up.
    pub fn install_plugin(&self, source: &str) -> Result<()> {
        let is_remote = is_git_url(source);
        let target = if is_remote {
            let plugin_name = derive_remote_plugin_name(source)?;
            self.plugin_dir.join(plugin_name)
        } else {
            let source_metadata =
                fs::symlink_metadata(source).context("invalid plugin source path")?;
            if source_metadata.file_type().is_symlink() {
                return Err(anyhow!("plugin source must not be a symlink"));
            }

            let source_dir = fs::canonicalize(source).context("invalid plugin source path")?;
            validate_plugin_source_permissions(&source_dir)?;
            let source_name = source_dir
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow!("invalid plugin source"))?;
            self.plugin_dir.join(source_name)
        };

        if target.exists() {
            return Err(anyhow!(
                "plugin already installed; run `avm uninstall <name>` first",
            ));
        }

        let install_result = if is_remote {
            let mut git = Command::new("git");
            git.args(["clone", "--depth", "1", source])
                .arg(&target)
                .env_clear()
                .env("PATH", DEFAULT_PLUGIN_PATH_ENV);
            run_timed(
                git,
                GIT_CLONE_TIMEOUT_MS,
                "git clone",
                "AVM_GIT_CLONE_TIMEOUT",
            )
        } else {
            let source = fs::canonicalize(source).context("invalid plugin source path")?;
            copy_dir_recursive(&source, &target).context("plugin copy failed")
        };

        if let Err(err) = install_result {
            let _ = fs::remove_dir_all(&target);
            return Err(err);
        }

        if is_asdf_plugin_source(&target) {
            return Ok(());
        }

        let _ = fs::remove_dir_all(&target);
        Err(anyhow!(
            "invalid plugin: missing asdf bin/list-all + bin/install"
        ))
    }

    /// Remove the installed plugin directory for `name`, if any; errors on an invalid name or failed removal.
    pub fn remove_plugin(&self, name: &str) -> Result<()> {
        avm_plugin_api::validate_name(name)?;
        if let Some((_, target)) = self.find_any(name) {
            fs::remove_dir_all(target).context("remove plugin")?;
        }
        Ok(())
    }

    /// The installed provider for `name`: a protocol plugin first, then the asdf adapter.
    pub fn provider(&self, name: &str) -> Option<Box<dyn ToolProvider>> {
        if let Some(process) = self.protocol_provider(name) {
            return Some(Box::new(process));
        }
        self.asdf_provider(name)
            .map(|asdf| Box::new(asdf) as Box<dyn ToolProvider>)
    }

    /// The installed asdf-style plugin for `name`, if any.
    pub fn asdf_provider(&self, name: &str) -> Option<AsdfToolProvider> {
        let (plugin_name, plugin_path) = self.find(name, ASDF_PREFIX, is_asdf_plugin_source)?;
        Some(AsdfToolProvider {
            name: name.to_string(),
            plugin_name,
            plugin_path,
        })
    }

    /// A user-installed plugin speaking the native protocol
    /// (`docs/plugins/CREATING_A_PLUGIN.md`): a plugin directory named
    /// `avm-plugin-<name>` with an executable `bin/avm-plugin` is surfaced
    /// as tool `<name>`.
    pub fn protocol_provider(&self, name: &str) -> Option<PluginProcess> {
        let (_, plugin_path) = self.find(name, PROTOCOL_PREFIX, is_protocol_plugin_source)?;
        Some(PluginProcess::new(
            name,
            plugin_path.join("bin").join(PLUGIN_BIN),
        ))
    }
}

/// Plugin directory: `AVM_PLUGIN_DIR` if set, else `~/.avm/plugins`.
pub(super) fn default_plugin_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("AVM_PLUGIN_DIR") {
        return PathBuf::from(dir);
    }
    crate::shims::avm_home()
        .map(|home| home.join("plugins"))
        .unwrap_or_else(|_| PathBuf::from(".avm/plugins"))
}

/// An asdf plugin has both `bin/list-all` and `bin/install`.
pub(super) fn is_asdf_plugin_source(path: &Path) -> bool {
    path.join("bin").join("list-all").exists() && path.join("bin").join("install").exists()
}

/// Whether `path` holds a protocol plugin: it has a `bin/avm-plugin` entry (existence only, not the exec bit).
pub(super) fn is_protocol_plugin_source(path: &Path) -> bool {
    path.join("bin").join(PLUGIN_BIN).exists()
}

/// Plugin dir name → tool name (`avm-plugin-node` → `node`, `asdf-kotlin` → `kotlin`).
pub(super) fn tool_name<'a>(dir: &'a str, prefix: &str) -> &'a str {
    dir.strip_prefix(prefix).unwrap_or(dir)
}

/// Check a local plugin source is a directory, not root-owned (unless we are root), and not world-writable (Unix).
pub(super) fn validate_plugin_source_permissions(path: &Path) -> Result<()> {
    let meta = fs::metadata(path).context("failed to read plugin source metadata")?;
    if !meta.is_dir() {
        return Err(anyhow!("plugin source must be a directory"));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        use std::os::unix::fs::PermissionsExt;
        if meta.uid() == 0 && current_euid() != 0 {
            return Err(anyhow!("plugin sources owned by root are not allowed"));
        }
        let mode = meta.permissions().mode();
        if mode & 0o002 != 0 {
            return Err(anyhow!("plugin source must not be world-writable"));
        }
    }

    Ok(())
}

/// The process's effective user id.
#[cfg(unix)]
pub(super) fn current_euid() -> u32 {
    // SAFETY:
    // Operation: `libc::geteuid()`.
    // Contract (POSIX `geteuid`): takes no arguments and always succeeds; it has
    // no memory-safety preconditions and does not touch Rust-managed memory.
    // The `libc` crate declares the return type as the platform's `uid_t`, so
    // there is no hand-written ABI declaration to get wrong.
    // Postcondition: none for Rust state; the returned id is a plain integer.
    unsafe { libc::geteuid() }
}

/// Derive the plugin directory name from a git URL's last path segment, minus any `.git` suffix, query or fragment.
/// Errors on an empty source or an unusable name.
pub(super) fn derive_remote_plugin_name(source: &str) -> Result<String> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("plugin source is empty"));
    }

    let mut candidate = trimmed;
    if candidate.starts_with("git@") {
        candidate = candidate
            .split_once(':')
            .map(|(_, tail)| tail)
            .unwrap_or(candidate);
    }

    candidate = candidate
        .split(['#', '?'])
        .next()
        .unwrap_or(candidate)
        .trim_end_matches('/');
    let name = candidate
        .split('/')
        .rfind(|part| !part.is_empty())
        .ok_or_else(|| anyhow!("unable to derive plugin name"))?;

    let name = name.strip_suffix(".git").unwrap_or(name);
    if name.is_empty() || name.contains('\0') || name.contains('/') || name.contains('\\') {
        return Err(anyhow!("invalid plugin name derived from source"));
    }
    if name.chars().any(|ch| ch.is_control()) {
        return Err(anyhow!("invalid plugin name derived from source"));
    }

    Ok(name.to_string())
}

/// Copy a directory tree; errors on symlinks or special files so a plugin cannot point outside its own directory.
pub(super) fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination).context("failed to create plugin destination")?;
    for entry in fs::read_dir(source).context("failed to read plugin source")? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src = entry.path();
        let dst = destination.join(entry.file_name());

        if file_type.is_symlink() {
            return Err(anyhow!(
                "plugin source contains symlink entries; remove symlinks before installing"
            ));
        }

        if file_type.is_dir() {
            copy_dir_recursive(&src, &dst)?;
            continue;
        }

        if file_type.is_file() {
            fs::copy(&src, &dst).with_context(|| format!("failed to copy {:?}", src))?;
            continue;
        }

        return Err(anyhow!("unsupported plugin source entry type: {:?}", src));
    }
    Ok(())
}

/// Whether `source` looks like a remote git URL (http(s), git@, git://, ssh://) rather than a local path.
pub(super) fn is_git_url(source: &str) -> bool {
    source.starts_with("https://")
        || source.starts_with("http://")
        || source.starts_with("git@")
        || source.starts_with("git://")
        || source.starts_with("ssh://")
}
