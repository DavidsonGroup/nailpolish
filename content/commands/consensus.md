---
title: nailpolish consensus
---

# nailpolish consensus

Consensus call duplicated reads.
The reads must first have been [indexed](./index.md).
By default, reads within each duplicate group will be clustered to eliminate false duplicates.

## Usage

```shell
$ nailpolish consensus --help
Generate a consensus-called 'cleaned up' file

Usage: nailpolish consensus [OPTIONS] <INPUT>

Arguments:
  <INPUT>  the input .fastq

Options:
  -o, --output <OUTPUT>              the output .fastq, or empty for stdout
  -t, --threads <THREADS>            the number of threads to use [default: 4]
      --report-original-reads        for each duplicate group of reads, report the original reads along with the consensus
      --report-original-header       if the original read headers are valuable, this will create a orig_header field
                                     in the consensus called result with the entire original read header
      --extra-stats                  add debugging information to the read header [intended for internal development]
                                     warning: since timings are reported, the output will not be identical across runs
      --no-clustering                disable the clustering algorithm; this will prevent nailpolish from detecting
                                     and separating false duplicates
      --len <LEN>                    filter lengths to a value within the given float interval [a,b] [default: 0,15000]
      --qual <QUAL>                  filter average read quality to a value within the given float interval [a,b] [default: 0,inf]
      --max-group-size <N>           skip consensus calling for groups larger than this size [default: 250]
      --sort-by <TAG>                sort groups by the specified capture group tag (e.g. 'CB' for cell barcode)
  -h, --help                         Print help
```

## Output format

A `.fastq` file will be produced. Read headers carry metadata as SAM auxiliary tags in the FASTQ
comment field (tab-separated, after the read name). See the [Output format reference](../reference/output-format.md)
for a complete description of all tags.

A typical output looks like this (tabs shown as newlines for clarity):

```
@processed_12047_1
  MI:Z:GATAGCTAGCAACAAT_ATTTTACCGACC
  nI:i:12047
  CB:Z:GATAGCTAGCAACAAT
  UB:Z:ATTTTACCGACC
  nT:Z:consensus
  nC:i:1
  nL:i:2
```

## Options

- `--threads`: set the number of threads that _nailpolish_ should use
- `--report-original-reads`: report the original reads as well as the consensus read
- `--report-original-header`: report the original headers of the reads used to produce
  a consensus
- `--no-clustering`: disable the false duplicate detection algorithm (see below)
- `--len <LEN>`: filter reads by sequence length. Reads outside the interval are excluded
  from consensus calling. Default: `0,15000` (reads longer than 15,000 bp are excluded,
  as excessively long reads from sequencing errors can dominate consensus calling time).
- `--qual <QUAL>`: filter reads by average base quality. Default: `0,inf` (no quality filter).
- `--max-group-size <N>`: skip consensus calling for groups larger than `N` reads (default: 250).
  Very large groups are typically caused by false duplicates; skipping them prevents an outsized
  impact on runtime.
- `--sort-by <TAG>`: sort groups by the named capture group tag before output
  (e.g. `--sort-by CB` to sort by cell barcode).

## False duplicate detection

By default, _nailpolish_ clusters reads within each duplicate group to detect and separate
_false duplicates_ — reads that share a barcode/UMI by coincidence rather than by biology.
Before adding each read to a partial order alignment graph, nailpolish checks whether the read
aligns well to the existing graph. If the alignment introduces too many new nodes relative to
existing ones (more than 25% of valid nodes), the read is assigned to a new cluster rather than
merged into the current one.

This produces multiple consensus sequences for a single group when the reads are insufficiently
similar. Each cluster is reported separately in the output with an incrementing `cluster=` field.

To disable this behaviour — for example, when you are confident that all reads in a group are
true duplicates, or when using pre-clustered inputs from a tool like isONclust — pass
`--no-clustering`. This provides a small performance benefit and guarantees a single consensus
per group.

```bash
# default: false duplicate detection enabled
nailpolish consensus reads.fastq -o output.fastq

# disabled: one consensus per group, no splitting
nailpolish consensus --no-clustering reads.fastq -o output.fastq
```

!!! note
    The false duplicate detection was designed for reads that share no biological similarity.
    For pre-clustered inputs from tools with relaxed clustering criteria (e.g. isONclust),
    many loosely similar clusters may still pass through as a single consensus.
    Whether to use `--no-clustering` depends on your confidence in the upstream clustering.