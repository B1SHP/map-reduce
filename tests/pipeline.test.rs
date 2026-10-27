use std::{
    io::Write,
    process::{Command, Stdio},
};

use map_reduce::Stats;

#[test]
fn should_merge_partitioned_statistics_into_pearson_correlation() {
    let first = [Stats::single(1.0, 2.0), Stats::single(2.0, 4.0)]
        .into_iter()
        .fold(Stats::default(), Stats::merge);
    let second = [Stats::single(3.0, 6.0), Stats::single(4.0, 8.0)]
        .into_iter()
        .fold(Stats::default(), Stats::merge);

    assert_eq!(first.merge(second).correlation(), Some(1.0));
}

#[test]
fn should_emit_one_hot_month_and_day_results() {
    let input = "7,5,mar,fri,86.2,26.2,94.3,5.1,8.2,51,6.7,0,0\n";
    let mapped = run(env!("CARGO_BIN_EXE_mapper"), input);

    assert!(mapped.contains("month_mar\t1,8.2,1,0,0,0"));
    assert!(mapped.contains("month_jan\t1,8.2,0,0,0,0"));
    assert!(mapped.contains("day_fri\t1,8.2,1,0,0,0"));
}

#[test]
fn should_keep_result_when_combiner_runs_before_reducer() {
    let input = "1,1,jan,mon,1,1,1,1,10,1,1,0,1\n2,2,feb,tue,2,2,2,2,20,2,2,0,2\n";
    let mapped = run(env!("CARGO_BIN_EXE_mapper"), input);
    let combined = run(env!("CARGO_BIN_EXE_combiner"), &mapped);
    let reduced = run(env!("CARGO_BIN_EXE_reducer"), &combined);

    assert!(reduced.contains("x\tr=1.000000\tn=2"));
    assert!(!reduced.contains("temp\tr="));
}

#[test]
fn should_mark_zero_variance_as_undefined() {
    let stats = [Stats::single(10.0, 1.0), Stats::single(10.0, 2.0)]
        .into_iter()
        .fold(Stats::default(), Stats::merge);

    assert_eq!(stats.correlation(), None);
}

fn run(binary: &str, input: &str) -> String {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap()
}
