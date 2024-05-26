mod cargo;
mod command_ext;
mod git;
mod update_all;
mod upgrade;

use clap::Parser;
use update_all::UpdateAllArgs;
use upgrade::UpgradeArgs;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    subcommand: Subcommand,
}

#[derive(clap::Subcommand)]
enum Subcommand {
    UpdateAll(UpdateAllArgs),
    Upgrade(UpgradeArgs),
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.subcommand {
        Subcommand::UpdateAll(update_all_args) => update_all::update_all(update_all_args),
        Subcommand::Upgrade(upgrade_one_args) => upgrade::upgrade_one(upgrade_one_args),
    }
}
