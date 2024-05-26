use std::{collections::HashMap, path::Path, process::Command};

use anyhow::Context;
use semver::{Version, VersionReq};
use tempfile::tempdir;

use crate::command_ext::CommandExt as _;

pub fn get_latest_versions(crate_names: &[String]) -> anyhow::Result<HashMap<String, Version>> {
    // Make a new Cargo project in a temporary directory.
    // We'll use this to discover the latest version of each
    // of the specified crates.
    let tmp_dir = tempdir().context("Failed to make temporary directory")?;

    let mut cmd = Command::new("cargo");
    // Give it a valid crate name rather than letting it infer a name from the directory.
    cmd.args(["init", "--name", "dummy_for_querying_crate_versions"])
        .null_io()
        .current_dir(tmp_dir.path());
    cmd.success_or_err().context("`cargo init` failed")?;

    // Add all the requested crates.
    let mut cmd = Command::new("cargo");
    cmd.args(["add", "--"])
        .args(crate_names)
        .null_io()
        .current_dir(tmp_dir.path());
    cmd.success_or_err().context("`cargo add` failed")?;

    // Read the manifest to see what versions we ended up with.
    let manifest =
        read_manifest(tmp_dir.path()).context("Failed to read manifest for dummy project")?;

    let mut result = HashMap::new();
    for dep in &manifest.dependencies {
        let version_req = VersionReq::parse(&dep.req)
            .context("Failed to parse version requirement from manifest")?;
        // Turn the version requirement into a version.
        // We make some assumptions about what `cargo add` will put in the file.
        // (Only normal releases, only "caret" requirements.)
        //
        // REVISIT: I should probably validate that it's all what I expect
        // rather than just awkwardly erroring on specific details or ignoring bits.
        let comparator = version_req
            .comparators
            .first()
            .context("Missing comparator in version requirement")?;
        let version = Version::new(
            comparator.major,
            comparator.minor.context("Missing minor version")?,
            comparator.patch.context("Missing patch version")?,
        );
        result.insert(dep.name.to_owned(), version);
    }

    Ok(result)
}

// TODO: Rationalize how you're managing paths.
// Everything should be explicit, and probably just be paths to Cargo.toml or whatever.
pub fn read_manifest(directory: &Path) -> anyhow::Result<Manifest> {
    let mut cmd = Command::new("cargo");
    cmd.args(["read-manifest"]).current_dir(directory);
    let output = cmd
        .output_if_success_else_err()
        .context("`cargo read-manifest` failed")?;
    let manifest: Manifest =
        serde_json::de::from_slice(&output.stdout).context("Failed to deserialize manifest")?;
    Ok(manifest)
}

// TODO: Rationalize how you're managing paths.
// Everything should be explicit, and probably just be paths to Cargo.toml or whatever.
pub fn metadata(directory: &Path, no_deps: bool) -> anyhow::Result<Metadata> {
    let mut cmd = Command::new("cargo");
    cmd.args(["metadata"]).current_dir(directory);
    if no_deps {
        cmd.arg("--no-deps");
    }
    let output = cmd
        .output_if_success_else_err()
        .context("`cargo read-manifest` failed")?;
    let metadata: Metadata =
        serde_json::de::from_slice(&output.stdout).context("Failed to deserialize metadata")?;
    Ok(metadata)
}

pub fn add(directory: &Path, crate_name: &str, extra_args: &[&str]) -> anyhow::Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.args(["add", crate_name])
        .current_dir(directory)
        .null_io();
    cmd.args(extra_args);
    cmd.success_or_err().context("`cargo add` failed")?;
    Ok(())
}

// TODO: Replace this with metadata output

#[derive(serde::Deserialize)]
pub struct Manifest {
    pub dependencies: Vec<Dependency>,
}

#[derive(serde::Deserialize)]
pub struct Dependency {
    pub name: String,
    pub req: String,
    pub kind: Option<DepKind>,
}

#[derive(serde::Deserialize)]
pub struct Metadata {
    pub packages: Vec<Package>,
}

#[derive(serde::Deserialize)]
pub struct Package {
    pub dependencies: Vec<Dependency>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DepKind {
    Dev,
    Build,
}
