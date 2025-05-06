use clap::builder::styling::AnsiColor;
use clap::builder::Styles;
use clap::{value_parser, Args, Parser, Subcommand};
use std::path::PathBuf;

pub mod interval;
use interval::ArgInterval;

pub mod preset;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const COMMIT: &str = match option_env!("CARGO_BUILD_DESC") {
    Some(e) => e,
    None => "",
};
const INFO_STRING: &str = "
💅 nailpolish version ";
const AFTER_STRING: &str = "
   ──────────────────────────────────
   tools for consensus calling barcode and UMI duplicates
   https://github.com/olliecheng/nailpolish";

// colouring of the help
const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default().bold())
    .usage(AnsiColor::BrightMagenta.on_default().bold())
    .literal(AnsiColor::BrightMagenta.on_default())
    .placeholder(AnsiColor::White.on_default());

// TODO: version label dynamic
pub fn get_version_label() -> String {
    format!("{}{}{}", INFO_STRING, VERSION, COMMIT)
}

#[derive(Parser)]
#[command(
    version = VERSION,
    about = format!("{}{}{}{}", INFO_STRING, VERSION, COMMIT, AFTER_STRING),
    arg_required_else_help = true,
    flatten_help = true,
    styles = STYLES
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create an index file from a demultiplexed .fast2q
    #[command(arg_required_else_help = true)]
    Index {
        /// the input .fastq file
        #[arg(value_parser = value_parser!(PathBuf))]
        file: PathBuf,

        #[arg(value_enum, conflicts_with = "barcode_regex", default_value = "bc-umi")]
        preset: preset::PresetBarcodeFormats,

        /// whether to use a file containing pre-clustered reads, with every line in one of two formats:
        ///   1. READ_ID;BARCODE
        ///   2. READ_ID;BARCODE;UMI
        #[arg(long, verbatim_doc_comment, value_parser = value_parser!(PathBuf), conflicts_with = "preset")]
        clusters: Option<PathBuf>,

        /// barcode regex format type, for custom header styles. this will override the preset given.
        /// for example, for the `bc-umi` preset:
        ///     ^([ATCG]{16})_([ATCG]{12})
        #[arg(long, verbatim_doc_comment, conflicts_with_all = ["clusters", "preset"])]
        barcode_regex: Option<String>,

        /// skip, instead of error, on reads which are not accounted for:
        /// - if a cluster file is passed, any reads which are not in any cluster
        /// - if a barcode regex or preset is used (default), any reads which do not match the regex
        #[arg(long, verbatim_doc_comment)]
        skip_unmatched: bool,

        /// filter lengths to a value within the given float interval [a,b].
        /// a is the minimum, and b is the maximum (both inclusive).
        /// alternatively, a can be `-inf` and b can be `inf.
        /// an unbounded interval (i.e. no length filter) is given by `0,inf`.
        #[arg(
            long,
            value_parser = |x: &str| ArgInterval::try_from(x),
            default_value = "0,15000",
            verbatim_doc_comment
        )]
        len: ArgInterval,

        /// filter average read quality to a value within the given float interval [a,b].
        /// see the docs for `--len` for documentation on how to use the interval.
        #[arg(
            long,
            value_parser = |x: &str| ArgInterval::try_from(x),
            default_value = "0,inf",
            verbatim_doc_comment
        )]
        qual: ArgInterval,
    },

    /// Generate a summary of duplicate statistics from an index file
    #[command(arg_required_else_help = true)]
    Summary {
        /// the index file
        #[arg(long)]
        index: String,

        /// output file
        #[arg(short, default_value = "summary.html")]
        output: String,
    },

    /// Generate a consensus-called 'cleaned up' file
    #[command(arg_required_else_help = true)]
    Call(CallArgs),

    /// Tag each read by its UMI group, and write to a .fastq file. Due to the large amounts of
    /// random file access required, this may take a while.
    #[command(arg_required_else_help = true)]
    Group {
        /// the index file
        #[arg(long)]
        index: String,

        #[arg(long)]
        input: String,

        #[arg(short)]
        output: Option<String>,
    },
}

#[derive(Debug, Args)]
pub struct CallArgs {
    /// the input .fastq
    pub input: PathBuf,

    /// the output .fastq
    #[arg(short)]
    pub output: PathBuf,

    /// the number of threads to use
    #[arg(short, long, default_value_t = 4)]
    pub threads: usize,

    /// only show the duplicated reads, not the single ones
    #[arg(long, action)]
    pub duplicates_only: bool,

    /// for each duplicate group of reads, report the original reads along with the consensus
    #[arg(long, action)]
    pub report_original_reads: bool,

    /// if the original read headers are valuable, this will create a orig_header field in the consensus called result with the entire original read header
    #[arg(long, action)]
    pub report_original_header: bool,

    /// add debugging information to the read header [intended for internal development]
    /// warning: since timings are reported, the output will not be identical across runs
    #[arg(long, action)]
    pub debugging_header: bool,
}
