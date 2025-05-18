// disable unused code warnings for dev builds
// #![cfg_attr(debug_assertions, allow(dead_code, unused_imports,
// unused_variables))]

extern crate env_logger;
#[macro_use]
extern crate log;

use anyhow::Result;
use clap::Parser;

mod cli;
mod consensus;
mod env;
mod extract;
mod io;
mod summary;
mod utils;

use cli::{get_version_label, Cli, Commands};
use itertools::Itertools;

/// Report resource usage statistics
#[cfg(unix)]
fn report_rstats(start: std::time::Instant) {
    let (maxrss_raw, cpu_time) = unsafe {
        let mut out: libc::rusage = std::mem::zeroed();
        libc::getrusage(libc::RUSAGE_SELF, &mut out);
        // maxrss: maximum resident set size in KB
        (out.ru_maxrss, out.ru_utime.tv_sec + out.ru_stime.tv_sec)
    };

    let real_time = start.elapsed().as_secs();

    // check for OS type: https://doc.rust-lang.org/std/env/consts/constant.OS.html
    let maxrss_b = if std::env::consts::OS == "macos" {
        // on macos, maxrss is in b
        maxrss_raw
    } else {
        // on linux, maxrss is in kb
        maxrss_raw * 1024
    };

    let format_opts = humansize::FormatSizeOptions::from(humansize::BINARY)
        .units(humansize::Kilo::Decimal)
        .decimal_places(3);

    let maxrss = humansize::format_size_i(maxrss_b, format_opts);

    info!("real time: {real_time} sec; CPU: {cpu_time} sec; peak RSS: {maxrss}")
}

#[cfg(windows)]
fn report_rstats(start: std::time::Instant) {
    // Only reports real time on Windows
    let real_time = start.elapsed.as_secs();
    info!("real time: {real_time}")
}

fn try_main() -> Result<()> {
    let start = std::time::Instant::now();

    // parse the CLI
    let cli = Cli::parse();

    // initialise logger
    let default_level = if cli.debug { "debug" } else { "info" };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_level))
        .format_target(false)
        .init();

    // start with version information, if a command has been run
    info!("{}", get_version_label());
    info!("CMD: {}", std::env::args().join(" "));

    debug!("Called with parameters:\n{:?}", cli);

    match &cli.command {
        Commands::Summary(args) => summary::summarize(args),
        Commands::Index(args) => {
            io::index::construct::construct_index(args).map(|_| report_rstats(start))
        }
        Commands::Consensus(args) => consensus::consensus(args).map(|_| report_rstats(start)),
        Commands::Extract(args) => extract::extract(args),
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
