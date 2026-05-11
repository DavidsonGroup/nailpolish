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
@processed_12047_1
  MI:Z:ATCGATCGATCGATCG_TTTTTTTTTTTT
  nI:i:12047
  CB:Z:ATCGATCGATCGATCG
  UB:Z:TTTTTTTTTTTT
  nT:Z:consensus
  nC:i:1
  nL:i:3
```

Fields are separated by tab characters (`\t`). The read name (before the first tab) encodes the read type,
group ID, and cluster ID. Each subsequent field is a SAM tag in `TAG:TYPE:VALUE` form.

---

## Read name

The read name prefix depends on the read type:

| Prefix                              | Read type                                          | Example              |
| ----------------------------------- | -------------------------------------------------- | -------------------- |
| `processed_{group}_{cluster}`       | Consensus or simplex                               | `processed_12047_1`  |
| `original_{group}_{cluster}_{read}` | Original read (requires `--report-original-reads`) | `original_12047_1_3` |
| `filtered_{group}_{read}`           | Filtered read (group exceeded `--max-group-size`)  | `filtered_99_1`      |

---

## Tags

| Tag              | SAM type | Description                                                                                                                                                                        |
| ---------------- | -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `MI`             | `:Z:`    | Molecular barcode. All barcode components joined with `_` (e.g., `CB_UB`). Present on all reads.                                                                                   |
| `nI`             | `:i:`    | Duplicate group index. Unique per distinct barcode key. Present on all reads.                                                                                                      |
| *(capture tags)* | `:Z:`    | One tag per named capture group from the index (e.g., `CB`, `UB`). Tag names come from the regex capture group names or the cluster file header columns. Present on all reads.     |
| `nT`             | `:Z:`    | Read type: `consensus`, `simplex`, `original`, or `filtered`. Present on all reads.                                                                                                |
| `nC`             | `:i:`    | Cluster index within the duplicate group (1-indexed). Present on `consensus` and `original` reads. Absent when `--no-clustering` is passed.                                        |
| `nL`             | `:i:`    | Number of reads in this cluster (or group if `--no-clustering`). Present on `consensus` and `simplex` reads.                                                                       |
| `nR`             | `:i:`    | Index of this read within its duplicate group (1-indexed). Present on `original` reads only.                                                                                       |
| `nH`             | `:Z:`    | JSON array of original read headers used to produce this read. Present on `consensus` and `simplex` reads when `--report-original-header` is passed.                               |
| `nE`             | `:i:`    | Elapsed time to process this duplicate group, in microseconds. Present on `consensus` and `simplex` reads when `--extra-stats` is passed. Output is non-deterministic across runs. |
| `nA`             | `:Z:`    | JSON array of SPOA alignment prediction results (fields: `new_nodes`, `sequence_len`, `valid_nodes`). Present on `original` reads when `--extra-stats` is passed.                  |

---

## Read types

The `nT` tag identifies how a read was produced. There are four possible values:

### `consensus`

The group contained two or more reads. A consensus sequence was generated using partial order alignment.
If clustering is enabled (the default), one consensus read is produced per cluster within the group.

```
@processed_12047_1	MI:Z:ATCGATCGATCGATCG_TTTTTTTTTTTT	nI:i:12047	CB:Z:ATCGATCGATCGATCG	UB:Z:TTTTTTTTTTTT	nT:Z:consensus	nC:i:1	nL:i:3
```
```
@processed_12047_2	MI:Z:ATCGATCGATCGATCG_TTTTTTTTTTTT	nI:i:12047	CB:Z:ATCGATCGATCGATCG	UB:Z:TTTTTTTTTTTT	nT:Z:consensus	nC:i:2	nL:i:2
```

Both reads share the same `MI` and `nI` (same duplicate group) but have different `nC` values (different
clusters, i.e., different consensus sequences). This happens when false duplicate detection splits the group.

### `simplex`

The group contained exactly one read. No consensus calling was performed; the read is passed through
with updated tags.

```
@processed_2829_1	MI:Z:GCAGTTAAGGATATAC_ACAGTTTCTTTG	nI:i:2829	CB:Z:GCAGTTAAGGATATAC	UB:Z:ACAGTTTCTTTG	nT:Z:simplex	nL:i:1
```

### `original`

The original, unmodified read from a group — emitted alongside the consensus when `--report-original-reads`
is passed. Each original read records which cluster it contributed to (`nC`) and its index in the group (`nR`).

```
@original_12047_1_3	MI:Z:ATCGATCGATCGATCG_TTTTTTTTTTTT	nI:i:12047	CB:Z:ATCGATCGATCGATCG	UB:Z:TTTTTTTTTTTT	nT:Z:original	nC:i:1	nR:i:3
```

Original reads appear in the output **before** the corresponding consensus read for each cluster.

### `filtered`

The group exceeded the `--max-group-size` threshold (default: 250 reads). Consensus calling was skipped
for this group. Each read in the group is emitted individually.

```
@filtered_99_1	MI:Z:ATCGATCGATCGATCG_TTTTTTTTTTTT	nI:i:99	CB:Z:ATCGATCGATCGATCG	UB:Z:TTTTTTTTTTTT	nT:Z:filtered
```

Filtered reads retain all global tags but no cluster or count tags.


## SAM compatibility

Because nailpolish uses the SAM auxiliary tag format, reads can be aligned with minimap2 and the
tags will be preserved in the resulting BAM:

```bash
minimap2 -y -ax splice reference.fa output.fastq | samtools sort -o aligned.bam
```

The `-y` flag instructs minimap2 to copy FASTQ comment fields into the SAM output as auxiliary tags.
All nailpolish tags will be available for downstream filtering in tools like `samtools view`.

