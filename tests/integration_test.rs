// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Group.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;

const SAMPLE_FASTQ: &str = "tests/data/scmixology2_sample.fastq";
const SAMPLE_FASTQ_GZ: &str = "tests/data/scmixology2_sample.fastq.gz";

fn make_temp_dir(
    source_path: &str,
    input_name: &str,
    should_index: bool,
) -> (assert_fs::TempDir, String, String) {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let input_path = temp_dir
        .path()
        .join(input_name)
        .to_string_lossy()
        .to_string();
    let output_path = temp_dir
        .path()
        .join("output.fastq")
        .to_string_lossy()
        .to_string();

    // Copy source file to input path
    std::fs::copy(source_path, &input_path).expect("Failed to copy source file");

    if should_index {
        index(&input_path);
    }

    (temp_dir, input_path, output_path)
}

fn index(file: &str) {
    let mut command = Command::cargo_bin("nailpolish").unwrap();
    let _ = command.args(["index", file]).assert().success();
}

fn nailpolish_bin() -> Command {
    Command::cargo_bin("nailpolish").unwrap()
}

#[test]
fn summary() {
    let (_temp_dir, input, output) = make_temp_dir(SAMPLE_FASTQ, "input.fastq", true);

    let _ = nailpolish_bin()
        .args(["summary", &input, "-o", &output])
        .assert()
        .success();

    assert!(predicate::path::exists().eval(Path::new(&output)));
}

#[test]
fn consensus_1t_no_clustering() {
    let (_temp_dir, input, output) = make_temp_dir(SAMPLE_FASTQ, "input.fastq", true);

    let _ = nailpolish_bin()
        .args([
            "consensus",
            &input,
            "-o",
            &output,
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
    // let _ = Command::new("diff").args([&output, CORRECT_FILE]).unwrap();
}

#[test]
fn consensus_3t_no_clustering() {
    let (_temp_dir, input, output) = make_temp_dir(SAMPLE_FASTQ, "input.fastq", true);

    let _ = nailpolish_bin()
        .args([
            "consensus",
            &input,
            "-o",
            &output,
            "--threads",
            "3",
            "--report-original-header",
            "--report-original-reads",
            "--no-clustering",
        ])
        .assert()
        .success();

    // DISABLED due to bug in SPOA consensus algorithm
    // TODO: remove when bug is fixed
    // const CORRECT_FILE: &str = "tests/correct/consensus_no_cluster.fastq";
    // let _ = Command::new("diff").args([&output, CORRECT_FILE]).unwrap();
}

#[test]
fn extract_3() {
    let (_temp_dir, input, output) = make_temp_dir(SAMPLE_FASTQ, "input.fastq", true);

    let _ = nailpolish_bin()
        .args(["extract", &input, "-o", &output, "--group-size", "3"])
        .assert()
        .success();

    const CORRECT_FILE: &str = "tests/correct/extract_3.fastq";
    let cmp_cmd = format!("diff {} {}", &output, CORRECT_FILE);

    let _ = Command::new("bash").arg("-c").arg(&cmp_cmd).unwrap();
}

#[test]
fn consensus_3t_with_clustering() {
    let (_temp_dir, input, output) = make_temp_dir(SAMPLE_FASTQ, "input.fastq", true);

    let _ = nailpolish_bin()
        .args([
            "consensus",
            &input,
            "-o",
            &output,
            "--threads",
            "3",
            "--report-original-header",
            "--report-original-reads",
        ])
        .assert()
        .success();

    const CORRECT_FILE: &str = "tests/correct/consensus_with_cluster.fastq";

    let _ = Command::new("diff").args([&output, CORRECT_FILE]).unwrap();
}

#[test]
fn consensus_sorted() {
    let (_temp_dir, input, output) = make_temp_dir(SAMPLE_FASTQ, "input.fastq", true);

    let _ = nailpolish_bin()
        .args([
            "consensus",
            &input,
            "-o",
            &output,
            "--threads",
            "3",
            "--sort-by",
            "CB",
            "--report-original-header",
            "--report-original-reads",
        ])
        .assert()
        .success();

    const CORRECT_FILE: &str = "tests/correct/consensus_with_cluster.fastq";

    // we just look at the headers to see if they match up
    let cmp_cmd = format!(
        "diff \
        <(awk 'NR % 4 == 1' {CORRECT_FILE} | sort) \
        <(awk 'NR % 4 == 1' {output} | sort)"
    );
    let _ = Command::new("bash").args(["-c", &cmp_cmd]).unwrap();
}

#[test]
fn consensus_3t_gz_with_clustering() {
    let (_temp_dir, input, output) = make_temp_dir(SAMPLE_FASTQ_GZ, "input.fastq.gz", true);

    let _ = nailpolish_bin()
        .args([
            "consensus",
            &input,
            "-o",
            &output,
            "--threads",
            "3",
            "--report-original-header",
            "--report-original-reads",
        ])
        .assert()
        .success();

    const CORRECT_FILE: &str = "tests/correct/consensus_with_cluster.fastq";

    let _ = Command::new("diff").args([&output, CORRECT_FILE]).unwrap();
}

#[test]
fn consensus_out_of_order() {
    let (_temp_dir, input, output) = make_temp_dir(SAMPLE_FASTQ, "input.fastq", true);

    let _ = nailpolish_bin()
        .args([
            "consensus",
            &input,
            "-o",
            &output,
            "--threads",
            "3",
            "--report-original-header",
            "--report-original-reads",
        ])
        .assert()
        .success();
}

#[test]
fn consensus_with_cluster_file() {
    let (_temp_dir, input, output) = make_temp_dir(SAMPLE_FASTQ, "input.fastq", false);

    // Create index using cluster file
    let _ = nailpolish_bin()
        .args(["index", &input, "--clusters", "tests/data/clusters.txt"])
        .assert()
        .success();

    // Run consensus
    let _ = nailpolish_bin()
        .args([
            "consensus",
            &input,
            "-o",
            &output,
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
        <(awk 'NR % 4 == 2 || NR % 4 == 0' {output} | sort)"
    );
    let _ = Command::new("bash").args(["-c", &cmp_cmd]).unwrap();
}
