//! The shell wrapper from `avm shell-init` (bash and zsh).
//! Source feature: `crates/avm-cli/src/app/` and `common/shell/`.

use crate::common::*;
use fs_err as fs;
use speculoos::prelude::*;
use std::process::Command;

/// The shell wrapper applies a project's `env` on entry and restores the user's own values on leaving (bash and zsh).
///
/// 1. Start a shell with `AVM_TEST_EXISTING` set and `AVM_TEST_NEW` unset; the project sets both.
/// 2. Load `avm-bin shell-init`; inside the project both project values are active.
/// 3. `cd ..` and run an `avm` command: the original value is back and the new variable is gone.
/// 4. Re-enter the project and re-apply: project values return; `avm env remove` unsets one.
/// 5. Back outside, `avm-bin run check` sees the restored values. zsh is skipped when not installed.
#[cfg(unix)]
#[test]
fn shell_environment_restores_original_values_when_leaving_projects() {
    let p = Sandbox::new("avm-test");
    fs::write(p.root.join("home/.avm.json"), serde_json::json!({"aliases":{"check":r#"test "$AVM_TEST_EXISTING" = "original ' value" && test "${AVM_TEST_NEW+x}" != x"#}}).to_string()).unwrap();
    fs::write(
        p.root.join("project/.avm.json"),
        r#"{"env":{"AVM_TEST_EXISTING":"project","AVM_TEST_NEW":"temporary"}}"#,
    )
    .unwrap();
    let bin = std::path::Path::new(env!("CARGO_BIN_EXE_avm-bin"))
        .parent()
        .unwrap();
    let script = format!(
        r#"
export PATH={}:$PATH
eval "$(avm-bin shell-init)"
[ "$AVM_TEST_EXISTING" = project ] && [ "$AVM_TEST_NEW" = temporary ] || exit 10
cd ..
avm plugin list >/dev/null
[ "$AVM_TEST_EXISTING" = "original ' value" ] && [ "${{AVM_TEST_NEW+x}}" != x ] || exit 11
cd project
_avm_apply_env
[ "$AVM_TEST_EXISTING" = project ] && [ "$AVM_TEST_NEW" = temporary ] || exit 12
avm env remove AVM_TEST_NEW >/dev/null
[ "${{AVM_TEST_NEW+x}}" != x ] || exit 13
cd ..
command avm-bin run check || exit 14
printf restored
"#,
        avm_plugin_sdk::sh_quote(&bin.to_string_lossy())
    );
    for shell in ["bash", "zsh"] {
        fs::write(
            p.root.join("project/.avm.json"),
            r#"{"env":{"AVM_TEST_EXISTING":"project","AVM_TEST_NEW":"temporary"}}"#,
        )
        .unwrap();
        let output = Command::new(shell)
            .args(["-c", &script])
            .current_dir(p.root.join("project"))
            .env("HOME", p.root.join("home"))
            .env("AVM_HOME", p.root.join("home/.avm"))
            .env("AVM_TRUST_ALL", "1")
            .env("AVM_NO_UPDATE_CHECK", "1")
            .env("AVM_TEST_EXISTING", "original ' value")
            .env_remove("AVM_TEST_NEW")
            .output();
        if shell == "zsh"
            && output
                .as_ref()
                .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        {
            continue;
        }
        let out = output.unwrap();
        assert!(
            out.status.success(),
            "{shell}: {} {:?}",
            String::from_utf8_lossy(&out.stderr),
            out.status
        );
        assert_eq!(out.stdout, b"restored");
    }
}

/// The `avm` shell function forwards `--env <profile>` and `--env=<profile>` to the binary.
///
/// 1. Create a root config with `build` = base and a `ci` profile overriding it.
/// 2. Load the shell wrapper in bash and zsh.
/// 3. `avm --env ci which build` and `avm --env=ci which build` report the ci alias; `avm --env ci run build` prints "ci".
/// 4. Without the option `avm run build` prints "base".
#[cfg(unix)]
#[test]
fn shell_wrapper_forwards_profile_options() {
    let p = Sandbox::new("avm-test");
    fs::write(
        p.root.join("project/.avm.json"),
        r#"{"root":true,"aliases":{"build":"echo base"}}"#,
    )
    .unwrap();
    fs::write(
        p.root.join("project/.avm.ci.json"),
        r#"{"aliases":{"build":"echo ci"}}"#,
    )
    .unwrap();
    let bin = std::path::Path::new(env!("CARGO_BIN_EXE_avm-bin"))
        .parent()
        .unwrap();
    let script = format!(
        r#"
export PATH={}:$PATH
eval "$(avm-bin shell-init)"
case "$(avm --env ci which build)" in *"echo ci"*) ;; *) exit 10 ;; esac
case "$(avm --env=ci which build)" in *"echo ci"*) ;; *) exit 11 ;; esac
[ "$(avm --env ci run build)" = ci ] || exit 12
[ "$(avm run build)" = base ] || exit 13
"#,
        avm_plugin_sdk::sh_quote(&bin.to_string_lossy())
    );
    for shell in ["bash", "zsh"] {
        let mut settings = p.command(&[]);
        settings.env("AVM_TRUST_ALL", "1");
        let mut cmd = Command::new(shell);
        cmd.args(["-c", &script])
            .current_dir(settings.get_current_dir().unwrap())
            .envs(settings.get_envs().filter_map(|(k, v)| v.map(|v| (k, v))));
        cmd.env_remove("AVM_ENV");
        let output = cmd.output();
        if shell == "zsh"
            && output
                .as_ref()
                .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        {
            continue;
        }
        success(output.unwrap());
    }
}

