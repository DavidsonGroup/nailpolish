---
title: nailpolish index
---

# nailpolish index

This command is used to create an index file from a demultiplexed `.fastq`.
An index is required to run the other _nailpolish_ commands.
The index command supports reads in multiple formats.

## Usage

An index file will be created at `<file>.fastq.nailpolish.idx`.

```shell
$ nailpolish index --help
Create an index file from a demultiplexed .fastq

Usage: nailpolish index [OPTIONS] <INPUT> [PRESET]

Arguments:
  <INPUT>
          the input .fastq file

  [PRESET]
          [default: bc-umi]

          Possible values:
          - bc-umi:           @BARCODE_UMI format as produced by Flexiplex for 10x3 chemistry
          - umi-tools:        `_<UMI>` format as produced by `umi-tools extract`
          - illumina:         bcl2fastq format, which has `:<UMI>` at the end of the read ID
          - sam-tagged-cb-ub: .sam tag format with barcode and UMI (CB:Z and UB:Z tags)
          - sam-tagged-cb:    .sam tag format with barcode only (CB:Z tag)

Options:
      --overwrite
          overwrite an existing index file, if it exists

      --clusters <CLUSTERS>
          use a file containing pre-clustered reads. the file must be semicolon-delimited
          with a header line, where the first column is the read ID and subsequent columns
          are tag names. for example:
            read_id;CB;UB
            READ_HEADER_1;BARCODE1;UMI1

      --barcode-regex <BARCODE_REGEX>
          barcode regex format type, for custom header styles. this will override the preset given.
          for example, for the `bc-umi` preset:
              ^(?<CB>[ATCGNX]{16})_(?<UB>[ATCGNX]{12})

      --skip-unmatched
          skip, instead of error, on reads which are not accounted for:
          - if a cluster file is passed, any reads which are not in any cluster
          - if a barcode regex or preset is used (default), any reads which do not match the regex

  -h, --help
          Print help (see a summary with '-h')
```

## Indexing different file types

### Flexiplex output

If your reads were demultiplexed with [Flexiplex](https://github.com/DavidsonGroup/flexiplex) (Cheng et al. 2024),
use the default `bc-umi` preset:

```bash
nailpolish index reads.fastq
# equivalent to:
nailpolish index reads.fastq bc-umi
```

Read headers are expected to look like `ATCGATCGATCGATCG_ATCGATCGATCG` (16 bp barcode + 12 bp UMI).

### SAM/BAM-tagged reads

If your reads come from an alignment pipeline that annotates reads with SAM tags, use one of the SAM presets:

```bash
# reads tagged with both CB (cell barcode) and UB (UMI)
nailpolish index reads.fastq sam-tagged-cb-ub

# reads tagged with CB only (no UMI)
nailpolish index reads.fastq sam-tagged-cb
```

These presets extract tags from the read comment field, which in FASTQ format carries the SAM auxiliary tags
(e.g., `\t:CB:Z:ATCGATCG\t:UB:Z:TTTTTTTT`).

### Pre-clustered reads from isONclust

If you have already clustered reads with an external tool such as
[isONclust](https://github.com/ksahlin/isONclust), use the `--clusters` option.
_nailpolish_ can then perform consensus calling on these clusters, which is significantly faster
than calling the `spoa` binary in a loop since it uses the same library under the hood.

isONclust produces output in the format `cluster_id<tab>read_header`. Convert it to the cluster
file format expected by _nailpolish_ using `awk`:

```bash
awk 'BEGIN{print "read_id;UB"} {print $2";"$1}' isonclust_clusters.tsv > clusters.txt
nailpolish index --clusters clusters.txt reads.fastq
```

Here, the cluster ID is used as the `UB` tag (effectively treated as the grouping key).

## Reading the index
### Presets

Five presets are bundled with _nailpolish_ for common barcode formats.
These are useful when the header of each read contains information about the barcode.

- `bc-umi`: read headers look like this: `ATCGATCGATCG_ATCGATCGATCGATCG` in the `BC_UMI` format.
  This is the default barcoding format produced by
  the [Flexiplex demultiplexer](https://github.com/DavidsonGroup/flexiplex) (Cheng et al. 2024).
- `umi-tools`: read headers look like this: `HISEQ:87:00000000T_ATCGATCGATCG` where `ATCGATCGATCG` is the UMI sequence.
  This is the default UMI header format expected from the [umi-tools](https://doi.org/10.1101/gr.209601.116) (Smith et al. 2017)
  collection of UMI management tools.
- `illumina`: read headers look like this: `SIM:1:FCX:1:2106:15337:1063:ATCGATCGATCG 1:N:0:ATCACG` where `ATCGATCGATCG`
  is the UMI sequence.
  This is the default UMI header format produced by tools such as `bcl2fastq`.
- `sam-tagged-cb-ub`: reads carry both a cell barcode (`CB:Z:`) and UMI (`UB:Z:`) as SAM auxiliary tags
  in the read comment field.
- `sam-tagged-cb`: reads carry only a cell barcode (`CB:Z:`) as a SAM auxiliary tag.

### Barcode regex

For reads where barcodes and UMIs are contained in the header, in an esoteric format, a custom regular expression
can be provided through the `--barcode-regex <BARCODE_REGEX>` parameter. As examples, here are the regular expressions
for the presets above:

- `bc-umi`: `--barcode-regex "^(?<CB>[ATCGNX]{16})_(?<UB>[ATCGNX]{12})"`
- `umi-tools`: `--barcode-regex "_(?<UB>[ATCGNX]+)$"`
- `illumina`: `--barcode-regex ":(?<UB>[ATCGNX]+)$"`

Capture group names (e.g. `CB`, `UB`) determine what tag names appear in the output of `nailpolish consensus`.

Regular expressions are parsed by the excellent `regex` library for Rust.
This library is performant and has guarantees on worst-case time complexity;
however, the scope of supported regular expression features is more limited.
For complex queries, it is recommended that you consult the [crate documentation](https://docs.rs/regex/latest/regex/)
and test your regular expression using [regex101](https://regex101.com/), ensuring that you set the 'Flavor' to 'Rust'.

_nailpolish_ expects that every read in the input `.fastq` **must** be able to be matched to the provided
regular expression.

### Cluster file

_nailpolish_ can alternatively read grouping information from a separately provided file, if this information is
not in the read headers. This is useful when reads have been pre-clustered by an external tool.

The file must be **semicolon-delimited** (`;`) and must include a **header line** as the first row.
The first column must be named `read_id` and contain the read header. Subsequent columns define the tag names
(e.g. `CB`, `UB`) and their values for each read.

```
read_id;CB;UB
READ_HEADER_1;BARCODE1;UMI1
READ_HEADER_2;BARCODE2;UMI2
```

The column names in the header row (after `read_id`) become the tag names used in the output of `nailpolish consensus`.

By default, _nailpolish_ expects that every read in the input `.fastq` **must** have a corresponding entry in the
cluster file.
In the event where this is not the case, _nailpolish_ will error. To ignore this error and silently skip over any
unmatched reads, the `--skip-unmatched` flag should be passed.