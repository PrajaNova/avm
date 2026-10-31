use super::*;
use crate::common::ui;

/// Where a plugin's compiled release lives: its name and the `<owner>/<repo>` that releases it.
pub struct PluginSource {
    pub name: String,
    pub repo: String,
}

/// GitHub API base URL, overridable with `AVM_GITHUB_API_URL`.
pub(super) fn github_api_base() -> String {
    std::env::var("AVM_GITHUB_API_URL").unwrap_or_else(|_| "https://api.github.com".to_string())
}

/// GET `path` under the GitHub API and parse the JSON body as `T`; errors on request failure or malformed JSON.
pub(super) fn github_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T> {
    let url = format!("{}/{path}", github_api_base().trim_end_matches('/'));
    let body = github_api_get(&url)
        .with_context(|| fill(msg::MARKETPLACE_QUERY_FAILED, &[("url", &url)]))?;
    serde_json::from_slice(&body)
        .with_context(|| fill(msg::MARKETPLACE_MALFORMED_RESPONSE, &[("url", &url)]))
}

/// Turn what the user typed into a release source:
/// `node` → the repo the registry lists for `node`,
/// `owner/repo` or `https://github.com/owner/repo` → that repo.
/// Returns `None` for anything else (asdf git URLs, local paths).
pub fn resolve_plugin_source(spec: &str) -> Result<Option<PluginSource>> {
    let repo = spec
        .strip_prefix("https://github.com/")
        .map(|r| r.trim_end_matches('/').trim_end_matches(".git"))
        .or_else(|| {
            (!is_git_url(spec) && spec.matches('/').count() == 1 && !Path::new(spec).exists())
                .then_some(spec)
        });
    if let Some(repo) = repo {
        let Some((_, name)) = repo.split_once('/') else {
            return Ok(None);
        };
        // A GitHub URL that isn't an avm plugin repo is an asdf plugin: clone it.
        if is_git_url(spec) && !name.starts_with(PLUGIN_TAG_PREFIX) {
            return Ok(None);
        }
        let name = name.strip_prefix(PLUGIN_TAG_PREFIX).unwrap_or(name);
        avm_plugin_sdk::validate_name(name)?;
        return Ok(Some(PluginSource {
            name: name.to_string(),
            repo: repo.to_string(),
        }));
    }
    if spec.contains('/') || is_git_url(spec) || Path::new(spec).exists() {
        return Ok(None);
    }
    avm_plugin_sdk::validate_name(spec)?;
    Ok(Some(PluginSource {
        name: spec.to_string(),
        repo: registry_repo(spec)?,
    }))
}

/// `(name, description)` rows matching `keyword`: registry plugins plus
/// GitHub repos tagged with the `avm-plugin` topic.
pub fn search_plugins(keyword: &str) -> Result<Vec<(String, String)>> {
    if !keyword
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(anyhow!(msg::MARKETPLACE_KEYWORD_INVALID));
    }
    let mut rows = search_registry(keyword)?;
    #[derive(serde::Deserialize)]
    struct Repo {
        full_name: String,
        description: Option<String>,
    }
    #[derive(serde::Deserialize)]
    struct Found {
        items: Vec<Repo>,
    }
    // Third-party results are best-effort: registry matches still print when GitHub search fails.
    if let Ok(found) =
        github_json::<Found>(&format!("search/repositories?q={keyword}+topic:avm-plugin"))
    {
        rows.extend(
            found
                .items
                .into_iter()
                .map(|r| (r.full_name, r.description.unwrap_or_default())),
        );
    }
    rows.sort();
    rows.dedup_by(|a, b| a.0 == b.0);
    Ok(rows)
}

