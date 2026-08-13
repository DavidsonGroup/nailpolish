---
title: nailpolish summary
---

# nailpolish summary

Quickly review the quality and duplicate rate of the dataset.
The reads must first have been [indexed](./index.md).

## Usage

```shell
$ nailpolish summary --help
Generate a summary of duplicate statistics from an index file

Usage: nailpolish summary [OPTIONS] <INPUT>

Arguments:
  <INPUT>  Input .fastq file

Options:
  -o, --output <OUTPUT>  Output .html file. By default, will write to <file>.summary.html
  -h, --help             Print help
```

## Output

A brief summary of the dataset is printed to the terminal:

```
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
[16:17:49]   Average quality:           21.2
[16:17:49]   Average length:            1030.6
```

`Total read count` is every read in the file: those which were indexed under a barcode,
plus those which had no barcode and were skipped. Reads without barcodes are only
possible if the index was built with
[`--skip-unmatched`](./index.md#reading-the-index) — without that flag, `nailpolish
index` errors on the first read it cannot match.

The full report, including the distribution of duplicate group sizes, is written to the
HTML file.

[See an example summary output file.](../assets/summary.html)

<iframe src="../../assets/summary.html" style="width: 100%; height: 60vh; min-height: 500px;"></iframe>