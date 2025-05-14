# 💅 nailpolish

[![Build status](https://github.com/olliecheng/nailpolish/actions/workflows/build.yml/badge.svg)](https://github.com/olliecheng/nailpolish/actions/workflows/build.yml) ![Static Badge](https://img.shields.io/badge/libc-%E2%89%A5%202.17-blue) [![GitHub Release](https://img.shields.io/github/v/release/olliecheng/nailpolish?include_prereleases)](https://github.com/DavidsonGroup/nailpolish/tags)

`nailpolish` is a collection of tools made for the deduplication of UMIs when working with long read single cell data.

<div align="center">
  <a href="#install">Install</a> &nbsp;&nbsp; | &nbsp;&nbsp; <a href="#example">Example</a> &nbsp;&nbsp; | &nbsp;&nbsp; <a href="#usage">Usage</a>
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

## Example

Say I have a demultiplexed `sample.fastq` file of the following form—for instance, one generated using
the [Flexiplex demultiplexer](https://github.com/DavidsonGroup/flexiplex):

```
@BC1_UMI1
sequence...
+
quality...
```

I first create an _index_ file using

```sh
$ nailpolish index --file sample.fastq --output index.tsv
```

I can view summary statistics about duplicate rates using:

```sh
$ nailpolish summary --index index.tsv
```

and I can also transparently remove duplicate reads using:

```sh
$ nailpolish consensus \
  --index index.tsv \
  --input sample.fastq \
  --output sample_called.fastq \
  --threads 4
```

which will output all non-duplicated and consensus called reads, removing all the original duplicated reads in the
process.

## Usage

### Help

```
💅 nailpolish version 0.1.0
   ──────────────────────────────────
   tools for consensus calling barcode and UMI duplicates
   https://github.com/DavidsonGroup/nailpolish

Usage: nailpolish generate-index [OPTIONS] --file <FILE>
       nailpolish summary --index <INDEX>
       nailpolish call [OPTIONS] --index <INDEX> --input <INPUT>
       nailpolish group [OPTIONS] --index <INDEX> --input <INPUT> [COMMAND]...
       nailpolish help [COMMAND]...

Options:
  -h, --help     Print help
  -V, --version  Print version

nailpolish generate-index:
Create an index file from a demultiplexed .fastq, if one doesn't already exist
      --file <FILE>    the input .fastq file
      --index <INDEX>  the output index file [default: index.tsv]
  -h, --help           Print help

nailpolish summary:
Generate a summary of duplicate statistics from an index file
      --index <INDEX>  the index file
  -h, --help           Print help

nailpolish call:
Generate a consensus-called 'cleaned up' file
      --index <INDEX>          the index file
      --input <INPUT>          the input .fastq
      --output <OUTPUT>        the output .fasta; note that quality values are not preserved
  -t, --threads <THREADS>      the number of threads to use [default: 4]
  -d, --duplicates-only        only show the duplicated reads, not the single ones
  -r, --report-original-reads  for each duplicate group of reads, report the original reads along with the consensus
  -h, --help                   Print help

nailpolish group:
'Group' duplicate reads, and pass to downstream applications
      --index <INDEX>      the index file
      --input <INPUT>      the input .fastq
      --output <OUTPUT>    the output location, or default to stdout
      --shell <SHELL>      the shell used to run the given command [default: bash]
  -t, --threads <THREADS>  the number of threads to use. this will not guard against race conditions in any downstream applications used. this will effectively set the number of individual processes to launch [default: 1]
  -h, --help               Print help
  [COMMAND]...         the command to run. any groups will be passed as .fastq standard input [default: cat]

nailpolish help:
Print this message or the help of the given subcommand(s)
  [COMMAND]...  Print help for the subcommand(s)
```

<details>
<summary>Example of <code>--duplicates-only</code> and <code>--report-original-reads</code></summary>
Suppose I have a demultiplexed read file of the following format (so that <code>seq2</code> and <code>seq3</code> are duplicates):
<pre>
@BCUMI_1
seq1
@BCUMI_2
seq2
@BCUMI_2
seq3
</pre>
Then, the effects of the following flags are:
<pre>
(default):
  >BCUMI_1_SIN
  seq1
  >BCUMI_2_CON_2
  seq2_and_3_consensus
</pre>

<pre>
--duplicates-only:
  >BCUMI_2_CON_2
  seq2_and_3_consensus
</pre>

<pre>
--report-original-reads
  >BCUMI_1_SIN
  seq1
  >BCUMI_2_DUP_1_of_2
  seq2
  >BCUMI_2_DUP_2_of_2
  seq3
  >BCUMI_2_CON_2
  seq2_and_3_consensus
</pre>
</details>

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
