---
title: Output format
---

# Output format

_nailpolish_ produces standard `.fastq` files. Read headers carry metadata as **SAM auxiliary tags**
embedded in the FASTQ comment field (the part of the `@` line after the first tab). This format is
compatible with the [SAM specification](https://samtools.github.io/hts-specs/SAMtags.pdf) and is
passed through to SAM/BAM output by aligners such as minimap2 when they are run with the `-y` flag.

A typical nailpolish read header looks like this (tabs shown as newlines for clarity):

```
@consensus_12047_1
  CB:Z:ATCGATCGATCGATCG
  UB:Z:TTTTTTTTTTTT
  nL:i:3
```

Fields are separated by tab characters (`\t`). The read name (before the first tab) encodes the read
type, group ID, and cluster ID. Each subsequent field is a SAM tag in `TAG:TYPE:VALUE` form.

---

## Read name

The read name states the read type, followed by the indices which identify the read:

| Name                                | Read type                                          | Example               |
| ----------------------------------- | -------------------------------------------------- | --------------------- |
| `consensus_{group}_{cluster}`       | Consensus of two or more reads                     | `consensus_12047_1`   |
| `singleton_{group}`                 | A group containing a single read                    | `singleton_2829`      |
| `original_{group}_{cluster}_{read}` | Original read (requires `--report-original-reads`) | `original_12047_1_3`  |
| `passthrough_{group}_{read}`        | Filtered read (group exceeded `--max-group-size`)  | `passthrough_99_1`    |

Group indices are 0-indexed; cluster and read indices are 1-indexed within their parent. Reads sharing
a group index came from the same barcode key. When `--no-clustering` is passed the cluster index is
always `1`.

These identifiers are only present in the read name — they are not duplicated as tags.

---

## Tags

| Tag              | SAM type | Description                                                                                                                                                                       |
| ---------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| *(capture tags)* | `:Z:`    | One tag per named capture group from the index (e.g., `CB`, `UB`). Tag names come from the regex capture group names or the cluster file header columns. Present on all reads.      |
| `nL`             | `:i:`    | Number of reads in this cluster (or group if `--no-clustering`). Present on `consensus` reads.                                                                                     |
| `nH`             | `:Z:`    | JSON array of original read headers used to produce this read. Present on `consensus` and `singleton` reads when `--report-original-header` is passed.                             |
| `nE`             | `:i:`    | Elapsed time to process this duplicate group, in microseconds. Present on `consensus` reads when `--extra-stats` is passed. Output is non-deterministic across runs.               |
| `nA`             | `:Z:`    | JSON array of SPOA alignment prediction results (fields: `new_nodes`, `sequence_len`, `valid_nodes`). Present on `original` reads when `--extra-stats` is passed.                  |

---

## Read types

### `consensus`

The group contained two or more reads. A consensus sequence was generated using partial order alignment.
If clustering is enabled (the default), one consensus read is produced per cluster within the group.

```
@consensus_12047_1	CB:Z:ATCGATCGATCGATCG	UB:Z:TTTTTTTTTTTT	nL:i:3
```
```
@consensus_12047_2	CB:Z:ATCGATCGATCGATCG	UB:Z:TTTTTTTTTTTT	nL:i:2
```

Both reads share the same group index `12047` (same duplicate group) but have different cluster indices
(different consensus sequences). This happens when false duplicate detection splits the group.

### `singleton`

The group contained exactly one read. No consensus calling was performed; the read is passed through
with updated tags.

```
@singleton_2829	CB:Z:GCAGTTAAGGATATAC	UB:Z:ACAGTTTCTTTG
```

### `original`

The original, unmodified read from a group — emitted alongside the consensus when `--report-original-reads`
is passed. The read name records which cluster it contributed to and its index within the group.

```
@original_12047_1_3	CB:Z:ATCGATCGATCGATCG	UB:Z:TTTTTTTTTTTT
```

Original reads appear in the output **before** the corresponding consensus read for each cluster.

### `passthrough`

The group exceeded the `--max-group-size` threshold (default: 250 reads). Consensus calling was skipped
for this group. Each read in the group is emitted individually.

```
@passthrough_99_1	CB:Z:ATCGATCGATCGATCG	UB:Z:TTTTTTTTTTTT
```

Passthrough reads retain the barcode tags but no cluster or count tags.


## SAM compatibility

Because nailpolish uses the SAM auxiliary tag format, reads can be aligned with minimap2 and the
tags will be preserved in the resulting BAM:

```bash
minimap2 -y -ax splice reference.fa output.fastq | samtools sort -o aligned.bam
```

The `-y` flag instructs minimap2 to copy FASTQ comment fields into the SAM output as auxiliary tags.
All nailpolish tags will be available for downstream filtering in tools like `samtools view`.

