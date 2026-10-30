//! Shared harness for the end-to-end tests: a throwaway home + project sandbox, helpers to
//! run `avm-bin` (directly or through a real shell), and fluent (speculoos) assertions over a
//! finished run. Test data (fake plugins, prebuilt projects) lives in `fixtures/`.
#![allow(dead_code)]

pub mod bench;

use speculoos::prelude::*;
use speculoos::{AssertionFailure, Spec};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// A temp dir holding `home/` and `project/`, removed on drop.
pub struct Sandbox {
    pub root: PathBuf,
}

impl Sandbox {
    /// Creates `<tmp>/<prefix>-<pid>-<n>/{home,project}`; `n` keeps parallel tests apart.
    pub fn new(prefix: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "{prefix}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("home")).unwrap();
        fs::create_dir_all(root.join("project")).unwrap();
        Self { root }
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// `avm-bin` run in the project dir, isolated from the real environment (own `HOME`,
    /// no update check, nothing trusted). Callers layer extra `.env(..)` on top.
    pub fn command(&self, args: &[&str]) -> Command {
        self.command_in(args, "project")
    }

    /// Like [`Sandbox::command`], run from `cwd` (relative to the sandbox root).
    pub fn command_in(&self, args: &[&str], cwd: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_avm-bin"));
        command
            .args(args)
            .current_dir(self.path(cwd))
            .env("HOME", self.path("home"))
            .env("AVM_HOME", self.path("home/.avm"))
            .env("AVM_PLUGIN_DIR", self.path("home/.avm/plugins"))
            .env("AVM_NO_UPDATE_CHECK", "1")
            .env("AVM_TRUST_ALL", "0")
            .env_remove("AVM_ENV")
            // Provider variables from the machine running the tests change what `avm env` prints.
            .env_remove("JAVA_HOME")
            .env_remove("ANDROID_HOME")
            .env_remove("ANDROID_SDK_ROOT");
        command
    }

    pub fn run_in(&self, args: &[&str], cwd: &str) -> Output {
        self.command_in(args, cwd).output().unwrap()
    }

    pub fn run(&self, args: &[&str]) -> Output {
        self.run_in(args, "project")
    }

    /// Like [`Sandbox::run`] with the project's config and `.env` trusted (`AVM_TRUST_ALL=1`).
    pub fn run_trusted(&self, args: &[&str]) -> Output {
        self.command(args)
            .env("AVM_TRUST_ALL", "1")
            .output()
            .unwrap()
    }

    /// Writes `contents` to `relative` (creating parent dirs).
    pub fn write(&self, relative: &str, contents: impl AsRef<[u8]>) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    pub fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.path(relative)).unwrap()
    }

    pub fn read_json(&self, relative: &str) -> serde_json::Value {
        read_json(&self.path(relative))
    }

    /// Runs `script` in `shell` with this sandbox's environment and `AVM_TRUST_ALL=1`.
    /// Returns `None` when the shell is not installed, so callers can skip it.
    pub fn run_shell(&self, shell: &str, script: &str) -> Option<Output> {
        let mut cmd = Command::new(shell);
        let base = self.command(&[]);
        cmd.args(["-c", script])
            .current_dir(self.path("project"))
            .envs(base.get_envs().filter_map(|(k, v)| v.map(|v| (k, v))))
            .env("AVM_TRUST_ALL", "1");
        match cmd.output() {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            output => Some(output.unwrap()),
        }
    }
}

/// Directory holding the freshly built `avm-bin` (and, in a workspace build, the plugins).
pub fn bin_dir() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_avm-bin")).parent().unwrap()
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// stdout as text with Windows line endings folded to `\n`.
pub fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n")
}

pub fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n")
}

/// Folds Windows path separators to `/`, so a test can assert `project/.avm.json` on every OS.
pub fn slashes(text: &str) -> String {
    text.replace('\\', "/")
}

/// Asserts success and returns stdout.
pub fn success(output: Output) -> String {
    assert_that(&output).succeeds();
    stdout_of(&output)
}

/// Asserts failure with `expected` somewhere in stderr.
pub fn failure(output: Output, expected: &str) {
    assert_that(&output).fails_with(expected);
}

/// Fluent checks on a finished process: `assert_that(&out).succeeds().stdout_contains("x")`.
pub trait OutputAssertions {
    fn succeeds(&mut self) -> &mut Self;
    fn fails(&mut self) -> &mut Self;
    fn fails_with(&mut self, needle: &str) -> &mut Self;
    fn stdout_contains(&mut self, needle: &str) -> &mut Self;
    fn stderr_contains(&mut self, needle: &str) -> &mut Self;
    fn stderr_lacks(&mut self, needle: &str) -> &mut Self;
    fn exits_with(&mut self, code: i32) -> &mut Self;
}

/// Fails the spec, showing what was expected and the process's stdout/stderr.
fn fail(spec: &Spec<'_, Output>, expected: String) {
    let actual = format!(
        "status {}\nstdout: {}\nstderr: {}",
        spec.subject.status,
        stdout_of(spec.subject),
        stderr_of(spec.subject)
    );
    AssertionFailure::from_spec(spec)
        .with_expected(expected)
        .with_actual(actual)
        .fail();
}

impl OutputAssertions for Spec<'_, Output> {
    fn succeeds(&mut self) -> &mut Self {
        if !self.subject.status.success() {
            fail(self, "a successful exit".into());
        }
        self
    }

    fn fails(&mut self) -> &mut Self {
        if self.subject.status.success() {
            fail(self, "a failing exit".into());
        }
        self
    }

    fn fails_with(&mut self, needle: &str) -> &mut Self {
        self.fails().stderr_contains(needle)
    }

    fn stdout_contains(&mut self, needle: &str) -> &mut Self {
        if !slashes(&stdout_of(self.subject)).contains(needle) {
            fail(self, format!("stdout containing {needle:?}"));
        }
        self
    }

    fn stderr_contains(&mut self, needle: &str) -> &mut Self {
        if !slashes(&stderr_of(self.subject)).contains(needle) {
            fail(self, format!("stderr containing {needle:?}"));
        }
        self
    }

    fn stderr_lacks(&mut self, needle: &str) -> &mut Self {
        if slashes(&stderr_of(self.subject)).contains(needle) {
            fail(self, format!("stderr without {needle:?}"));
        }
        self
    }

    fn exits_with(&mut self, code: i32) -> &mut Self {
        if self.subject.status.code() != Some(code) {
            fail(self, format!("exit code {code}"));
        }
        self
    }
}

/// Reads a JSON file, panicking with the path on failure.
pub fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

/// Whether tests run as root, which bypasses file permissions (so permission tests must skip).
pub fn running_as_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
}
