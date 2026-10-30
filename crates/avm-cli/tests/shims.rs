//! Shims: `avm shims install/activate/remove` and `exec-shim` dispatch.
//! Source feature: `crates/avm-cli/src/shims/`.

use crate::common::*;
use fs_err as fs;
use speculoos::prelude::*;

/// A pin that is a path (absolute, `../`, Windows drive) is rejected instead of being used as a directory.
///
/// 1. For each malicious spec, write it as the project's node pin.
/// 2. Run a shim call.
/// 3. It fails with "invalid version spec".
#[test]
fn untrusted_tool_pins_cannot_escape_managed_directories() {
    let p = Sandbox::new("avm-test");
    for spec in [
        p.root.to_string_lossy().to_string(),
        "../payload".into(),
        "C:\\payload".into(),
    ] {
        fs::write(
            p.root.join("project/.avm.json"),
            serde_json::json!({"tools":{"node":spec}}).to_string(),
        )
        .unwrap();
        let out = p.run(&["exec-shim", "node", "--", "--version"]);
        assert_that(&out).fails();
        assert_that(&out).stderr_contains("invalid version spec");
    }
}

/// Binaries installed by a tool (e.g. `foo.bar`) are runnable through shims.
///
/// 1. Create `tools/node/20.1.0/bin/foo.bar` and a minimal node plugin; pin node 20.1.0.
/// 2. Run `avm exec-shim foo.bar`.
/// 3. The binary runs and prints its output.
#[cfg(unix)]
#[test]
fn managed_package_binaries_can_have_dotted_names() {
    use std::os::unix::fs::PermissionsExt;
    let p = Sandbox::new("avm-test");
    let bin = p.root.join("home/.avm/tools/node/20.1.0/bin");
    let plugin = p.root.join("home/.avm/plugins/avm-plugin-node/bin");
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&plugin).unwrap();
    for (file, script) in [
        (bin.join("foo.bar"), "#!/bin/sh\necho managed-package\n"),
        (
            plugin.join("avm-plugin"),
            "#!/bin/sh\necho '{\"env\":{}}'\n",
        ),
    ] {
        fs::write(&file, script).unwrap();
        fs::set_permissions(file, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    fs::write(
        p.root.join("project/.avm.json"),
        r#"{"tools":{"node":"20.1.0"}}"#,
    )
    .unwrap();
    let out = p.run(&["exec-shim", "foo.bar"]);
    assert_that(&out).succeeds();
    assert_eq!(out.stdout, b"managed-package\n");
}

/// `shims activate` appends to a shell rc file byte-for-byte and only once.
///
/// 1. Create `.bashrc` containing a Latin-1 byte.
/// 2. Run `avm shims activate`: the original bytes are a prefix and the avm block was added.
/// 3. Run it again: the file is unchanged.
#[test]
fn shims_activate_appends_without_rewriting_non_utf8_rc_files() {
    let p = Sandbox::new("avm-test");
    let bashrc = p.root.join("home/.bashrc");
    let original = b"# caf\xe9 latin-1 comment\nalias ll='ls -l'\n".to_vec();
    fs::write(&bashrc, &original).unwrap();
    assert_that(&p.run(&["shims", "activate"])).succeeds();
    let after = fs::read(&bashrc).unwrap();
    assert!(
        after.starts_with(&original),
        "existing bytes must be preserved"
    );
    assert!(String::from_utf8_lossy(&after).contains("# >>> avm shims >>>"));
    assert_that(&p.run(&["shims", "activate"])).succeeds();
    assert_eq!(
        fs::read(&bashrc).unwrap(),
        after,
        "second run must not append again"
    );
}

/// Re-running `shims install` does not rewrite shims that are already correct.
///
/// 1. Run `avm shims install` and record the `node` shim's inode.
/// 2. Run it again: the inode is the same.
#[cfg(unix)]
#[test]
fn shims_install_leaves_unchanged_shims_untouched() {
    use std::os::unix::fs::MetadataExt;
    let p = Sandbox::new("avm-test");
    assert_that(&p.run(&["shims", "install"])).succeeds();
    let node = p.root.join("home/.avm/shims/node");
    let inode = fs::metadata(&node).unwrap().ino();
    assert_that(&p.run(&["shims", "install"])).succeeds();
    assert_eq!(fs::metadata(&node).unwrap().ino(), inode);
}

/// `avm shims path / install / remove / activate` and `exec-shim` errors.
mod commands {
    use super::*;
    use rstest::rstest;

    /// `shims path` prints the managed shim directory.
    ///
    /// 1. Run `avm shims path`.
    /// 2. It prints `<AVM_HOME>/shims`.
    #[test]
    fn path_prints_the_shim_directory() {
        let p = Sandbox::new("avm-test");
        let expected = slashes(&p.path("home/.avm/shims").display().to_string());
        assert_that(&slashes(success(p.run(&["shims", "path"])).trim())).is_equal_to(expected);
    }

    /// `install` (and its `reshim` alias) generate a shim per managed tool.
    ///
    /// 1. Run the command with no tools installed.
    /// 2. It reports "avm shims installed." and `node`, `npm`, `java`, `javac` shims exist.
    #[cfg(unix)]
    #[rstest]
    #[case::install("install")]
    #[case::reshim("reshim")]
    fn install_creates_the_shims(#[case] verb: &str) {
        use std::os::unix::fs::PermissionsExt;
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(&["shims", verb]))
            .succeeds()
            .stdout_contains("avm shims installed.");
        for tool in ["node", "npm", "java", "javac"] {
            let shim = p.path(&format!("home/.avm/shims/{tool}"));
            assert_that(&shim).exists();
            assert_that(&(fs::metadata(&shim).unwrap().permissions().mode() & 0o111 != 0))
                .is_true();
        }
    }

    /// `remove` deletes a single shim and leaves the others.
    ///
    /// 1. Run `shims install`, then `shims remove node`.
    /// 2. It reports "Removed shim for node"; `node` is gone and `java` remains.
    #[test]
    fn remove_deletes_one_shim() {
        let p = Sandbox::new("avm-test");
        success(p.run(&["shims", "install"]));
        assert_that(&p.run(&["shims", "remove", "node"]))
            .succeeds()
            .stdout_contains("Removed shim for node");
        let ext = std::env::consts::EXE_SUFFIX; // Windows shims are node.exe, java.exe
        assert_that(&p.path(&format!("home/.avm/shims/node{ext}"))).does_not_exist();
        assert_that(&p.path(&format!("home/.avm/shims/java{ext}"))).exists();
    }

    /// Shim names are validated as plain names.
    ///
    /// 1. Run `shims remove` with a path-like or empty-ish name.
    /// 2. It fails with "invalid managed name".
    #[rstest]
    #[case::parent("../x")]
    #[case::slash("a/b")]
    #[case::dot("..")]
    fn remove_rejects_path_like_names(#[case] name: &str) {
        let p = Sandbox::new("avm-test");
        failure(p.run(&["shims", "remove", name]), "invalid managed name");
    }

    /// `activate` adds one marked block to each startup file, and only once.
    ///
    /// 1. Run `avm shims activate`.
    /// 2. `.zshenv`, `.bashrc` and `.profile` each contain the avm block.
    /// 3. A second run reports "already active" and leaves every file unchanged.
    #[cfg(unix)]
    #[test]
    fn activate_updates_each_startup_file_once() {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(&["shims", "activate"]))
            .succeeds()
            .stdout_contains("Added avm shims to");
        let files = [".zshenv", ".bashrc", ".profile"];
        let before: Vec<_> = files.iter().map(|f| p.read(&format!("home/{f}"))).collect();
        for text in &before {
            assert_that(text).contains("# >>> avm shims >>>");
        }
        assert_that(&p.run(&["shims", "activate"]))
            .succeeds()
            .stdout_contains("already active");
        for (file, original) in files.iter().zip(before) {
            assert_that(&p.read(&format!("home/{file}"))).is_equal_to(original);
        }
    }

    /// The internal shim dispatcher reports a missing command clearly.
    ///
    /// 1. Run `avm exec-shim nosuch`.
    /// 2. It fails with "command 'nosuch' not found in PATH".
    #[test]
    fn exec_shim_reports_an_unknown_command() {
        let p = Sandbox::new("avm-test");
        failure(
            p.run(&["exec-shim", "nosuch"]),
            "command 'nosuch' not found in PATH",
        );
    }

    /// `exec-shim` needs a tool name.
    ///
    /// 1. Run `avm exec-shim` with no arguments.
    /// 2. clap exits 2 asking for `<TOOL>`.
    #[test]
    fn exec_shim_requires_a_tool_name() {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(&["exec-shim"]))
            .exits_with(2)
            .stderr_contains("<TOOL>");
    }
}
