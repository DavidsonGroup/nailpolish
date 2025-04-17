use clap::builder::styling::AnsiColor;
use clap::builder::Styles;
use clap::{value_parser, Parser, Subcommand};
use std::path::PathBuf;

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
        #[arg(value_paser = value_parser!(PathBuf))]
        file: PathBuf,

        #[arg(value_enum, conflicts_with = "barcode_regex", default_value = "bc-umi")]
        preset: crate::preset::PresetBarcodeFormats,

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
    Call {
        /// the index file
        #[arg(long)]
        index: String,

        /// the input .fastq
        #[arg(long)]
        input: String,

        /// the output .fastq
        #[arg(short)]
        output: Option<String>,

        /// the number of threads to use
        #[arg(short, long, default_value_t = 4)]
        threads: usize,

        /// only show the duplicated reads, not the single ones
        #[arg(short, long, action)]
        duplicates_only: bool,

        /// for each duplicate group of reads, report the original reads along with the consensus
        #[arg(short, long, action)]
        report_original_reads: bool,
    },

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

#[derive(Copy, Clone, Debug)]
pub struct ArgInterval {
    pub min: f64,
    pub max: f64,
}

/// Error type for parsing an interval string.
#[derive(Debug)]
pub struct ParseIntervalErr(String);

impl std::fmt::Display for ParseIntervalErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid interval format: {}", self.0)
    }
}

impl std::error::Error for ParseIntervalErr {}

impl<'a> TryFrom<&'a str> for ArgInterval {
    type Error = ParseIntervalErr;

    fn try_from(arg: &'a str) -> Result<ArgInterval, Self::Error> {
        let arg_lc = arg.to_lowercase();
        let parts: Vec<&str> = arg_lc.split(',').collect();

        if parts.len() != 2 {
            return Err(ParseIntervalErr(indoc::formatdoc! {"
            Expected format '<min>,<max>', got '{arg}'. The expected format is \
            `a,b`, as in:
              --len 0,15000
              --len 0,inf
              --len 100,15000
            "}));
        }

        // Try to parse the minimum and maximum, handling unbounded cases.
        let min = match parts[0].trim() {
            "-inf" => f64::NEG_INFINITY,
            s => s.parse::<f64>().map_err(|_| {
                ParseIntervalErr(format!(
                    "Invalid minimum value: '{}' (should be any float or `-inf`)",
                    parts[0].trim()
                ))
            })?,
        };

        let max = match parts[1].trim() {
            "inf" => f64::INFINITY,
            s => s.parse::<f64>().map_err(|_| {
                ParseIntervalErr(format!(
                    "Invalid maximum value: '{}' (should be any float or `inf`)",
                    parts[1].trim()
                ))
            })?,
        };

        Ok(ArgInterval { min, max })
    }
}

impl ArgInterval {
    pub fn contains(&self, v: f64) -> bool {
        let v = v as f64;
        (self.min < v) && (v < self.max)
    }
}