/// The `(os, arch)` pair used in release asset names (`darwin`/`linux`/`windows`, `arm64`/`amd64`); errors on unsupported platforms.
pub fn marketplace_platform() -> Result<(&'static str, &'static str)> {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        "linux" => "linux",
        "windows" => "windows",
        other => {
            return Err(anyhow!(fill(
                msg::MARKETPLACE_UNSUPPORTED_OS,
                &[("os", other)]
            )))
        }
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "amd64",
        other => {
            return Err(anyhow!(fill(
                msg::MARKETPLACE_UNSUPPORTED_ARCH,
                &[("arch", other)]
            )))
        }
    };
    Ok((os, arch))
}

#[derive(Debug, serde::Deserialize)]
pub struct GithubRelease {
    pub tag_name: String,
    pub assets: Vec<GithubAsset>,
}

#[derive(Debug, serde::Deserialize)]
pub struct GithubAsset {
    pub name: String,
    pub browser_download_url: String,
}

/// `repo`'s release for `tag`, or its latest. `AVM_GITHUB_API_URL` overrides
/// the API base (a local directory works too, for tests).
pub fn github_release(repo: &str, tag: Option<&str>) -> Result<GithubRelease> {
    let api = std::env::var("AVM_GITHUB_API_URL")
        .unwrap_or_else(|_| "https://api.github.com".to_string());
    let which = tag.map_or("latest".to_string(), |t| format!("tags/{t}"));
    let url = format!(
        "{}/repos/{repo}/releases/{which}",
        api.trim_end_matches('/')
    );
    let body = github_api_get(&url).with_context(|| {
        fill(
            msg::MARKETPLACE_RELEASE_QUERY_FAILED,
            &[("which", &which), ("repo", repo)],
        )
    })?;
    serde_json::from_slice(&body)
        .with_context(|| fill(msg::MARKETPLACE_RELEASE_MALFORMED, &[("repo", repo)]))
}

/// The release to install for plugin `name` from `repo`: the newest `avm-plugin-<name>-v*`
/// release (a repo that ships several plugins), else the repo's latest release.
fn plugin_release(repo: &str, name: &str) -> Result<GithubRelease> {
    let prefix = format!("{PLUGIN_TAG_PREFIX}{name}-v");
    // A failed listing is not fatal: a single-plugin repo only needs `latest`.
    let listed: Result<Vec<GithubRelease>> =
        github_json(&format!("repos/{repo}/releases?per_page=100"));
    match listed
        .ok()
        .and_then(|rs| rs.into_iter().find(|r| r.tag_name.starts_with(&prefix)))
    {
        Some(release) => Ok(release),
        None => github_release(repo, None),
    }
}

/// The local path behind a `file://` asset URL (offline mirrors, tests); curl is
/// limited to http(s), so these are copied instead.
fn mirror_path(url: &str) -> Option<&str> {
    let path = url.strip_prefix("file://")?;
    // `file:///D:/dir` leaves `/D:/dir` after the scheme; Windows needs `D:/dir`.
    match path.as_bytes() {
        [b'/', drive, b':', ..] if drive.is_ascii_alphabetic() => Some(&path[1..]),
        _ => Some(path),
    }
}

/// GET a GitHub API URL. Sends `GITHUB_TOKEN`/`GH_TOKEN` when set, since
/// unauthenticated calls share 60/hour per IP (offices, CI runners).
pub(super) fn github_api_get(url: &str) -> Result<Vec<u8>> {
    let token = std::env::var("GITHUB_TOKEN")
        .or_else(|_| std::env::var("GH_TOKEN"))
        .ok()
        .filter(|t| !t.is_empty());
    let Some(token) = token.filter(|_| url.starts_with("https://api.github.com/")) else {
        return fetch(url, MARKETPLACE_TIMEOUT_SECS);
    };
    fetch_with_headers(
        url,
        MARKETPLACE_TIMEOUT_SECS,
        &[
            format!("Authorization: Bearer {token}"),
            "Accept: application/vnd.github+json".to_string(),
        ],
    )
}