/// `avm create` and `avm shell-init`.
mod commands {
    use super::*;
    use rstest::rstest;

    /// `create` prints the command that scaffolds a plugin from the template repo.
    ///
    /// 1. Run `avm create foo`.
    /// 2. It prints a `gh repo create avm-plugin-foo --template ...` line.
    #[test]
    fn create_prints_the_template_command() {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(&["create", "foo"]))
            .succeeds()
            .stdout_contains("gh repo create avm-plugin-foo --template");
    }

    /// `shell-init` emits setup for each supported shell and defaults to a POSIX one.
    ///
    /// 1. Run `avm shell-init [shell]`.
    /// 2. The script defines the shim directory and the matching `avm` function.
    #[rstest]
    #[case::sh(&["shell-init", "sh"], "AVM_SHIM_DIR=", "avm()")]
    #[case::pwsh(&["shell-init", "pwsh"], "$env:AVM_SHIM_DIR", "function global:avm")]
    fn shell_init_emits_setup(
        #[case] args: &[&str],
        #[case] shim_dir: &str,
        #[case] function: &str,
    ) {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(args))
            .succeeds()
            .stdout_contains(shim_dir)
            .stdout_contains(function)
            .stdout_contains("shims install");
    }

    /// With no shell named, `shell-init` targets the platform's own shell.
    ///
    /// 1. Run `avm shell-init`.
    /// 2. POSIX platforms get the sh setup, Windows gets PowerShell.
    #[test]
    fn shell_init_defaults_to_the_platform_shell() {
        let p = Sandbox::new("avm-test");
        let (shim_dir, function) = if cfg!(windows) {
            ("$env:AVM_SHIM_DIR", "function global:avm")
        } else {
            ("AVM_SHIM_DIR=", "avm()")
        };
        assert_that(&p.run(&["shell-init"]))
            .succeeds()
            .stdout_contains(shim_dir)
            .stdout_contains(function);
    }

    /// `cmd` has no init hook and says what to do instead.
    ///
    /// 1. Run `avm shell-init cmd`.
    /// 2. It fails pointing at `avm-bin env --shell cmd`.
    #[test]
    fn cmd_has_no_init_hook() {
        let p = Sandbox::new("avm-test");
        failure(p.run(&["shell-init", "cmd"]), "avm-bin env --shell cmd");
    }

    /// An unknown shell is a usage error.
    ///
    /// 1. Run `avm shell-init fish`.
    /// 2. clap exits 2 listing the possible values.
    #[test]
    fn an_unknown_shell_is_a_usage_error() {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(&["shell-init", "fish"]))
            .exits_with(2)
            .stderr_contains("possible values");
    }

    /// The generated POSIX script is valid shell for bash and zsh.
    ///
    /// 1. Syntax-check `avm shell-init` output with `bash -n` and `zsh -n`.
    /// 2. Both accept it (zsh is skipped when not installed).
    #[rstest]
    #[case("bash")]
    #[case("zsh")]
    fn the_generated_script_is_valid_shell(#[case] shell: &str) {
        let p = Sandbox::new("avm-test");
        let script = success(p.run(&["shell-init", "sh"]));
        p.write("init.sh", &script);
        let file = p.path("init.sh");
        let out = std::process::Command::new(shell)
            .arg("-n")
            .arg(&file)
            .output();
        match out {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            out => {
                assert_that(&out.unwrap()).succeeds();
            }
        }
    }
}
