# nailpolish consensus

Consensus call duplicated reads.
The reads must first have been [indexed](./generate-index.md).
By default, reads within each duplicate group will be clustered to eliminate false duplicates.

## Usage

```shell
$ nailpolish call --help
Generate a consensus-called 'cleaned up' file

Usage: nailpolish consensus [OPTIONS] <INPUT>

Arguments:
  <INPUT>  the input .fastq

Options:
  -o, --output <OUTPUT>         the output .fastq, or empty for stdout
  -t, --threads <THREADS>       the number of threads to use [default: 4]
      --report-original-reads   for each duplicate group of reads, report the original reads along with the consensus
      --report-original-header  if the original read headers are valuable, this will create a orig_header field 
                                in the consensus called result with the entire original read header
      --extra-stats             add debugging information to the read header [intended for internal development]
                                warning: since timings are reported, the output will not be identical across runs
      --no-clustering           disable the clustering algorithm this will prevent nailpolish from detecting and separating false duplicates
  -h, --help                    Print help
```

## Output format

A .fastq file will be produced. By default, each read will look like this:

```bash

@GATAGCTAGCAACAAT_ATTTTACCGACC|id=12047|type=consensus|cluster=1|reads_called=2
#  barcode─┘        UMI─┘  group id─┘       type─┘             │              │
#                                   ID of cluster within group─┘              │
#                                            this cluster has two reads in it─┘


# if duplicate group 12047 has 2 clusters, both are reported...
@GATAGCTAGCAACAAT_ATTTTACCGACC|id=12047|type=consensus|cluster=2|reads_called=3
#            same group as above...─┘                          │              │
#                                     ...but different cluster─┘              │
#                                          this cluster has three reads in it─┘                                


@GCAGTTAAGGATATAC_ACAGTTTCTTTG|id=2829|type=single|cluster=1|reads_called=1
#                 this group has only one read─┘           │              │
#                         in this case, these are always 1─┴──────────────┘

```

Flags can be passed to add other information to the output as well.

```bash

# using `--report-original-reads`, the original reads are produced as well...
@CTCAAGACATTGAGCT_ATTTTTTTTTTT|id=3566|type=original|read=3|cluster=2
#                    this is the original read─┘          │         │
#                                 third read in the group─┘         │
#                                this read contributed to cluster 2─┘


# using `--report-original-header`, the original headers are outputted...
@GGAGGATTCTTCTAAC_TGTTCTTGAAGC|<...removed...>|orig_header=["GGAGGATTCTTCTAAC_TGTTCTTGAAGC#682b2274-473a-4a59-affe-30dbe4f1d070_+1of1","GGAGGATTCTTCTAAC_TGTTCTTGAAGC#36a1b7bd-5bab-45e1-a591-cb966f890f90_-1of1"]
#                            ... see the original header →                                   →                             →                   →                           →                                ... these are the two reads that were called.

```


## Options

- `--threads`: set the number of threads that _nailpolish_ should use
- `--report-original-reads`: report the original reads as well as the consensus read
- `--report-original-header`: report the original headers of the reads used to produce
  a consensus