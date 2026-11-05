use crate::{TaskResult, assurance, run};
use std::fs;
use std::path::Path;

const TOOL_VERSION: &str = "0.13.1";
const NIGHTLY: &str = "nightly-2026-08-21";
const RUNS: usize = 4096;
const SEED: usize = 1;
const MAX_LENGTH: usize = 256;
const RSS_MIB: usize = 1024;
const INPUT_SAFETY_SECONDS: usize = 10;
const TARGET: &str = "macos_version";
const CORPUS: &str = "fuzz/corpus/macos_version/seeds.json";

fn completed(log: &str) -> bool {
    log.lines().any(|line| {
        let mut fields = line.split_whitespace();
        fields.next() == Some(format!("#{RUNS}").as_str()) && fields.next() == Some("DONE")
    })
}

pub(crate) fn run_campaign() -> TaskResult<()> {
    let source = assurance::source_digest(Path::new("."))?;
    let version = run("cargo", &["fuzz", "--version"], "fuzz-version")?;
    if String::from_utf8(version.stdout)?.trim() != format!("cargo-fuzz {TOOL_VERSION}") {
        return Err("The fuzz tool differs from the pinned version".into());
    }
    let seeds: Vec<Vec<u8>> = serde_json::from_slice(&fs::read(CORPUS)?)?;
    if seeds.is_empty() || seeds.iter().any(|seed| seed.len() > MAX_LENGTH) {
        return Err("Fuzz corpus is empty or exceeds the campaign input bound".into());
    }
    let root = std::env::current_dir()?;
    let workspace = root.join("build/fuzz-project");
    if workspace.try_exists()? {
        fs::remove_dir_all(&workspace)?;
    }
    crate::proof_controls::copy_workspace(&workspace)?;
    let fuzz_directory = workspace.join("fuzz");
    let fuzz_directory = fuzz_directory
        .to_str()
        .ok_or("Fuzz project path is not UTF-8")?;
    let target_directory = root.join("target/fuzz");
    let target_directory = target_directory
        .to_str()
        .ok_or("Fuzz build path is not UTF-8")?;
    let corpus = root.join("build/fuzz-corpus");
    if corpus.try_exists()? {
        fs::remove_dir_all(&corpus)?;
    }
    fs::create_dir(&corpus)?;
    fs::create_dir_all("build/fuzz-artifacts")?;
    for (index, seed) in seeds.iter().enumerate() {
        fs::write(corpus.join(index.to_string()), seed)?;
    }
    let corpus = corpus.to_str().ok_or("Fuzz corpus path is not UTF-8")?;
    let count = format!("-runs={RUNS}");
    let seed = format!("-seed={SEED}");
    let length = format!("-max_len={MAX_LENGTH}");
    let rss = format!("-rss_limit_mb={RSS_MIB}");
    let timeout = format!("-timeout={INPUT_SAFETY_SECONDS}");
    let artifacts = format!("-artifact_prefix={}/build/fuzz-artifacts/", root.display());
    let output = crate::run_with_environment(
        "rustup",
        &[
            "run",
            NIGHTLY,
            "cargo",
            "fuzz",
            "run",
            TARGET,
            corpus,
            "--fuzz-dir",
            fuzz_directory,
            "--target-dir",
            target_directory,
            "--sanitizer",
            "address",
            "--",
            &count,
            &seed,
            &length,
            &rss,
            &timeout,
            &artifacts,
        ],
        "fuzz-campaign",
        &[
            ("CARGO_PROFILE_RELEASE_LTO", "false"),
            ("CARGO_PROFILE_RELEASE_STRIP", "none"),
        ],
    )?;
    let mut log = output.stdout;
    log.extend(output.stderr);
    if !completed(&String::from_utf8(log)?) {
        return Err("The fuzz campaign did not complete its fixed run count".into());
    }
    if source != assurance::source_digest(Path::new("."))? {
        return Err("Source changed during fuzz verification".into());
    }
    fs::create_dir_all("build/evidence")?;
    fs::write(
        "build/evidence/fuzz.json",
        serde_json::to_vec_pretty(&serde_json::json!({
            "source_sha256": source, "tool": format!("cargo-fuzz {TOOL_VERSION}"), "nightly": NIGHTLY,
            "target": TARGET, "sanitizer": "address", "seed": SEED, "runs": RUNS, "max_input_bytes": MAX_LENGTH,
            "corpus_entries": seeds.len(), "corpus_sha256": assurance::hash_file(CORPUS)?,
            "log_sha256": assurance::hash_file("build/logs/fuzz-campaign.log")?
        }))?,
    )?;
    println!(
        "[qrscanner] {RUNS} fuzz runs completed after {} corpus cases",
        seeds.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_fuzz_campaign_requires_its_completed_run_count() {
        assert!(completed(
            "#4096\tDONE cov: 10\nDone 4096 runs in 1 second(s)\n"
        ));
        for log in [
            "",
            "#4095 DONE cov: 10\n",
            "#4096 NEW cov: 10\n",
            "ERROR: libFuzzer: timeout\n",
        ] {
            assert!(!completed(log));
        }
    }
}
