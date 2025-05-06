// disable unused code warnings for dev builds
// #![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables))]

extern crate env_logger;
#[macro_use]
extern crate log;
use std::{
    fs::File,
    io::{prelude::*, stdout, BufWriter},
    path::Path,
};

use anyhow::Result;
use clap::Parser;

mod cli;
mod consensus;
mod group;
mod io;
mod summary;

#[macro_use]
mod utils;

use cli::{get_version_label, preset, Cli, Commands};
use io::index::filter;

/// Creates a `BufWriter` for the given output option. This allows for an output file to be passed
/// or otherwise will default to using standard output.
///
/// If `output` is `Some`, it creates a file at the specified path and returns a `BufWriter` for it.
/// If `output` is `None`, it returns a `BufWriter` for the standard output.
///
/// # Arguments
///
/// * `output` - An `Option` containing the path to the output file as a `String`.
///
/// # Returns
///
/// A `Result` containing a `BufWriter` that implements `Write`.
fn get_writer(output: &Option<String>) -> Result<impl Write> {
    // get output as a BufWriter - equal to stdout if None
    let writer = BufWriter::new(match output {
        Some(ref x) => {
            let file = File::create(Path::new(x))?;
            Box::new(file) as Box<dyn Write + Send>
        }
        None => Box::new(stdout()) as Box<dyn Write + Send>,
    });
    Ok(writer)
}

fn try_main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_target(false)
        .init();

    let cli = Cli::parse();

    println!("nailpolish version {}", get_version_label());

    match &cli.command {
        Commands::Summary { index, output } => {
            summary::summarize(index, output)?;
        }
        Commands::Index {
            file,
            preset,
            barcode_regex,
            clusters,
            skip_unmatched,
            len,
            qual,
        } => {
            let barcode_regex = match barcode_regex {
                Some(v) => {
                    info!("Using specified barcode format: {v}");
                    v.clone()
                }
                None => {
                    let regex = preset::get_barcode_regex(preset);
                    info!("Using preset barcode format {regex}");
                    regex
                }
            };

            let filter_opts = filter::FilterOpts {
                len: *len,
                quality: *qual,
            };

            let barcode_location = match clusters {
                None => io::index::construct::BarcodeLocation::Regex(barcode_regex),
                Some(p) => io::index::construct::BarcodeLocation::ClusterFile(p.clone()),
            };

            io::index::construct::construct_index(
                file,
                barcode_location,
                *skip_unmatched,
                filter_opts,
            )?;

            info!("Completed index generation to index file... TODO fix");
        }
        Commands::Call(cli) => {
            consensus::consensus(cli);

            info!("Completed successfully.")
        }
        Commands::Group {
            index,
            input,
            output,
        } => {
            todo!();
        }
    };
    Ok(())
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
