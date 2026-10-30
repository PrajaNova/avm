use std::{
    io,
    process::{Child, Command},
};

/// Kills remaining descendants on drop, including those holding output pipes.
pub struct ProcessTree {
    /// Invariant: the id of a process group this process created with
    /// `process_group(0)` in `spawn_isolated`, and `pid > 1`. Only
    /// `spawn_isolated` constructs it. Semantic hazard, not memory safety: once
    /// the group has no members left, the id may be recycled, and the kill in
    /// `drop` could then reach an unrelated group.
    #[cfg(unix)]
    pid: i32,
    /// Invariant: a job object handle owned by this value, closed exactly once
    /// in `drop`; only `attach_job` constructs it.
    #[cfg(windows)]
    job: windows_sys::Win32::Foundation::HANDLE,
}

/// Spawns `cmd` in its own process group (Unix) or job object (Windows) so the whole tree can be killed on timeout.
pub fn spawn_isolated(cmd: &mut Command) -> io::Result<(Child, ProcessTree)> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
        let mut child = cmd.spawn()?;
        // Keeps the `ProcessTree::pid` invariant: a pid that fits `i32` and is
        // never 0 or 1, so `-pid` can only name a real process group.
        let Some(pid) = i32::try_from(child.id()).ok().filter(|pid| *pid > 1) else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other("child pid out of range"));
        };
        Ok((child, ProcessTree { pid }))
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use windows_sys::Win32::System::Threading::CREATE_SUSPENDED;
        // Assign the job before the child can spawn any descendants.
        cmd.creation_flags(CREATE_SUSPENDED);
        let mut child = cmd.spawn()?;
        match attach_job(&child) {
            Ok(tree) => Ok((child, tree)),
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                Err(err)
            }
        }
    }
}

impl Drop for ProcessTree {
    fn drop(&mut self) {
        // Negative target: the process group `spawn_isolated` created for the child.
        #[cfg(unix)]
        sigkill(-self.pid);
        #[cfg(windows)]
        // SAFETY:
        // Operation: `CloseHandle(self.job)`.
        // Contract (Win32 `CloseHandle`): the argument must be an open handle
        // this process may close; closing it twice is an error.
        // Evidence: `ProcessTree::job` is a valid job handle owned solely by this
        // value (its field invariant), and `drop` runs at most once.
        // Postcondition: the handle is closed; with KILL_ON_JOB_CLOSE this kills
        // every process still in the job. Nothing reads `self.job` afterwards.
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.job);
        }
    }
}

/// Sends SIGKILL to `target`: a pid, or a negated process group id.
/// The caller picks the target, so which process is signalled is its concern.
#[cfg(unix)]
fn sigkill(target: i32) {
    // SAFETY:
    // Operation: `libc::kill(target, SIGKILL)`.
    // Contract (POSIX `kill`): takes two integers and no pointers; it can fail
    // (reported through the ignored return value) but has no memory-safety
    // preconditions, and never touches Rust-managed memory.
    // Postcondition: none for Rust state. The signalled process may terminate;
    // `ESRCH`/`EPERM` are deliberately ignored because the target may already be gone.
    unsafe {
        libc::kill(target, libc::SIGKILL);
    }
}

/// Include nested timed commands that have their own Unix process groups.
#[cfg(unix)]
pub(crate) fn terminate_descendants(pid: u32) {
    let Ok(output) = Command::new("/bin/ps")
        .args(["-axo", "pid=,ppid="])
        .output()
    else {
        return;
    };
    let pairs: Vec<(u32, u32)> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some((fields.next()?.parse().ok()?, fields.next()?.parse().ok()?))
        })
        .collect();
    let mut descendants = vec![pid];
    let mut at = 0;
    while at < descendants.len() {
        for &(child, parent) in &pairs {
            if parent == descendants[at] && !descendants.contains(&child) {
                descendants.push(child);
            }
        }
        at += 1;
    }
    // The pids come from `ps` text, so they are untrusted: a value over
    // `i32::MAX` would wrap negative (a process group), and 0 or 1 name our own
    // group or init. Only pids above 1 that fit `i32` are signalled. The
    // snapshot can be stale: a listed pid may be recycled before this runs.
    for child in descendants.into_iter().rev().filter(|child| *child != pid) {
        if let Some(child) = i32::try_from(child).ok().filter(|child| *child > 1) {
            sigkill(child);
        }
    }
}

/// Assigns `child` to a new Windows job object that kills its processes on close, returning the handle wrapper.
#[cfg(windows)]
fn attach_job(child: &Child) -> io::Result<ProcessTree> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::{
        Foundation::*,
        System::{Diagnostics::ToolHelp::*, JobObjects::*, Threading::*},
    };
    // SAFETY (whole block; each call below is justified where it appears):
    // all pointers passed are either null where the API documents null as
    // "default", or point to live locals that outlive the call; every handle
    // is checked against its API's failure sentinel before use and closed on
    // every path.
    unsafe {
        // `CreateJobObjectW(null, null)`: null attributes/name are documented
        // as "default security, unnamed job". Failure is NULL, checked next.
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            return Err(io::Error::last_os_error());
        }
        // From here `tree` owns `job`: any early return closes it (and, with
        // KILL_ON_JOB_CLOSE, kills the suspended child).
        let tree = ProcessTree { job };
        // `mem::zeroed` on JOBOBJECT_EXTENDED_LIMIT_INFORMATION: a plain C struct
        // of integers and raw pointers, for which all-zero is a valid value.
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // `SetInformationJobObject` needs a valid job handle (`job`, checked
        // non-null) and a pointer/size pair for the struct the information class
        // names: `&limits` and `size_of_val(&limits)` describe the live local.
        // `AssignProcessToJobObject` needs valid job and process handles:
        // `child.as_raw_handle()` is valid while `child` is borrowed here.
        if SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &limits as *const _ as _,
            std::mem::size_of_val(&limits) as u32,
        ) == 0
            || AssignProcessToJobObject(job, child.as_raw_handle()) == 0
        {
            return Err(io::Error::last_os_error());
        }
        // `CreateToolhelp32Snapshot`: flags and pid are plain integers; failure is
        // INVALID_HANDLE_VALUE, checked next, and the handle is closed below.
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let result = (|| {
            // Zeroed THREADENTRY32 is a valid plain C struct; `dwSize` is set as
            // `Thread32First`/`Thread32Next` require, and `snapshot` is a valid
            // snapshot handle until `CloseHandle(snapshot)` after this closure.
            let mut entry: THREADENTRY32 = std::mem::zeroed();
            entry.dwSize = std::mem::size_of_val(&entry) as u32;
            let mut found = Thread32First(snapshot, &mut entry);
            while found != 0 {
                if entry.th32OwnerProcessID == child.id() {
                    // `child` was created suspended, so its only thread is the
                    // one found here. `OpenThread` failure is NULL (checked);
                    // `ResumeThread` failure is u32::MAX (checked), and the OS
                    // error is read before `CloseHandle` can overwrite it.
                    let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID);
                    if thread.is_null() {
                        return Err(io::Error::last_os_error());
                    }
                    let resumed = ResumeThread(thread);
                    let error = io::Error::last_os_error();
                    CloseHandle(thread);
                    return if resumed == u32::MAX {
                        Err(error)
                    } else {
                        Ok(())
                    };
                }
                found = Thread32Next(snapshot, &mut entry);
            }
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "suspended child thread not found",
            ))
        })();
        CloseHandle(snapshot);
        result?;
        Ok(tree)
    }
}
