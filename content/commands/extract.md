---
title: nailpolish extract
---

# nailpolish extract

Retrieve the original unmodified reads within duplicate groups that match a predicate.

## Usage
```bash
$ nailpolish extract --help
Extract reads belonging to groups that match a predicate

Usage: nailpolish extract [OPTIONS] <INPUT>

Arguments:
  <INPUT>  the input .fastq

Options:
  -o, --output <OUTPUT>          the output .fastq, or empty for stdout
      --id <ID>                  Filter by specific group IDs (comma-separated)
      --key <KEY>                Filter by regex pattern for the key
      --group-size <GROUP_SIZE>  Filter by the size of the duplicate group
      --read-nums <READ_NUMS>    Choose a subset of reads by index within a group (comma-separated)
      --format <FORMAT>          Output format type [default: fastq] [possible values: fastq, fasta, metadata]
  -h, --help                     Print help
```

## Predicates
These are mutually exclusive predicates i.e. only one can be given at a time.

- `--id`: A comma-separated list (i.e. 5 or 5,6,7) of group IDs
- `--key`: a regular expression matching the BC_UMI key
- `--group-size`: the number of reads in the duplicate group.
  This is equivalent to the total number of reads called in all the clusters of a group.
  For example, the below group (with two clusters) has a group size of 5.

```bash
@GATAGCTAGCAACAAT_ATTTTACCGACC|id=12047|type=consensus|cluster=1|reads_called=2
#└────────────key────────────┘      └─id
@GATAGCTAGCAACAAT_ATTTTACCGACC|id=12047|type=consensus|cluster=2|reads_called=3
```

## Other options

- `--read-nums`: select a subset of reads by their index within each matching group
  (comma-separated, e.g. `1,2`). Can be combined with any predicate.
- `--format`: output format. Options are `fastq` (default), `fasta`, or `metadata`
  (tab-separated metadata only, without sequence data).