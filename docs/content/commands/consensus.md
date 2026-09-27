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
  <INPUT>
          the input .fastq

Options:
  -o, --output <OUTPUT>
          the output .fastq, or empty for stdout

  -t, --threads <THREADS>
          the number of threads to use
          
          [default: 4]

  -h, --help
          Print help (see a summary with '-h')

Duplicate handling:
      --no-false-duplicate-detection
          disable the clustering algorithm this will prevent nailpolish from detecting and separating false duplicates

      --fdd-threshold <FDD_THRESHOLD>
          threshold for the insert-node ratio used to decide whether an alignment should be clustered (merged) into an existing group, rather than treated as a new group. lower values are stricter (fewer merges)
          
          [default: 0.25]

      --max-group-size <MAX_GROUP_SIZE>
          filter out groups larger than this size (skip consensus calling for very large groups)
          
          this will prevent large groups, which are typically false duplicates, from having an outsized impact on runtime.
          
          [default: 250]

      --large-group-method <LARGE_GROUP_METHOD>
          how to handle groups larger than --max-group-size.
          
          [default: passthrough]

          Possible values:
          - passthrough: Output all reads unmodified, skipping consensus calling (backwards-compatible default)
          - drop:        Omit the group from output entirely
          - sample:      Pseudorandomly subsample reads to max-group-size, then consensus call
          - longest:     Keep the longest reads (up to max-group-size), then consensus call

Filtering:
      --len <LEN>
          filter lengths to a value within the given float interval [a,b].
          a is the minimum, and b is the maximum (both inclusive).
          alternatively, a can be `-inf` and b can be `inf.
          an unbounded interval (i.e. no length filter) is given by `0,inf`.
          
          [default: 0,15000]

      --qual <QUAL>
          filter average read quality to a value within the given float interval [a,b].
          see the docs for `--len` for documentation on how to use the interval.
          
          [default: 0,inf]

Output options:
      --report-original-reads
          for each duplicate group of reads, report the original reads along with the consensus

      --report-original-header
          include original read headers in the output as the nH:Z: tag

      --extra-stats
          [intended for internal development] add debugging information to the read header

      --sort-by <SORT_BY>
          sort output groups by the specified capture group tag (e.g., 'CB' for cell barcode)
```

## Output format

A `.fastq` file will be produced. Read headers carry metadata as SAM auxiliary tags in the FASTQ
comment field (tab-separated, after the read name). See the [Output format reference](../reference/output-format.md)
for a complete description of all tags.

A typical output looks like this (tabs shown as newlines for clarity):

```
@consensus_12047_1
  CB:Z:GATAGCTAGCAACAAT
  UB:Z:ATTTTACCGACC
  nL:i:2
```

## Options

- `--threads`: set the number of threads that _nailpolish_ should use
- `--report-original-reads`: report the original reads as well as the consensus read
- `--report-original-header`: report the original headers of the reads used to produce
  a consensus
- `--no-false-duplicate-detection`: disable the false duplicate detection algorithm (see below). Also known as `--no-clustering`.
- `--fdd-threshold <RATIO>`: the insert-node ratio threshold used by false duplicate detection
  (default: `0.25`). Lower values are stricter, splitting reads into separate clusters more
  readily; higher values merge more reads into the same cluster. See below.
- `--len <LEN>`: filter reads by sequence length. Reads outside the interval are excluded
  from consensus calling. Default: `0,15000` (reads longer than 15,000 bp are excluded,
  as excessively long reads from sequencing errors can dominate consensus calling time).
- `--qual <QUAL>`: filter reads by average base quality. Default: `0,inf` (no quality filter).
- `--max-group-size <N>`: the size threshold for large-group handling (default: 250).
  Groups exceeding this size are processed according to `--large-group-method`.
- `--large-group-method <METHOD>`: controls what happens to groups that exceed `--max-group-size`.
  Options:
    - `passthrough` *(default)*: output all reads unmodified with no consensus calling — the existing behaviour.
      Very large groups are typically caused by false duplicates; skipping consensus calling prevents an outsized
      impact on runtime.
    - `drop`: omit the group from output entirely.
    - `sample`: pseudorandomly subsample reads down to `--max-group-size` reads and produce
      a consensus from the sample. The random seed is derived from the group ID, so output
      is fully reproducible for a given input file.
    - `longest`: keep only the longest reads (up to `--max-group-size`) and produce a consensus
      from those reads.
  - `--sort-by <TAG>`: sort groups by the named capture group tag before output
    (e.g. `--sort-by CB` to sort by cell barcode).

## False duplicate detection

By default, _nailpolish_ clusters reads within each duplicate group to detect and separate
_false duplicates_ — reads that share a barcode/UMI by coincidence rather than by biology.
Before adding each read to a partial order alignment graph, nailpolish checks whether the read
aligns well to the existing graph. If the alignment introduces too many new nodes relative to
existing ones (more than the `--fdd-threshold` fraction of valid nodes, default 25%), the read is
assigned to a new cluster rather than merged into the current one. Pass `--fdd-threshold <RATIO>`
to make this stricter (lower) or looser (higher) for noisier or cleaner data.

To disable this behaviour — for example, when you are confident that all reads in a group are
true duplicates, or when using pre-clustered inputs from a tool like isONclust — pass
`--no-false-duplicate-detection`. This provides a small performance benefit and guarantees a single consensus
per group.

```bash
# default: false duplicate detection enabled
nailpolish consensus reads.fastq -o output.fastq

# disabled: one consensus per group, no splitting
nailpolish consensus --no-false-duplicate-detection reads.fastq -o output.fastq
```

!!! note
    The false duplicate detection was designed for reads that share no biological similarity.
    For pre-clustered inputs from tools with relaxed clustering criteria (e.g. isONclust),
    many loosely similar clusters may still pass through as a single consensus.
    Whether to use `--no-false-duplicate-detection` depends on your confidence in the upstream clustering.