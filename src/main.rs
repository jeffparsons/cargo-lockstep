mod command_ext;
mod git;

use std::{path::PathBuf, process::Command, str::FromStr};

use anyhow::Context;
use clap::Parser;
use git::switch_to_new_branch;
use walkdir::WalkDir;

use crate::{command_ext::CommandExt, git::is_working_tree_clean};

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    subcommand: Subcommand,
}

#[derive(clap::Subcommand)]
enum Subcommand {
    UpdateAll(UpdateAllArgs),
}

#[derive(clap::Args)]
struct UpdateAllArgs {
    /// Exclude "Cargo.lock" files or containing directories.
    ///
    /// Must be specified relative to the current working directory.
    #[arg(long)]
    exclude: Vec<String>,

    /// Run `cargo check` after applying updates.
    #[arg(long)]
    check: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.subcommand {
        Subcommand::UpdateAll(update_all_args) => {
            // Validate that all exclude rules point to actual paths.
            // (It's bad to let people think that their arguments are doing something if they're not!)
            let mut exclude_paths = Vec::new();
            for exclude in &update_all_args.exclude {
                let exclude_path = PathBuf::from_str(exclude)
                    .with_context(|| "\"{exclude:?}\" isn't a valid path")?;
                if !exclude_path.exists() {
                    anyhow::bail!("Excluded path {exclude_path:?} doesn't exist!");
                }
                exclude_paths.push(exclude_path.canonicalize().with_context(|| {
                    format!("Failed to canonicalize exclude path {exclude_path:?}")
                })?);
            }

            // TODO: Find git root by default instead of just operating from CWD.
            // (Have option for operating just within CWD.)

            if !git::is_working_tree_clean().context("Failed to check if working tree is clean")? {
                anyhow::bail!(
                    "Working tree is not clean; please commit or stash your changes first."
                );
            }

            let base_branch = git::guess_base_branch().context("Failed to guess base branch")?;
            git::fetch(&base_branch).context("Failed to update base branch from origin")?;

            // Make a branch based on the current time.
            let compact_now = chrono::Utc::now().format("%Y%m%d%H%M%S");
            let new_branch_name = format!("cargo-lockstep-update-all-{compact_now}");
            switch_to_new_branch(&new_branch_name, &base_branch)
                .context("Failed to create branch for applying updates")?;

            // Find all the Cargo lockfiles so we can run `cargo update` in those directories.
            println!("Looking for \"Cargo.lock\" files...");
            let mut any_changes = false;
            for entry in WalkDir::new(".").follow_links(false).into_iter() {
                // TODO: More helpful error.
                let entry = entry.context("Couldn't read dir entry")?;

                let file_name = entry.file_name().to_string_lossy();
                if file_name != "Cargo.lock" {
                    continue;
                }

                let dir = entry
                    .path()
                    .parent()
                    .context("Cargo lockfile didn't have a parent directory")?;

                let absolute_path = entry
                    .path()
                    .canonicalize()
                    .with_context(|| format!("Failed to canonicalize path {:?}", entry.path()))?;
                if exclude_paths
                    .iter()
                    .any(|exclude_path| absolute_path.starts_with(exclude_path))
                {
                    println!("  Skipping {dir:?} because it matches an excluded path.");
                    continue;
                }

                println!("  Running `cargo update` in {dir:?}...");

                let mut cmd = Command::new("cargo");
                cmd.arg("update").current_dir(dir);
                // TODO: Don't blow up the whole process if we fail in here.
                cmd.success_or_err().context("`cargo update` failed")?;
                if is_working_tree_clean()? {
                    println!("    Already up-to-date!");
                    continue;
                }

                any_changes = true;

                if update_all_args.check {
                    println!("  Running `cargo check` in {dir:?}...");
                    let mut cmd = Command::new("cargo");
                    cmd.arg("check").current_dir(dir);
                    cmd.success_or_err().context("`cargo check` failed")?;
                }

                println!("    Committing updates...");
                let message = format!("cargo update in {dir:?}\n\nAll semver-compatible-updates, by running `cargo update`.\nThis commit was created by `cargo-lockstep`.");
                git::commit(&message).context("Failed to commit changes")?;
            }

            if !any_changes {
                println!("All \"Cargo.lock\" files were already up-to-date!");
            }

            println!("Updates applied! You can now push this branch and make a pull-request.");
        }
    }

    Ok(())
}
