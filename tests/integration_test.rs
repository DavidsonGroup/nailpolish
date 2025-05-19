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

    const CORRECT_FILE: &str = "tests/correct/consensus_no_cluster.fastq";
    let cmp_cmd = format!("diff {} {}", temp.path().to_str().unwrap(), CORRECT_FILE);

    let _ = Command::new("bash").arg("-c").arg(&cmp_cmd).unwrap();

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

    const CORRECT_FILE: &str = "tests/correct/consensus_no_cluster.fastq";
    let cmp_cmd = format!("diff {} {}", temp.path().to_str().unwrap(), CORRECT_FILE);

    let _ = Command::new("bash").arg("-c").arg(&cmp_cmd).unwrap();

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
    let cmp_cmd = format!("diff {} {}", temp.path().to_str().unwrap(), CORRECT_FILE);

    let _ = Command::new("bash").arg("-c").arg(&cmp_cmd).unwrap();

    temp.close().unwrap();
}
