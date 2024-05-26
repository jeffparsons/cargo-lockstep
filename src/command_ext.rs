use std::process::{Command, ExitStatus, Output, Stdio};

use anyhow::Context;

pub trait CommandExt {
    fn null_io(&mut self) -> &mut Command;
    fn clean_exit_status(&mut self) -> anyhow::Result<ExitStatus>;
    fn clean_exit_code(&mut self) -> anyhow::Result<i32>;
    fn success_or_err(&mut self) -> anyhow::Result<()>;
    fn clean_output(&mut self) -> anyhow::Result<Output>;
    fn output_if_success_else_err(&mut self) -> anyhow::Result<Output>;
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

    fn clean_output(&mut self) -> anyhow::Result<Output> {
        self.output()
            .with_context(|| format!("Failed to start `{:?}`", self))
    }

    fn output_if_success_else_err(&mut self) -> anyhow::Result<Output> {
        let output = self.clean_output()?;
        if !output.status.success() {
            anyhow::bail!("`{self:?}` returned a nonzero exit code");
        }
        Ok(output)
    }
}
