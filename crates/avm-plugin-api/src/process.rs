use std::{
    io,
    process::{Child, Command},
};

/// Kills remaining descendants on drop, including those holding output pipes.
pub struct ProcessTree {
    #[cfg(unix)]
    pid: i32,
    #[cfg(windows)]
    job: windows_sys::Win32::Foundation::HANDLE,
}

pub fn spawn_isolated(cmd: &mut Command) -> io::Result<(Child, ProcessTree)> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
        let child = cmd.spawn()?;
        let tree = ProcessTree {
            pid: child.id() as i32,
        };
        Ok((child, tree))
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
        #[cfg(unix)]
        unsafe {
            // spawn_isolated gives this child its own process group.
            libc::kill(-self.pid, libc::SIGKILL);
        }
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.job);
        }
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
    for child in descendants.into_iter().rev().filter(|child| *child != pid) {
        unsafe {
            libc::kill(child as i32, libc::SIGKILL);
        }
    }
}

#[cfg(windows)]
fn attach_job(child: &Child) -> io::Result<ProcessTree> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::{
        Foundation::*,
        System::{Diagnostics::ToolHelp::*, JobObjects::*, Threading::*},
    };
    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            return Err(io::Error::last_os_error());
        }
        let tree = ProcessTree { job };
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
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
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let result = (|| {
            let mut entry: THREADENTRY32 = std::mem::zeroed();
            entry.dwSize = std::mem::size_of_val(&entry) as u32;
            let mut found = Thread32First(snapshot, &mut entry);
            while found != 0 {
                if entry.th32OwnerProcessID == child.id() {
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
