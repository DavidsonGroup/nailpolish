#!/bin/bash
# CN: nextest setup script to decompress test data
set -e

# Decompress scmixology2_sample.fastq.gz if the uncompressed version doesn't exist
if [ ! -f "tests/data/scmixology2_sample.fastq" ]; then
    echo "Decompressing scmixology2_sample.fastq.gz..."
    gunzip -c "tests/data/scmixology2_sample.fastq.gz" > "tests/data/scmixology2_sample.fastq"
    echo "✓ Created tests/data/scmixology2_sample.fastq"
else
    echo "✓ tests/data/scmixology2_sample.fastq already exists"
fi