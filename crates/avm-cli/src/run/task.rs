use anyhow::Result;

/// Something avm can run. Commands are built once by `build`, then wrapped
/// (see `wrappers`) with behavior that depends on the command or its flags.
pub trait Runnable {
    /// Runs the command, consuming it.
    fn run(self: Box<Self>) -> Result<()>;

    /// Whether to print the "new avm available" notice afterwards. It is for
    /// people at a terminal: not shims, eval'd output or the alias hot path.
    fn wants_update_notice(&self) -> bool {
        true
    }
}

pub type Cmd = Box<dyn Runnable>;

/// Adapts a closure (the existing `cmd_*` functions) to `Runnable`.
pub struct Task {
    run: Box<dyn FnOnce() -> Result<()>>,
    notice: bool,
}

impl Task {
    /// A command that prints the update notice afterwards.
    pub fn cmd(run: impl FnOnce() -> Result<()> + 'static) -> Cmd {
        Box::new(Self {
            run: Box::new(run),
            notice: true,
        })
    }

    /// A command whose output is consumed by a shell or tool, so no notice.
    pub fn quiet(run: impl FnOnce() -> Result<()> + 'static) -> Cmd {
        Box::new(Self {
            run: Box::new(run),
            notice: false,
        })
    }
}

impl Runnable for Task {
    fn run(self: Box<Self>) -> Result<()> {
        (self.run)()
    }

    fn wants_update_notice(&self) -> bool {
        self.notice
    }
}
