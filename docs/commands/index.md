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
          - bc-umi:    @BARCODE_UMI format as produced by Flexiplex for 10x3 chemistry
          - umi-tools: `_<UMI>` format as produced by `umi-tools extract`
          - illumina:  bcl2fastq format, which has `:<UMI>` at the end of the read ID

Options:
      --overwrite
          overwrite an existing index file, if it exists

      --clusters <CLUSTERS>
          whether to use a file containing pre-clustered reads, with every line in one of two formats:
            1. READ_ID;BARCODE
            2. READ_ID;BARCODE;UMI

      --barcode-regex <BARCODE_REGEX>
          barcode regex format type, for custom header styles. this will override the preset given.
          for example, for the `bc-umi` preset:
              ^([ATCG]{16})_([ATCG]{12})

      --skip-unmatched
          skip, instead of error, on reads which are not accounted for:
          - if a cluster file is passed, any reads which are not in any cluster
          - if a barcode regex or preset is used (default), any reads which do not match the regex

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

  -h, --help
          Print help (see a summary with '-h')
```

## Reading the index
### Presets

Three presets are bundled with _nailpolish_ for common barcode formats.
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

### Barcode regex

For reads where barcodes and UMIs are contained in the header, in an esoteric format, a custom regular expression
can be provided through the `--barcode-regex <BARCODE_REGEX>` parameter. As examples, here are the regular expressions
for the presets above:

- `bc-umi`: `--barcode-regex "^([ATCG]{16})_([ATCG]{12})"`
- `umi-tools`: `--barcode-regex "_([ATCG]+)$"`
- `illumina`: `--barcode-regex ":([ATCG]+)$"`

Regular expressions are parsed by the excellent `regex` library for Rust.
This library is performant and has guarantees on worst-case time complexity;
however, the scope of supported regular expression features is more limited.
For complex queries, it is recommended that you consult the [crate documentation](https://docs.rs/regex/latest/regex/)
and test your regular expression using [regex101](https://regex101.com/), ensuring that you set the 'Flavor' to 'Rust'.

_nailpolish_ expects that every read in the input `.fastq` **must** be able to be matched to the provided
regular expression.

### Cluster file

_nailpolish_ can alternatively extract UMIs from a separately provided delimiter-separated file, if this information is
not in the read headers.
The file must be **semicolon-delimited** (`;`). Rows must be in the format `READ_ID;BARCODE` or `READ_ID;BARCODE;UMI`.
Note that **no header line** should be present in the file.

By default, _nailpolish_ expects that every read in the input `.fastq` **must** have a corresponding entry in the
cluster file.
In the event where this is not the case, _nailpolish_ will error. To ignore this error and silently skip over any
unmatched reads, the `--skip-unmatched` flag should be passed.

## Filtering
_nailpolish_ has filter settings which will exclude a read from being consensus called or considered part of a group.
**The read will be in the final consensus called output.** This exists because sometimes sequencing errors can be excessively long, which have an outsized impact overall consensus calling time.

The default filtering settings are very conservative, only filtering reads with length >15000bp.

Two types of metadata can be filtered against: sequence length (using `--len`) and sequence quality (using `--qual`).
Filters should be _intervals_; that is, a string `[a, b]` where `a` and `b` represent the inclusive upper and lower
bound respectively.