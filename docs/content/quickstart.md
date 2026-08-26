---
title: Quick Start
---

# Quick Start

This quick start guide will walk you through installing Nailpolish and running it on a small demo dataset.
The demo dataset is a small subset of the _scmixology2_ Chromium 10x droplet-based dataset, sequenced using
Nanopore technology, released by [Tian et al. (2021)](https://doi.org/10.1186/s13059-021-02525-6).

Our Flexiplex tool is used to demultiplex the dataset.

## Install

_For more information, see [Install](./install.md)._

For x64 Linux, run:

```shell
curl -LsSf "https://github.com/DavidsonGroup/nailpolish/releases/download/latest/nailpolish" -o nailpolish
chmod +x nailpolish
```

Download the example `scmixology2_sample` dataset using:

```shell
wget "https://github.com/DavidsonGroup/nailpolish/releases/download/sample-fastq-for-quickstart/scmixology2_sample.fastq"
```

## Indexation

_For more information, see [nailpolish index](commands/index.md)._

By default, nailpolish expects the barcode and UMI to be in the `@BC_UMI` format at the start of the header.
Alternative barcode and UMI formats can be provided through either a preset (one of `bc-umi`, `umi-tools`, `illumina`)
or a custom barcode regex.

```console
$ nailpolish index scmixology2_sample.fastq

[16:17:08] nailpolish v0.2.2, commit #65a177c-modified
[16:17:08] CMD: ../target/release/nailpolish index scmixology2_sample.fastq
[16:17:08] Indexing 28.9MB from scmixology2_sample.fastq
[16:17:08]
[16:17:08]    bytes        %
[16:17:08]   28.9MB   100.0%
[16:17:08]
[16:17:08] Statistics:
[16:17:08]   14,143 reads in total
[16:17:08]   14,143 reads with barcodes
[16:17:08]   0 reads without barcodes
[16:17:08] completed in 0.0s runtime
[16:17:08] Writing to scmixology2_sample.fastq.nailpolish.idx...
[16:17:08] real time: 0 sec; CPU: 0 sec; peak RSS: 10.984 MB
```

## Summary of duplicate count

_For more information, see [nailpolish summary](commands/summary.md)._

A .html file can be generated to summarise some key statistics about the input reads.
The output file will be written to `scmixology2_sample.summary.html`.

```console
$ nailpolish summary scmixology2_sample.fastq

[16:17:49] nailpolish v0.2.2, commit #65a177c-modified
[16:17:49] CMD: ../target/release/nailpolish summary scmixology2_sample.fastq
[16:17:49] Summarising scmixology2_sample.fastq → scmixology2_sample.summary.html
[16:17:49] Summary written to scmixology2_sample.summary.html. In brief:
[16:17:49]   Nailpolish version:        nailpolish v0.2.2, commit #65a177c-modified
[16:17:49]   File path:                 scmixology2_sample.fastq
[16:17:49]   Dataset size:              28.9 MB
[16:17:49]   Index date:                2026-08-13T16:17:08.927358+10:00
[16:17:49]   Total read count:          14,143
[16:17:49]   Reads with barcodes:       14,143
[16:17:49]   Reads without barcodes:    0
[16:17:49]   Singleton groups:          10,855
[16:17:49]   Duplicate groups:          1,310   (3,288 reads)
[16:17:49]   Average quality:           21.2
[16:17:49]   Average length:            1030.6
```


Here is an example summary output file. [Open in a new tab...](./assets/summary.html)
<iframe src="../assets/summary.html" style="width: 100%; height: 60vh; min-height: 500px;"></iframe>

## Consensus call duplicates

_For more information, see [nailpolish consensus](commands/consensus.md)._

```console
$ nailpolish consensus scmixology2_sample.fastq \
  -o scmixology2_consensus.fastq \
  --threads 4 

[16:22:21] nailpolish v0.2.2, commit #65a177c-modified
[16:22:21] CMD: ../target/release/nailpolish consensus scmixology2_sample.fastq -o scmixology2_consensus.fastq --threads 4
[16:22:21] Consensus calling scmixology2_sample.fastq → scmixology2_consensus.fastq
[16:22:21] Using thread pool with 4 threads
[16:22:21] Calling consensus on 0 reads across 12,164 groups
[16:22:21]
[16:22:21]                      ┌── input groups ───┐   ┌─ false duplicates ─┐
[16:22:21]     reads        %   singleton   duplicate   detections   avg/group    filtered
[16:22:25]    14,143   100.0%      10,855       1,310          150        0.11           1
[16:22:25]
[16:22:25] Complete. Final counts, after false-duplicate detection:
[16:22:25]   input reads          14143
[16:22:25]   singleton groups     11063
[16:22:25]   duplicate groups     1252   (3079 reads)
[16:22:25]   filtered reads       1
[16:22:25] real time: 3 sec; CPU: 13 sec; peak RSS: 1.219 GB
```

There are alternative parameters which can be passed to configure the output.
See the _[nailpolish consensus](commands/consensus.md)_ documentation for more.