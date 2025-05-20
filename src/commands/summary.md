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

[See an example summary output file.](../assets/summary.html)

<iframe src="../assets/summary.html" style="width: 100%; height: 60vh; min-height: 500px;"></iframe>