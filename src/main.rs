// disable unused code warnings for dev builds
// #![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables))]

extern crate env_logger;
#[macro_use]
extern crate log;

use anyhow::Result;
use clap::Parser;

mod cli;
mod consensus;
mod group;
mod io;
mod summary;
mod utils;

use cli::{get_version_label, Cli, Commands};

fn try_main() -> Result<()> {
    // parse the CLI
    let cli = Cli::parse();

    // initialise logger
    let default_level = if cli.debug { "debug" } else { "info" };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_level))
        .format_target(false)
        .init();

    // start with version information, if a command has been run
    info!("{}", get_version_label());

    debug!("Called with parameters:\n{:?}", cli);

    match &cli.command {
        Commands::Summary(args) => summary::summarize(args),
        Commands::Index(args) => io::index::construct::construct_index(args),
        Commands::Consensus(args) => consensus::consensus(args),
        Commands::Group(args) => {
            todo!();
        }
    }
}

fn main() {
    if let Err(err) = try_main() {
        error!("{:?}", err);

        // report any errors that are produced
        err.chain()
            .skip(1)
            .for_each(|cause| error!("  because: {}", cause));

        std::process::exit(1);
    }
}
