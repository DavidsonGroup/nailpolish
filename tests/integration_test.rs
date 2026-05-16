// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Group.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

// HOW TO USE THESE TESTS:
// To run tests:
//   $ cargo nextest r
// To run a specific test:
//   $ cargo nextest r <test_name>
// To persist the results of tests:
//   $ TEST_PERSIST_FILES=1 cargo nextest r <test_name>

use assert_cmd::Command;
use assert_fs::fixture::TempDir;
use predicates::prelude::*;
use std::path::Path;

const SAMPLE_FASTQ: &str = "tests/data/scmixology2_sample.fastq";
const SAMPLE_FASTQ_GZ: &str = "tests/data/scmixology2_sample.fastq.gz";
const PARTIAL_CLUSTERS: &str = "tests/data/partial_clusters.txt";
const SMALL_LARGE_GROUP_FASTQ: &str = "tests/data/small_large_group.fastq";

fn make_temp_dir(
    source_path: &str,
    input_name: &str,
    output_name: Option<&str>,
    should_index: bool,
) -> (assert_fs::TempDir, String, String) {
    let temp_dir = TempDir::new_in("tests/")
        .unwrap()
        .into_persistent_if(std::env::var_os("TEST_PERSIST_FILES").is_some());
    let input_path = temp_dir
        .path()
        .join(input_name)
        .to_string_lossy()
        .to_string();
    let output_path = temp_dir
        .path()
        .join(output_name.unwrap_or("output.fastq"))
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
    let (_temp_dir, input, output) =
        make_temp_dir(SAMPLE_FASTQ, "input.fastq", Some("summary.html"), true);

    let _ = nailpolish_bin()
        .args(["summary", &input, "-o", &output])
        .assert()
        .success();

    assert!(predicate::path::exists().eval(Path::new(&output)));
}

#[test]
fn consensus_1t_no_clustering() {
    let (_temp_dir, input, output) = make_temp_dir(SAMPLE_FASTQ, "input.fastq", None, true);

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
    let (_temp_dir, input, output) = make_temp_dir(SAMPLE_FASTQ, "input.fastq", None, true);

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
    let (_temp_dir, input, output) = make_temp_dir(SAMPLE_FASTQ, "input.fastq", None, true);

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
    let (_temp_dir, input, output) = make_temp_dir(SAMPLE_FASTQ, "input.fastq", None, true);

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
fn large_group_drop() {
    let (_temp_dir, input, output) = make_temp_dir(SMALL_LARGE_GROUP_FASTQ, "input.fastq", None, true);

    let _ = nailpolish_bin()
        .args([
            "consensus",
            &input,
            "-o",
            &output,
            "--threads",
            "1",
            "--no-clustering",
            "--max-group-size",
            "2",
            "--large-group-method",
            "drop",
        ])
        .assert()
        .success();

    const CORRECT_FILE: &str = "tests/correct/large_group_drop.fastq";
    let _ = Command::new("diff").args([&output, CORRECT_FILE]).unwrap();
}

#[test]
fn large_group_sample() {
    let (_temp_dir, input, output) = make_temp_dir(SMALL_LARGE_GROUP_FASTQ, "input.fastq", None, true);

    let _ = nailpolish_bin()
        .args([
            "consensus",
            &input,
            "-o",
            &output,
            "--threads",
            "1",
            "--no-clustering",
            "--max-group-size",
            "2",
            "--large-group-method",
            "sample",
        ])
        .assert()
        .success();

    const CORRECT_FILE: &str = "tests/correct/large_group_sample.fastq";
    let _ = Command::new("diff").args([&output, CORRECT_FILE]).unwrap();
}

#[test]
fn large_group_longest() {
    let (_temp_dir, input, output) = make_temp_dir(SMALL_LARGE_GROUP_FASTQ, "input.fastq", None, true);

    let _ = nailpolish_bin()
        .args([
            "consensus",
            &input,
            "-o",
            &output,
            "--threads",
            "1",
            "--no-clustering",
            "--max-group-size",
            "2",
            "--large-group-method",
            "longest",
        ])
        .assert()
        .success();

    const CORRECT_FILE: &str = "tests/correct/large_group_longest.fastq";
    let _ = Command::new("diff").args([&output, CORRECT_FILE]).unwrap();
}

#[test]
fn consensus_sorted() {
    let (_temp_dir, input, output) = make_temp_dir(SAMPLE_FASTQ, "input.fastq", None, true);

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
    let (_temp_dir, input, output) = make_temp_dir(SAMPLE_FASTQ_GZ, "input.fastq.gz", None, true);

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
    let (_temp_dir, input, output) = make_temp_dir(SAMPLE_FASTQ, "input.fastq", None, true);

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
fn index_cluster_file_skip_unmatched_errors_without_flag() {
    let (_temp_dir, input, _) = make_temp_dir(SAMPLE_FASTQ, "input.fastq", None, false);

    nailpolish_bin()
        .args(["index", &input, "--clusters", PARTIAL_CLUSTERS])
        .assert()
        .failure();
}

#[test]
fn index_cluster_file_skip_unmatched_succeeds_with_flag() {
    let (_temp_dir, input, _) = make_temp_dir(SAMPLE_FASTQ, "input.fastq", None, false);

    nailpolish_bin()
        .args([
            "index",
            &input,
            "--clusters",
            PARTIAL_CLUSTERS,
            "--skip-unmatched",
        ])
        .assert()
        .success();
}

#[test]
fn consensus_with_cluster_file() {
    let (_temp_dir, input, output) = make_temp_dir(SAMPLE_FASTQ, "input.fastq", None, false);

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

    let _ = Command::new("diff").args([&output, CORRECT_FILE]).unwrap();
}
