use std::process::{Command, ExitStatus, Stdio};

use anyhow::Context;

pub trait CommandExt {
    fn null_io(&mut self) -> &mut Command;
    fn clean_exit_status(&mut self) -> anyhow::Result<ExitStatus>;
    fn clean_exit_code(&mut self) -> anyhow::Result<i32>;
    fn success_or_err(&mut self) -> anyhow::Result<()>;
}

impl CommandExt for Command {
    fn null_io(&mut self) -> &mut Command {
        self.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        self
    }

    fn clean_exit_status(&mut self) -> anyhow::Result<ExitStatus> {
        self.status()
            .with_context(|| format!("Failed to start `{:?}`", self))
    }

    fn clean_exit_code(&mut self) -> anyhow::Result<i32> {
        self.clean_exit_status()?
            .code()
            .with_context(|| format!("`{self:?}` was terminated by a signal"))
    }

    fn success_or_err(&mut self) -> anyhow::Result<()> {
        if !self.clean_exit_status()?.success() {
            anyhow::bail!("`{self:?}` returned a nonzero exit code");
        }
        Ok(())
    }
}
