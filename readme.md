# 💅 nailpolish

[![Build status](https://github.com/olliecheng/nailpolish/actions/workflows/build.yml/badge.svg)](https://github.com/olliecheng/nailpolish/actions/workflows/build.yml) ![Static Badge](https://img.shields.io/badge/libc-%E2%89%A5%202.17-blue) [![GitHub Release](https://img.shields.io/github/v/release/olliecheng/nailpolish?include_prereleases)](https://github.com/DavidsonGroup/nailpolish/tags)

`nailpolish` is a software tool made for the deduplication of UMI and barcodes when working with long read single cell data.

<div align="center">
  <a href="#install">Install</a> &nbsp;&nbsp; | &nbsp;&nbsp; <a href="#example">Example</a> &nbsp;&nbsp; | &nbsp;&nbsp; <a href="#usage">Usage</a> &nbsp;&nbsp; | &nbsp;&nbsp; <a href="https://davidsongroup.github.io/nailpolish/">Docs</a>

</div>

## Install

`nailpolish` is distributed as a single binary with no dependencies (beyond libc).
Up-to-date builds are available through the
[Releases](https://github.com/DavidsonGroup/nailpolish/releases/tag/nightly_develop)
section for macOS (Intel & Apple Silicon) and x64-based Linux systems.

**Releases:**
[macOS](https://github.com/DavidsonGroup/nailpolish/releases/download/nightly_develop/nailpolish-macos-universal),
[Linux](https://github.com/DavidsonGroup/nailpolish/releases/download/nightly_develop/nailpolish)

`nailpolish` is in active development. If you are running into any issues, please check to ensure that you are using
the most current version of the software!

For detailed usage instructions, see the [documentation](https://davidsongroup.github.io/nailpolish/).

## Example

Say I have a demultiplexed `sample.fastq` file of the following form—for instance, one generated using
the [Flexiplex demultiplexer](https://github.com/DavidsonGroup/flexiplex):

```bash
# original file has BC/UMI duplicates
$ head -n 4 sample.fastq
@BC1_UMI1
...
@BC2_UMI2
...
@BC1_UMI1
...

# index the file
$ nailpolish index sample.fastq

# summarise duplicate rates in an accessible format
$ nailpolish summary sample.fastq
<creates HTML summary file>

# generate consensus file
$ nailpolish consensus sample.fastq | head
  @BC1_UMI1|type=consensus|
  ...
  @BC2_UMI2|type=single|
  ...
```

Consensus generation will output all non-duplicated and consensus called reads, removing all the original duplicated reads in the process.

There are options to:
- Set the filtering options to determine which duplicate groups should be called
- Configure the output format and what information to report
- Control the false positive prevention algorithm

See the [documentation](https://davidsongroup.github.io/nailpolish/) for more information.

## Usage

### Help

```
nailpolish v0.2.0, commit #9ddba6c
──────────────────────────────────
tools for finding, grouping, and consensus calling PCR duplicates

git:  https://github.com/DavidsonGroup/nailpolish
docs: https://davidsongroup.github.io/nailpolish/


Usage: nailpolish index [OPTIONS] <INPUT> [PRESET]
       nailpolish summary [OPTIONS] <INPUT>
       nailpolish consensus [OPTIONS] <INPUT>
       nailpolish extract [OPTIONS] <INPUT>
       nailpolish help [COMMAND]...

Options:
  -h, --help     Print help
  -V, --version  Print version

nailpolish index:
Create an index file from a demultiplexed .fastq
      --overwrite                      overwrite an existing index file, if it exists
      --clusters <CLUSTERS>            whether to use a file containing pre-clustered reads, with every line in one of two formats:
                                         1. READ_ID;BARCODE
                                         2. READ_ID;BARCODE;UMI
      --barcode-regex <BARCODE_REGEX>  barcode regex format type, for custom header styles. this will override the preset given.
                                       for example, for the `bc-umi` preset:
                                           ^([ATCG]{16})_([ATCG]{12})
      --skip-unmatched                 skip, instead of error, on reads which are not accounted for:
                                       - if a cluster file is passed, any reads which are not in any cluster
                                       - if a barcode regex or preset is used (default), any reads which do not match the regex
      --len <LEN>                      filter lengths to a value within the given float interval [a,b].
                                       a is the minimum, and b is the maximum (both inclusive).
                                       alternatively, a can be `-inf` and b can be `inf.
                                       an unbounded interval (i.e. no length filter) is given by `0,inf`. [default: 0,15000]
      --qual <QUAL>                    filter average read quality to a value within the given float interval [a,b].
                                       see the docs for `--len` for documentation on how to use the interval. [default: 0,inf]
  -h, --help                           Print help (see more with '--help')
  <INPUT>                          the input .fastq file
  [PRESET]                         [default: bc-umi] [possible values: bc-umi, umi-tools, illumina]

nailpolish summary:
Generate a summary of duplicate statistics from an index file
  -o, --output <OUTPUT>  Output .html file. By default, will write to <file>.summary.html
  -h, --help             Print help
  <INPUT>            Input .fastq file

nailpolish consensus:
Generate a consensus-called 'cleaned up' file
  -o, --output <OUTPUT>         the output .fastq, or empty for stdout
  -t, --threads <THREADS>       the number of threads to use [default: 4]
      --duplicates-only         only show the duplicated reads, not the single ones
      --report-original-reads   for each duplicate group of reads, report the original reads along with the consensus
      --report-original-header  if the original read headers are valuable, this will create a orig_header field in the consensus called result with the entire original read header
      --extra-stats             add debugging information to the read header [intended for internal development] warning: since timings are reported, the output will not be identical across runs
      --no-clustering           disable the clustering algorithm this will prevent nailpolish from detecting and separating false duplicates
  -h, --help                    Print help
  <INPUT>                   the input .fastq

nailpolish extract:
Extract reads beloning to specific group queries a .fastq file, unmodified
  -o, --output <OUTPUT>          the output .fastq, or empty for stdout
      --id <ID>                  Filter by specific group IDs (comma-separated)
      --key <KEY>                Filter by regex pattern for the key
      --group-size <GROUP_SIZE>  Filter by the size of the duplicate group
      --format <FORMAT>          Output format type [default: fastq] [possible values: fastq, fasta]
  -h, --help                     Print help
  <INPUT>                    the input .fastq

nailpolish help:
Print this message or the help of the given subcommand(s)
  [COMMAND]...  Print help for the subcommand(s)
```

## Install from source

### Prebuilt binaries

The recommended way to download Nailpolish is to use the automated builds, which can be found in the
[Releases](https://github.com/DavidsonGroup/nailpolish/releases/tag/nightly_develop)
section for macOS (Intel + Apple Silicon) and x64 Linux systems.

### Install from source

You will need a modern version of Rust installed on your machine, as well as the Cargo package manager. That's it - all
package installations will be done automatically at the build stage.
This will install `nailpolish` into your local `PATH`.

```sh
$ cargo install --git https://github.com/DavidsonGroup/nailpolish.git

# or, from a local directory
$ cargo install --path .
```

#### Note to HPC users on older systems

You will need a reasonably modern version of `gcc` and `cmake` installed, and the `CARGO_NET_GIT_FETCH_WITH_CLI` flag
enabled. For instance:

```
$ module load gcc/latest cmake/latest
$ CARGO_NET_GIT_FETCH_WITH_CLI="true" cargo install --git https://github.com/DavidsonGroup/nailpolish.git
```

### Build from source

```sh
$ git clone https://github.com/DavidsonGroup/nailpolish.git
$ cargo build --release
```

The binary can be found at `/target/release/nailpolish`.