/// Download `asset_name` from `release`, verify it against the release's
/// `checksums.txt` (refusing if there is none, unless AVM_ALLOW_UNVERIFIED=1),
/// and extract it into `extract_dir` (which must be private to this process).
/// Returns (sha256, verified).
pub fn fetch_verified_archive(
    repo: &str,
    release: &GithubRelease,
    asset_name: &str,
    extract_dir: &Path,
) -> Result<(String, bool)> {
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| {
            anyhow!(fill(
                msg::MARKETPLACE_ASSET_MISSING,
                &[
                    ("tag", &release.tag_name),
                    ("repo", repo),
                    ("asset", asset_name)
                ]
            ))
        })?;
    let checksums = release.assets.iter().find(|a| a.name == "checksums.txt");
    if checksums.is_none() && std::env::var("AVM_ALLOW_UNVERIFIED").as_deref() != Ok("1") {
        return Err(anyhow!(fill(
            msg::MARKETPLACE_NO_CHECKSUMS,
            &[
                ("tag", &release.tag_name),
                ("repo", repo),
                ("asset", asset_name)
            ]
        )));
    }

    let ext = if asset_name.ends_with(".zip") {
        "zip"
    } else {
        "tar.gz"
    };
    // The archive lives in the caller's private dir, never at a guessable shared /tmp path.
    fs::create_dir_all(extract_dir).context(msg::MARKETPLACE_EXTRACT_DIR_FAILED)?;
    let tmp = extract_dir.join(format!(".download.{ext}"));
    match mirror_path(&asset.browser_download_url) {
        Some(local) => fs::copy(local, &tmp)
            .map(|_| ())
            .with_context(|| fill(msg::MARKETPLACE_MIRROR_COPY_FAILED, &[("local", local)]))?,
        None => avm_plugin_sdk::download::download(
            &asset.browser_download_url,
            &tmp,
            MARKETPLACE_INSTALL_TIMEOUT_MS,
            "AVM_MARKETPLACE_INSTALL_TIMEOUT",
            &fill(msg::MARKETPLACE_DOWNLOADING, &[("asset", asset_name)]),
        )?,
    }

    let (sha256, verified) = match checksums {
        Some(c) => fetch(
            mirror_path(&c.browser_download_url).unwrap_or(&c.browser_download_url),
            MARKETPLACE_TIMEOUT_SECS,
        )
        .context(msg::MARKETPLACE_CHECKSUMS_FAILED)
        .and_then(|sums| verify_sha256(&tmp, &String::from_utf8_lossy(&sums), asset_name))
        .map_err(|e| {
            let _ = fs::remove_file(&tmp);
            anyhow!(fill(
                msg::MARKETPLACE_REFUSING,
                &[("err", &format!("{e:#}")), ("repo", repo)]
            ))
        })
        .map(|hash| (hash, true))?,
        None => {
            ui::warn(fill(msg::MARKETPLACE_UNVERIFIED, &[("asset", asset_name)]));
            (sha256_file(&tmp)?, false)
        }
    };

    let extracted = avm_plugin_sdk::download::extract_archive(
        &tmp,
        extract_dir,
        0,
        MARKETPLACE_INSTALL_TIMEOUT_MS,
        "AVM_MARKETPLACE_INSTALL_TIMEOUT",
        &fill(msg::MARKETPLACE_EXTRACTING, &[("asset", asset_name)]),
    );
    let _ = fs::remove_file(&tmp);
    extracted.with_context(|| fill(msg::MARKETPLACE_EXTRACT_FAILED, &[("asset", asset_name)]))?;
    Ok((sha256, verified))
}

