// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

// disable unused code warnings for dev builds
// #![cfg_attr(debug_assertions, allow(dead_code, unused_imports,
// unused_variables))]

extern crate env_logger;
#[macro_use]
extern crate log;

use std::io::Write;

use anyhow::Result;
use clap::Parser;
use itertools::Itertools;

mod cli;
mod consensus;
mod env;
mod extract;
mod io;
mod summary;
mod utils;

use cli::{get_about_label, get_version_label, Cli, Commands};

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
    let real_time = start.elapsed().as_secs();
    info!("real time: {real_time}")
}

fn try_main() -> Result<()> {
    let start = std::time::Instant::now();

    // parse the CLI
    let cli = Cli::parse();

    // initialise logger
    let default_level = if cli.debug { "debug" } else { "info" };

    // format: local 24-hour time in brackets, then the message. the level tag is
    // omitted for INFO and printed after the bracket otherwise. every line of a
    // multi-line record is prefixed, so continuation lines don't dangle.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_level))
        .format(move |buf, record| {
            let time = chrono::Local::now().format("%H:%M:%S");

            let tag = if record.level() == log::Level::Info {
                String::new()
            } else {
                let style = buf.default_level_style(record.level());
                format!("{style}{:<5}{style:#} ", record.level())
            };

            let message = record.args().to_string();

            for line in message.lines() {
                if line.is_empty() {
                    // no trailing whitespace on spacer lines
                    writeln!(buf, "[{time}]")?;
                } else {
                    writeln!(buf, "[{time}] {tag}{line}")?;
                }
            }

            // `lines()` yields nothing for an empty message, but the record was
            // still emitted, so give it a line of its own
            if message.is_empty() {
                writeln!(buf, "[{time}]")?;
            }

            Ok(())
        })
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
        Commands::Version => {
            use spoa::{AlignmentEngine, AlignmentType};
            println!("{}", get_about_label());

            let mut alignment_engine =
                AlignmentEngine::new(AlignmentType::kOV, 5, -4, -8, -6, -10, -4);
            let engine_type = alignment_engine.alignment_engine_type();

            println!("Alignment engine type: {engine_type}");

            Ok(())
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
