mod command_ext;
mod git;
mod update_all;

use clap::Parser;
use update_all::UpdateAllArgs;

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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.subcommand {
        Subcommand::UpdateAll(update_all_args) => update_all::update_all(update_all_args),
    }
}
