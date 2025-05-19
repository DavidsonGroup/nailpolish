# Quick Start


## Warning

**This guide was written for Nailpolish v0.1.0 and so is considered depreciated. Commands listed here likely will not work.**

This quick start guide will walk you through installing Nailpolish and running it on a small demo dataset.
The demo dataset is a small subset of the _scmixology2_ Chromium 10x droplet-based dataset, sequenced using
Nanopore technology, released by [Tian et al. (2021)](https://doi.org/10.1186/s13059-021-02525-6).

Our Flexiplex tool is used to demultiplex the dataset.

## Install

_For more information, see [Install](./install.md)._

For x64 Linux, run:

```shell
curl --proto '=https' --tlsv1.2 -LsSf "https://github.com/DavidsonGroup/nailpolish/releases/download/nightly_develop/nailpolish" -o nailpolish
chmod +x nailpolish
```

### Get test files

Download the `scmixology2` subset reads using:

```shell
wget https://github.com/DavidsonGroup/nailpolish/releases/download/sample-fastq-for-quickstart/scmixology2_sample.fastq
```

## Indexation

_For more information, see [nailpolish index](commands/generate-index.md)._

By default, nailpolish expects the barcode and UMI to be in the `@BC_UMI` format at the start of the header.
Alternative barcode and UMI formats can be provided through either a preset (one of `bc-umi`, `umi-tools`, `illumina`)
or a custom barcode regex.

```shell
# write the index file to `index.tsv`
nailpolish index --index index.tsv scmixology2_sample.fastq
```

## Summary of duplicate count

_For more information, see [nailpolish summary](commands/summarize.md)._

A .html file can be generated to summarise some key statistics about the input reads.
The output file is written to `summary.html` by default.

```shell
nailpolish summary index.tsv
```

![nailpolish summary](./assets/summary_image.png)

## Consensus call duplicates

_For more information, see [nailpolish call](commands/call.md)._

By consensus calling duplicates, only one read is returned per UMI group.
For singleton reads, there is no change
(apart from including UMI group information in the header).

```shell
nailpolish call \
  --index index.tsv \
  --input scmixology2_sample.fastq \
  --output scmixology2_sample_consensus_called.fastq \
  --threads 4 
```

There are alternative parameters which can be passed to configure the output.
See the _[nailpolish call](commands/call.md)_ documentation for more.