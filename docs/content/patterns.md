---
hide:
  - navigation
---

# Common usage patterns

Nailpolish is a flexible tool designed to fit in to other workflows. Here are some examples of how Nailpolish
can integrate with your preferred tools and pipelines.

## I have...
### ... a convoluted header format
The `nailpolish index` command is flexible and supports a custom barcode structure: you can write
a custom regular expression to capture as many fields as you need.
[See the docs for more.](commands/index.md/#barcode-regex)

### ... more than one barcode or UMI, like split-seq
Nailpolish's indexer applies no limits on the number of barcode/UMI entries. The only requirement is that
they are all named. For instance, here is a suitable barcode regex for this demultiplexed LR-split-seq file:

```bash
nailpolish index lr-split-seq.fastq \
    --barcode-regex "(?<C1>[ATCGNX]+)_#(?<C2>[ATCGNX]+)_#(?<C3>[ATCGNX]+)_(?<UB>[ATCGNX]+)#"

example read:
@AATAGAAC_#GAGCTGAA_#CGGATTGC_TGTTTTCAGA#SRR13948564.5_+1of1_+1of1_+1of1
```

### ... pre-clustered reads in a file
If you have already clustered reads with an external tool such as
[isONclust](https://github.com/ksahlin/isONclust), use the `--clusters` option.
_nailpolish_ can then perform consensus calling on these clusters. This is significantly faster than
calling `spoa` through the shell in a loop.

### ... pre-clustered reads from isONclust
isONclust produces output in the format `cluster_id<tab>read_header`. Convert it to the cluster
file format expected by _nailpolish_ using `awk`:

```bash
awk 'BEGIN{print "read_id;UB"} {print $2";"$1}' isonclust_clusters.tsv > clusters.txt
nailpolish index --clusters clusters.txt reads.fastq
```

Here, the cluster ID is used as the `UB` tag (effectively treated as the grouping key).

### ... pre-clustered reads from UMI-tools
Running `umi_tools group --group-out=groups.tsv` produces a tab-separated file with the columns
`read_id, contig, position, gene, umi, umi_count, final_umi, final_umi_count, unique_id`.

Only `read_id` and `unique_id` are strictly needed. Note that `final_umi` alone is *not* a safe
grouping key: UMI-tools clusters UMIs within a bundle of reads sharing an alignment position
(or a gene, under `--per-gene`), so the same corrected UMI is routinely reused across unrelated
loci. `unique_id` is assigned after clustering and is unique across the whole file. We combine
the two below so that the UMI sequence remains visible in the tag, which is convenient when
inspecting or filtering clusters later.

```bash
awk -F'\t' 'BEGIN{print "read_id;UB"} NR>1 {print $1";"$7"_"$9}' groups.tsv > clusters.txt
nailpolish index --clusters clusters.txt reads.fastq
```

## I want...
### ... to quickly see an estimate of the duplicate count, before I do anything
Try `nailpolish summary` - it generates a .HTML file with a histogram of duplicate rates. [See an example here.](./assets/summary.html)

### ... to keep the barcode and UMI after alignment with minimap2
Nailpolish inserts the identifiers (such as barcode and UMI) into the read header via ".fastq comments" (`\tCB:Z:<>` and `\tUB:Z:<>`).
minimap2 can produce these comments as .SAM tags via the `-y` command.

### ... to sort the output by cell barcode, for Oarfish downstream
There's a flag for that. To sort by the "CB" identifier, run:
```bash
nailpolish consensus input.fastq \
    --threads 16 \
    --sort-by CB
```

### ... for extremely large groups to be consensus called anyways
By default, large duplicate groups (>250 duplicates) are not consensus called together; they are
often not true molecular replicates, large groups reduce consensus quality, and have an outsized
effect on runtime.

If you're confident that these duplicates are true duplicates, you can choose how to process them
using the two flags `--longest` and `--large-group-method`:

```bash
nailpolish consensus input.fastq \
    --threads 16 \
    --large-group-method sample
```

```bash
--max-group-size <MAX_GROUP_SIZE>
    filter out groups larger than this size (skip consensus calling for very large groups) this will prevent large groups, which are typically false duplicates, from having an outsized impact on runtime

    [default: 250]
    
--large-group-method <LARGE_GROUP_METHOD>
    how to handle groups larger than --max-group-size.
    - `passthrough` outputs all reads without consensus calling (default);
    - `drop` omits the group from output entirely;
    - `sample` pseudorandomly subsamples to max-group-size and consensus calls the result;
    - `longest` keeps the longest reads up to max-group-size and consensus calls the result.

    [default: passthrough]

    Possible values:
    - passthrough: Output all reads unmodified, skipping consensus calling (backwards-compatible default)
    - drop:        Omit the group from output entirely
    - sample:      Pseudorandomly subsample reads to max-group-size, then consensus call
    - longest:     Keep the longest reads (up to max-group-size), then consensus call
```