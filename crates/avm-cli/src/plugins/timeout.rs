use super::*;

/// Run `cmd` with piped output and return its stdout, killing the process tree after `timeout_ms`.
/// Errors on spawn failure, timeout (including descendants holding the pipes open), or non-zero exit (carrying stderr).
pub(super) fn run_with_timeout(mut cmd: Command, timeout_ms: u64) -> Result<String> {
    let started = std::time::Instant::now();
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let (mut child, tree) = avm_plugin_sdk::process::spawn_isolated(&mut cmd)
        .context("failed to start plugin command")?;

    // Pipes have a small OS buffer (~64KB). Waiting for the child to exit
    // before reading them deadlocks the moment output exceeds that: the
    // child blocks mid-write with nobody draining the pipe, so it never
    // exits, so the wait never returns Some — it just times out. Drain
    // both pipes on their own threads *while* waiting, so arbitrarily
    // large output never blocks the child.
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();
    let stdout_reader = std::thread::spawn(move || {
        let mut buf = String::new();
        if let Some(mut pipe) = stdout_pipe {
            let _ = pipe.read_to_string(&mut buf);
        }
        buf
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut buf = String::new();
        if let Some(mut pipe) = stderr_pipe {
            let _ = pipe.read_to_string(&mut buf);
        }
        buf
    });

    let status =
        wait_deadline(&mut child, timeout_ms).context("failed while waiting for plugin command")?;
    drop(tree);
    // Detached Unix descendants can leave the group; they must not extend our deadline.
    while !stdout_reader.is_finished() || !stderr_reader.is_finished() {
        if started.elapsed().as_millis() >= u128::from(timeout_ms) {
            return Err(anyhow!("plugin command timed out"));
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let status = match status {
        Some(status) => status,
        None => {
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(anyhow!("plugin command timed out"));
        }
    };

    let out = stdout_reader.join().unwrap_or_default();
    let err = stderr_reader.join().unwrap_or_default();

    if !status.success() {
        if !err.trim().is_empty() {
            return Err(anyhow!("plugin command failed: {err}"));
        }
        return Err(anyhow!("plugin command failed with exit code"));
    }

    Ok(out)
}

#[cfg(test)]
mod timeout_tests {
    use super::*;
    #[test]
    fn descendants_cannot_hold_plugin_pipes_open() {
        let started = std::time::Instant::now();
        #[cfg(unix)]
        let mut command = Command::new("sh");
        #[cfg(unix)]
        command.args(["-c", "sleep 20 & wait"]);
        #[cfg(windows)]
        let mut command = Command::new("powershell");
        #[cfg(windows)] command.args(["-NoProfile", "-Command", "$p = Start-Process ping -ArgumentList '-n','20','127.0.0.1' -NoNewWindow -PassThru; Wait-Process -Id $p.Id"]);
        assert!(run_with_timeout(command, 1000)
            .unwrap_err()
            .to_string()
            .contains("timed out"));
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        #[cfg(unix)]
        {
            let mut command = Command::new("sh");
            command.args(["-c", "sleep 20 & printf ready"]);
            assert_eq!(run_with_timeout(command, 1000).unwrap(), "ready");
        }
    }
}
