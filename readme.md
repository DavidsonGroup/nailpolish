# 💅 nailpolish

[![Build status](https://github.com/olliecheng/nailpolish/actions/workflows/build.yml/badge.svg)](https://github.com/olliecheng/nailpolish/actions/workflows/build.yml) ![Static Badge](https://img.shields.io/badge/libc-%E2%89%A5%202.17-blue) [![GitHub Release](https://img.shields.io/github/v/release/olliecheng/nailpolish?include_prereleases)](https://github.com/DavidsonGroup/nailpolish/tags)

Nailpolish is a high-performance Rust tool designed to improve the accuracy of sequencing data by error correcting PCR duplicates.

Nailpolish identifies PCR duplicates in barcoded data (reads containing identical barcodes and UMIs, forming "duplicate groups") and applies the partial order alignment consensus algorithm to replace multiple duplicate reads with a single consensus error-corrected read. This process corrects sequencing errors which naturally occur in the reads, improving the overall quality and reliability of sequencing data.

Nailpolish operates in a reference-free manner, first identifying duplicate groups and then clustering within each duplicate group. This process ensures that only true duplicates are included in consensus calling. That is, unrelated reads that share barcodes and UMIs (due to read or demultiplexing errors) are not consensus called together, and are instead separated into separate subgroups.

For a detailed description of Nailpolish's methodology and how it compares against other tools, please see our [preprint](https://doi.org/10.64898/2026.09.25.754331).

<div align="center">
  <a href="#install">Install</a> &nbsp;&nbsp; | &nbsp;&nbsp; <a href="#output">Output</a> &nbsp;&nbsp; | &nbsp;&nbsp; <a href="https://davidsongroup.github.io/nailpolish/">Docs</a>
</div>
<br />
<img src="https://raw.githubusercontent.com/DavidsonGroup/nailpolish/refs/heads/develop/docs/content/assets/consensus_diagram.svg">

## Install

Nailpolish is distributed as a single binary with no dependencies (beyond libc).
Up-to-date builds are available through the
[Releases](https://github.com/DavidsonGroup/nailpolish/releases/tag/nightly)
section for macOS (Intel & Apple Silicon) and x64-based Linux systems.

**Releases:**
[Latest](https://github.com/DavidsonGroup/nailpolish/releases/latest),
[Nightly](https://github.com/DavidsonGroup/nailpolish/releases/tag/nightly)

Detailed installation instructions are available in the [documentation](https://davidsongroup.github.io/nailpolish/install/).

## Output


**An example showing how to use Nailpolish can be found in the [Quick Start](https://davidsongroup.github.io/nailpolish/quickstart/).**

Consensus generation will output all non-duplicated and consensus called reads, removing all the original duplicated reads in the process.

There are options to:
- Set the filtering options to determine which duplicate groups should be called
- Configure the output format and what information to report
- Control the false positive prevention algorithm

See the [documentation](https://davidsongroup.github.io/nailpolish/) for more information.

## Install from source

### Prebuilt binaries

The recommended way to download Nailpolish is to use the automated builds, which can be found in the
[Releases](https://github.com/DavidsonGroup/nailpolish/releases/tag/nightly)
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

**For robots 🤖**: Detailed documentation about each command, as well as usage guides, can be found at the agents-friendly [llms.md page](https://davidsongroup.github.io/nailpolish/llms.md).
