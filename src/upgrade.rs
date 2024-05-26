use std::{path::PathBuf, process::Command, str::FromStr as _};

use anyhow::Context;
use semver::{Op, Version, VersionReq};
use walkdir::WalkDir;

use crate::{
    cargo::{self, DepKind},
    command_ext::CommandExt as _,
    git,
};

#[derive(clap::Args, Debug)]
pub struct UpgradeArgs {
    /// Exclude "Cargo.lock" files or containing directories.
    ///
    /// Must be specified relative to the current working directory.
    #[arg(long)]
    exclude: Vec<String>,

    /// Run `cargo check` after applying upgrades.
    #[arg(long)]
    check: bool,

    /// Name of crates to upgrade.
    dep_crate_names: Vec<String>,
}

pub fn upgrade_one(upgrade_args: &UpgradeArgs) -> anyhow::Result<()> {
    // TODO: Factor out a bunch of this stuff that's common
    // to both subcommands.

    // Validate that all exclude rules point to actual paths.
    // (It's bad to let people think that their arguments are doing something if they're not!)
    let mut exclude_paths = Vec::new();
    for exclude in &upgrade_args.exclude {
        let exclude_path =
            PathBuf::from_str(exclude).with_context(|| "\"{exclude:?}\" isn't a valid path")?;
        if !exclude_path.exists() {
            anyhow::bail!("Excluded path {exclude_path:?} doesn't exist!");
        }
        exclude_paths.push(
            exclude_path
                .canonicalize()
                .with_context(|| format!("Failed to canonicalize exclude path {exclude_path:?}"))?,
        );
    }

    // TODO: Find git root by default instead of just operating from CWD.
    // (Have option for operating just within CWD.)

    if !git::is_working_tree_clean().context("Failed to check if working tree is clean")? {
        anyhow::bail!("Working tree is not clean; please commit or stash your changes first.");
    }

    let latest_versions = cargo::get_latest_versions(&upgrade_args.dep_crate_names)
        .context("Failed to get latest versions for requested crates")?;

    // Update all the projects we can find!

    let base_branch = git::guess_base_branch().context("Failed to guess base branch")?;
    git::fetch(&base_branch).context("Failed to update base branch from origin")?;

    // Make a branch based on the current time.
    let compact_now = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let new_branch_name = format!("cargo-lockstep-upgrade-{compact_now}");
    git::switch_to_new_branch(&new_branch_name, &format!("origin/{base_branch}"))
        .context("Failed to create branch for applying upgrades")?;

    // Find all the Cargo.toml files so we can upgrade all of the requested deps.
    println!("Looking for \"Cargo.toml\" files...");
    let mut any_changes = false;
    for entry in WalkDir::new(".").follow_links(false).into_iter() {
        // TODO: More helpful error.
        let entry = entry.context("Couldn't read dir entry")?;

        let file_name = entry.file_name().to_string_lossy();
        if file_name != "Cargo.toml" {
            continue;
        }

        let dir = entry
            .path()
            .parent()
            .context("Cargo.toml file didn't have a parent directory")?;

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

        println!("  Looking for dependencies to upgrade in {dir:?}...");

        // TODO: Switch to only using `cargo metadata --no-deps` for this.
        // And then ignore directories that don't actually have any packages
        // directly in them.
        let Ok(manifest) = cargo::read_manifest(dir) else {
            eprintln!("Failed to read manifest of project to check for available upgrades; assuming it is a virtual manifest");
            continue;
        };

        for dep in &manifest.dependencies {
            // TODO: Make a hashset for checking this.
            if !upgrade_args.dep_crate_names.contains(&dep.name) {
                // We're not trying to upgrade this.
                continue;
            }

            // Prepare a candiate version based on what we found above.
            let Some(candidate_version) = latest_versions.get(&dep.name) else {
                eprintln!(
                    "Warning: didn't find {:?} in latest versions; this shouldn't happen.",
                    dep.name
                );
                continue;
            };

            // REVISIT: Should we null out the patch level? I'm in two minds about that...

            let version_req = VersionReq::parse(&dep.req)
                .context("Failed to parse version requirement from manifest")?;

            if version_req.comparators.len() > 1 {
                eprintln!(
                    "Warning: multiple comparators found in version requirement for {:?}; skipping",
                    dep.name
                );
                continue;
            }

            let comparator = version_req
                .comparators
                .first()
                .context("Missing comparator in version requirement")?;

            if comparator.op != Op::Caret {
                eprintln!(
                    "Warning: comparator in version requirement for {:?} was not a 'caret'; skipping",
                    dep.name
                );
                continue;
            }

            // Convert the requirement to a version, and see if the candidate is newer.
            let version = Version {
                major: comparator.major,
                minor: comparator.minor.unwrap_or(0),
                patch: comparator.patch.unwrap_or(0),
                pre: comparator.pre.clone(),
                build: semver::BuildMetadata::EMPTY,
            };

            if version >= *candidate_version {
                println!(
                    "{:?} is already on its newest normal release. Nothing to do!",
                    dep.name
                );
                continue;
            }

            let extra_args = match dep.kind {
                Some(DepKind::Build) => vec!["--build"],
                Some(DepKind::Dev) => vec!["--dev"],
                None => vec![],
            };

            // Bump the dependency to the candidate version.
            cargo::add(
                dir,
                &format!("{}@{}", dep.name, candidate_version),
                &extra_args,
            )
            .context("Failed to update dependency version")?;
        }

        any_changes = true;
    }

    if !any_changes {
        println!("All specified dependencies were already on their latest versions!");
    }

    // Now to a pass to update lockfiles and maybe run a `cargo check`.
    println!("Looking for \"Cargo.lock\" files...");
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

        // `cargo metadata` forces dependency resolution, so we can run it
        // instead of requesting an update of individual dependencies.
        let _metadata = cargo::metadata(dir, false)
            .context("Failed to run `cargo metadata` to resolve dependencies")?;

        if upgrade_args.check {
            println!("  Running `cargo check --all-targets` in {dir:?}...");
            let mut cmd = Command::new("cargo");
            cmd.args(["check", "--all-targets"]).current_dir(dir);
            cmd.success_or_err().context("`cargo check` failed")?;
        }
    }

    println!("    Committing updates...");
    // Heuristic for making a commit summary line that's useful but not too long.
    let mut commit_message: String = match &upgrade_args.dep_crate_names[..] {
        [first, second] => format!("Upgrade {first} and {second} crates"),
        [first, second, rest @ ..] => {
            format!("Upgrade {first}, {second} and {} other crates", rest.len())
        }
        [one] => format!("Upgrade {one} crate"),
        [] => anyhow::bail!("No crates were specified to be upgraded"),
    };

    commit_message += "\n\nThese crates were upgraded:\n\n";
    for crate_name in &upgrade_args.dep_crate_names {
        let latest_version = latest_versions
            .get(crate_name)
            .context("Missing latest version for crate")?;
        commit_message += &format!("- {crate_name}@{latest_version}\n");
    }

    commit_message += "\nThis commit was created by `cargo-lockstep`.\n";

    git::commit(&commit_message).context("Failed to commit changes")?;

    println!("Upgrades applied! You can now push this branch and make a pull-request.");

    Ok(())
}
