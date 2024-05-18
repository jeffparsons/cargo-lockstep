use std::process::Command;

use crate::command_ext::CommandExt;

pub fn is_working_tree_clean() -> anyhow::Result<bool> {
    let mut cmd = Command::new("git");
    cmd.args(["diff", "--exit-code"]).null_io();
    let exit_code = cmd.clean_exit_code()?;
    if exit_code == 0 {
        // Working tree is clean.
        Ok(true)
    } else if exit_code == 1 {
        // There are changes in the working tree.
        Ok(false)
    } else {
        anyhow::bail!("Unrecognised exit code {exit_code} from `{cmd:?}`")
    }
}

pub fn guess_base_branch() -> anyhow::Result<String> {
    // Modern Git default
    let main_branch_exists = branch_exists("main")?;
    // Legacy Git default
    let master_branch_exists = branch_exists("master")?;

    if main_branch_exists && master_branch_exists {
        anyhow::bail!(r#"Both "main" and "master" branches exist"#);
    } else if !main_branch_exists && !master_branch_exists {
        anyhow::bail!(r#"Neither "main" nor "master" branch exists"#);
    } else if main_branch_exists {
        Ok("main".to_string())
    } else {
        Ok("master".to_string())
    }
}

pub fn fetch(branch_name: &str) -> anyhow::Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(["fetch", "origin", branch_name]).null_io();
    cmd.success_or_err()
}

pub fn switch_to_new_branch(new_branch_name: &str, start_point: &str) -> anyhow::Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(["checkout", "-b", new_branch_name, start_point])
        .null_io();
    cmd.success_or_err()
}

pub fn commit(message: &str) -> anyhow::Result<()> {
    let mut cmd = Command::new("git");
    // TODO: It would be safer to not pass '-a' here;
    // I'd prefer to add exactly what we intend to commit
    // and then blow up if there was anything else not staged.
    cmd.args(["commit", "-a", "-m", message]).null_io();
    cmd.success_or_err()
}

fn branch_exists(branch_name: &str) -> anyhow::Result<bool> {
    let mut cmd = Command::new("git");
    cmd.args(["show-branch", branch_name]).null_io();
    Ok(cmd.clean_exit_status()?.success())
}
