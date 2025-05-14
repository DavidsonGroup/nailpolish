use clap::builder::styling::AnsiColor;
use clap::builder::Styles;
use clap::{Args, Parser, Subcommand};
use git_version::git_version;
use std::path::PathBuf;

pub mod interval;
use interval::ArgInterval;

pub mod preset;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const COMMIT: &str = git_version!(prefix = "#", cargo_prefix = "cargo:#", fallback = "unknown");

// colouring of the help
const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default().bold())
    .usage(AnsiColor::BrightMagenta.on_default().bold())
    .literal(AnsiColor::BrightMagenta.on_default())
    .placeholder(AnsiColor::White.on_default());

pub fn get_version_label() -> String {
    format!("nailpolish v{VERSION}, commit {COMMIT}")
}

pub fn get_about_label() -> String {
    indoc::formatdoc! { "
        nailpolish v{VERSION}, commit {COMMIT}
        ──────────────────────────────────
        tools for finding, grouping, and consensus calling PCR duplicates

        git:  https://github.com/DavidsonGroup/nailpolish
        docs: https://davidsongroup.github.io/nailpolish/
    " }
}

#[derive(Parser)]
#[command(
    version = VERSION,
    about = get_about_label(),
    arg_required_else_help = true,
    flatten_help = true,
    styles = STYLES
)]
#[derive(Debug)]
pub struct Cli {
    /// Print debugging information. Intended for development use.
    #[arg(long, action, hide(true))]
    pub debug: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(arg_required_else_help = true)]
    Index(IndexArgs),

    #[command(arg_required_else_help = true)]
    Summary(SummaryArgs),

    #[command(arg_required_else_help = true)]
    Consensus(ConsensusArgs),

    #[command(arg_required_else_help = true)]
    Extract(ExtractArgs),
}

#[derive(Debug, Args)]
/// Create an index file from a demultiplexed .fastq
pub struct IndexArgs {
    /// the input .fastq file
    pub input: PathBuf,

    /// overwrite an existing index file, if it exists
    #[arg(long, action)]
    pub overwrite: bool,

    #[arg(value_enum, conflicts_with = "barcode_regex", default_value = "bc-umi")]
    pub preset: preset::PresetBarcodeFormats,

    /// whether to use a file containing pre-clustered reads, with every line in one of two formats:
    ///   1. READ_ID;BARCODE
    ///   2. READ_ID;BARCODE;UMI
    #[arg(long, verbatim_doc_comment, conflicts_with = "preset")]
    pub clusters: Option<PathBuf>,

    /// barcode regex format type, for custom header styles. this will override the preset given.
    /// for example, for the `bc-umi` preset:
    ///     ^([ATCG]{16})_([ATCG]{12})
    #[arg(long, verbatim_doc_comment, conflicts_with_all = ["clusters", "preset"])]
    pub barcode_regex: Option<String>,

    /// skip, instead of error, on reads which are not accounted for:
    /// - if a cluster file is passed, any reads which are not in any cluster
    /// - if a barcode regex or preset is used (default), any reads which do not match the regex
    #[arg(long, verbatim_doc_comment)]
    pub skip_unmatched: bool,

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
    pub len: ArgInterval,

    /// filter average read quality to a value within the given float interval [a,b].
    /// see the docs for `--len` for documentation on how to use the interval.
    #[arg(
            long,
            value_parser = |x: &str| ArgInterval::try_from(x),
            default_value = "0,inf",
            verbatim_doc_comment
        )]
    pub qual: ArgInterval,
}

#[derive(Debug, Args)]
/// Generate a summary of duplicate statistics from an index file
pub struct SummaryArgs {
    /// Input .fastq file
    pub input: PathBuf,

    /// Output .html file. By default, will write to <file>.summary.html
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
/// Generate a consensus-called 'cleaned up' file
pub struct ConsensusArgs {
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
    pub extra_stats: bool,
}

/// Extract reads beloning to specific group queries a .fastq file, unmodified.
#[derive(Debug, Args)]
pub struct ExtractArgs {
    /// the input .fastq
    pub input: PathBuf,

    /// the output .fastq
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Filter by specific group IDs (comma-separated)
    #[arg(long, conflicts_with_all = [ "key", "group_size" ])]
    pub id: Option<String>,

    /// Filter by regex pattern for the key
    #[arg(long, conflicts_with_all = [ "id", "group_size" ])]
    pub key: Option<String>,

    #[arg(long, conflicts_with_all = ["id", "key"])]
    pub group_size: Option<usize>,
}