/// Install the compiled GitHub release of `source` (its tag, or the repo's latest
/// when none) — never source, never built locally.
/// `avm-plugin-<name>_<os>_<arch>.tar.gz` (`.zip` on Windows) on `repo`'s
/// selected release, containing exactly one file named `avm-plugin-<name>`, is
/// the expected contract (documented in docs/plugins/CREATING_A_PLUGIN.md).
pub fn install_from_marketplace(source: &PluginSource, plugin_dir: &Path) -> Result<String> {
    let (os, arch) = marketplace_platform()?;
    let (name, repo) = (source.name.as_str(), &source.repo);
    let release = plugin_release(repo, name)?;
    let asset_name = format!("avm-plugin-{name}_{os}_{arch}.{PLUGIN_ARCHIVE_EXT}");
    if !release.assets.iter().any(|a| a.name == asset_name) {
        let prefix = format!("avm-plugin-{name}_");
        let available: Vec<&str> = release
            .assets
            .iter()
            .filter_map(|a| {
                let platform = a.name.strip_prefix(&prefix)?;
                platform
                    .strip_suffix(".tar.gz")
                    .or_else(|| platform.strip_suffix(".zip"))
            })
            .collect();
        let available = if available.is_empty() {
            msg::MARKETPLACE_NONE_AVAILABLE.to_string()
        } else {
            available.join(", ")
        };
        return Err(anyhow!(fill(
            msg::MARKETPLACE_NO_PLATFORM_RELEASE,
            &[
                ("name", name),
                ("os", os),
                ("arch", arch),
                ("tag", &release.tag_name),
                ("repo", repo),
                ("available", &available),
            ]
        )));
    }

    // Staged under the plugin dir (same filesystem, owned by the user), not shared /tmp.
    let extract_dir = plugin_dir.join(format!(".tmp-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&extract_dir);
    let installed = (|| -> Result<(String, bool)> {
        let (sha256, verified) = fetch_verified_archive(repo, &release, &asset_name, &extract_dir)?;
        let bin_name = format!("avm-plugin-{name}{}", std::env::consts::EXE_SUFFIX);
        let extracted_bin = extract_dir.join(&bin_name);
        if !extracted_bin.exists() {
            return Err(anyhow!(fill(
                msg::MARKETPLACE_BIN_MISSING,
                &[("asset", &asset_name), ("bin", &bin_name)]
            )));
        }
        let target_dir = plugin_dir.join(format!("{PROTOCOL_PREFIX}{name}"));
        let bin_dir = target_dir.join("bin");
        fs::create_dir_all(&bin_dir).context(msg::MARKETPLACE_BIN_DIR_FAILED)?;
        let dest = bin_dir.join(PLUGIN_BIN);
        // Copy beside the old binary, then rename over it: a failed copy leaves the
        // working plugin in place, and the new file still lands on a fresh inode
        // (overwriting in place and running it right away gets the process SIGKILLed
        // by macOS Gatekeeper tracking — exit 137). Same dir, so no cross-device rename.
        let staged = bin_dir.join(format!(".{PLUGIN_BIN}.new"));
        fs::copy(&extracted_bin, &staged).context(msg::MARKETPLACE_BIN_MOVE_FAILED)?;
        crate::platform::make_executable(&staged)?;
        fs::rename(&staged, &dest)
            .inspect_err(|_| {
                let _ = fs::remove_file(&staged);
            })
            .context(msg::MARKETPLACE_BIN_MOVE_FAILED)?;
        let meta = serde_json::json!({ "version": release.tag_name, "sha256": sha256, "verified": verified });
        fs::write(target_dir.join("meta.json"), meta.to_string())
            .context(msg::MARKETPLACE_META_FAILED)?;
        Ok((sha256, verified))
    })();
    let _ = fs::remove_dir_all(&extract_dir);
    installed?;

    Ok(release.tag_name)
}

#[cfg(test)]
mod mirror_tests {
    use super::mirror_path;

    #[test]
    fn file_urls_become_local_paths() {
        assert_eq!(mirror_path("file:///tmp/a.zip"), Some("/tmp/a.zip"));
        assert_eq!(mirror_path("file:///D:/a/b.zip"), Some("D:/a/b.zip"));
        assert_eq!(mirror_path("https://example.com/a.zip"), None);
    }
}
