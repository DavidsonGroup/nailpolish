use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

const SAMPLE_FASTQ: &str = "tests/data/scmixology2_sample.fastq";

#[test]
fn index() {
    let temp = assert_fs::NamedTempFile::new("_index.tsv").unwrap();

    let mut command = Command::cargo_bin("nailpolish").unwrap();

    let _ = command
        .args(&["index", SAMPLE_FASTQ, "-o", temp.path().to_str().unwrap()])
        .assert()
        .success();

    // lazy way of checking that these files are the same
    // EXCEPT for the header, which contains unique date and runtime information
    let cmp_cmd = format!(
        "diff <(tail -n+2 tests/correct/index.tsv) <(tail -n+2 {})",
        temp.path().to_str().unwrap()
    );

    let _ = Command::new("bash").arg("-c").arg(&cmp_cmd).unwrap();

    temp.close().unwrap();
}

#[test]
fn summary() {
    let temp = assert_fs::NamedTempFile::new("_summary.html").unwrap();

    let mut command = Command::cargo_bin("nailpolish").unwrap();

    let _ = command
        .args(&[
            "summary",
            "--index",
            "tests/correct/index.tsv",
            "-o",
            temp.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    temp.assert(predicate::path::exists());
}

#[test]
fn consensus_1t() {
    let temp = assert_fs::NamedTempFile::new("consensus.fastq").unwrap();

    let mut command = Command::cargo_bin("nailpolish").unwrap();

    let _ = command
        .args(&[
            "consensus",
            "--index",
            "tests/correct/index.tsv",
            "--input",
            SAMPLE_FASTQ,
            "-o",
            temp.path().to_str().unwrap(),
            "--threads",
            "1",
        ])
        .assert()
        .success();

    const CORRECT_FILE: &str = "tests/correct/consensus.fastq";
    let cmp_cmd = format!("diff {} {}", temp.path().to_str().unwrap(), CORRECT_FILE);

    let _ = Command::new("bash").arg("-c").arg(&cmp_cmd).unwrap();

    temp.close().unwrap();
}

#[test]
fn consensus_4t() {
    let temp = assert_fs::NamedTempFile::new("consensus_4t.fastq").unwrap();

    let mut command = Command::cargo_bin("nailpolish").unwrap();

    let _ = command
        .args(&[
            "consensus",
            "--index",
            "tests/correct/index.tsv",
            "--input",
            SAMPLE_FASTQ,
            "-o",
            temp.path().to_str().unwrap(),
            "--threads",
            "4",
        ])
        .assert()
        .success();

    const CORRECT_FILE: &str = "tests/correct/consensus.fastq";
    let cmp_cmd = format!("diff {} {}", temp.path().to_str().unwrap(), CORRECT_FILE);

    let _ = Command::new("bash").arg("-c").arg(&cmp_cmd).unwrap();

    temp.close().unwrap();
}
