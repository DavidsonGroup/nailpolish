// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Group.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

const SAMPLE_FASTQ: &str = "tests/data/scmixology2_sample.fastq";

// This is not possible, instead, the index file is now provided alongside the original file.
// #[test]
// fn index() {
//     let temp = assert_fs::NamedTempFile::new("scmixology2_sample.fastq.nailpolish.idx").unwrap();
//     let mut command = Command::cargo_bin("nailpolish").unwrap();
//     let _ = command.args(["index", SAMPLE_FASTQ]).assert().success();
//     // lazy way of checking that these files are the same
//     // EXCEPT for the header, which contains unique date and runtime information
//     let cmp_cmd = format!(
//         "diff <(tail -n+2 tests/correct/index.tsv) <(tail -n+2 {})",
//         temp.path().to_str().unwrap()
//     );
//     let _ = Command::new("bash").arg("-c").arg(&cmp_cmd).unwrap();
//     temp.close().unwrap();
// }

#[test]
fn summary() {
    let temp = assert_fs::NamedTempFile::new("_summary.html").unwrap();
    let path = temp.path().to_str().unwrap();

    let mut command = Command::cargo_bin("nailpolish").unwrap();

    let _ = command
        .args(["summary", SAMPLE_FASTQ, "-o", path])
        .assert()
        .success();

    temp.assert(predicate::path::exists());
}

#[test]
fn consensus_1t_no_clustering() {
    let temp = assert_fs::NamedTempFile::new("consensus.fastq").unwrap();
    let path = temp.path().to_str().unwrap();

    let mut command = Command::cargo_bin("nailpolish").unwrap();

    let _ = command
        .args([
            "consensus",
            SAMPLE_FASTQ,
            "-o",
            path,
            "--threads",
            "1",
            "--report-original-header",
            "--report-original-reads",
            "--no-clustering",
        ])
        .assert()
        .success();

    // DISABLED due to bug in SPOA consensus algorithm
    // TODO: remove when bug is fixed
    // const CORRECT_FILE: &str = "tests/correct/consensus_no_cluster.fastq";
    // let _ = Command::new("diff").args([path, CORRECT_FILE]).unwrap();

    temp.close().unwrap();
}

#[test]
fn consensus_4t_no_clustering() {
    let temp = assert_fs::NamedTempFile::new("consensus_4t.fastq").unwrap();
    let path = temp.path().to_str().unwrap();

    let mut command = Command::cargo_bin("nailpolish").unwrap();

    let _ = command
        .args([
            "consensus",
            SAMPLE_FASTQ,
            "-o",
            path,
            "--threads",
            "4",
            "--report-original-header",
            "--report-original-reads",
            "--no-clustering",
        ])
        .assert()
        .success();

    // DISABLED due to bug in SPOA consensus algorithm
    // TODO: remove when bug is fixed
    // const CORRECT_FILE: &str = "tests/correct/consensus_no_cluster.fastq";
    // let _ = Command::new("diff").args([path, CORRECT_FILE]).unwrap();

    temp.close().unwrap();
}

#[test]
fn extract_3() {
    let temp = assert_fs::NamedTempFile::new("extract_3.fastq").unwrap();
    let path = temp.path().to_str().unwrap();

    let mut command = Command::cargo_bin("nailpolish").unwrap();

    let _ = command
        .args(["extract", SAMPLE_FASTQ, "-o", path, "--group-size", "3"])
        .assert()
        .success();

    const CORRECT_FILE: &str = "tests/correct/extract_3.fastq";
    let cmp_cmd = format!("diff {} {}", temp.path().to_str().unwrap(), CORRECT_FILE);

    let _ = Command::new("bash").arg("-c").arg(&cmp_cmd).unwrap();

    temp.close().unwrap();
}

#[test]
fn consensus_4t_with_clustering() {
    let temp = assert_fs::NamedTempFile::new("consensus_4t.fastq").unwrap();
    let path = temp.path().to_str().unwrap();

    let mut command = Command::cargo_bin("nailpolish").unwrap();

    let _ = command
        .args([
            "consensus",
            SAMPLE_FASTQ,
            "-o",
            path,
            "--threads",
            "4",
            "--report-original-header",
            "--report-original-reads",
        ])
        .assert()
        .success();

    const CORRECT_FILE: &str = "tests/correct/consensus_with_cluster.fastq";

    let _ = Command::new("diff").args([path, CORRECT_FILE]).unwrap();

    temp.close().unwrap();
}

#[test]
fn consensus_with_cluster_file() {
    use std::fs;

    // Create temporary FASTQ file copy so we can create our own index
    let temp_fastq = assert_fs::NamedTempFile::new("temp_sample.fastq").unwrap();
    fs::copy(SAMPLE_FASTQ, temp_fastq.path()).unwrap();

    let temp_consensus = assert_fs::NamedTempFile::new("consensus_cluster.fastq").unwrap();
    let path = temp_consensus.path().to_str().unwrap();

    // First, create index using cluster file
    let mut index_command = Command::cargo_bin("nailpolish").unwrap();
    index_command
        .args([
            "index",
            temp_fastq.path().to_str().unwrap(),
            "--clusters",
            "tests/data/clusters.txt",
        ])
        .assert()
        .success();

    // Then run consensus
    let mut consensus_command = Command::cargo_bin("nailpolish").unwrap();
    consensus_command
        .args([
            "consensus",
            temp_fastq.path().to_str().unwrap(),
            "-o",
            path,
            "--threads",
            "3",
            "--report-original-header",
            "--report-original-reads",
        ])
        .assert()
        .success();

    const CORRECT_FILE: &str = "tests/correct/consensus_with_cluster.fastq";

    // cluster files will not have the CB/UB tags, so
    // we will only compare lines 2 and 4 of each output read
    let cmp_cmd = format!(
        "diff \
        <(awk 'NR % 4 == 2 || NR % 4 == 0' {CORRECT_FILE} | sort) \
        <(awk 'NR % 4 == 2 || NR % 4 == 0' {path} | sort)"
    );
    let _ = Command::new("bash").args(["-c", &cmp_cmd]).unwrap();

    temp_fastq.close().unwrap();
    temp_consensus.close().unwrap();
}
