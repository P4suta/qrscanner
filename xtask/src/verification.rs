use crate::{TaskResult, assurance, run};
use sha2::Digest;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const IMAGE_RECORD: &str = "build/evidence/verification-image.txt";
const RECIPE_RECORD: &str = "build/evidence/verification-recipe.txt";
const SECONDS: u64 = 900;
const SHA256_HEX_LENGTH: usize = 64;
const IMAGE_ID_PREFIX: &str = "sha256:";
const POLL_INTERVAL_MILLISECONDS: u64 = 250;
const MUTATION_TEMP_DIRECTORY: &str = "/tmp";
const MUTATION_TOOL_VERSION: &str = "27.1.0";
const CONTAINER_TIMEOUT_EXIT: i32 = 124;
const KILLED_PROCESS_EXIT: i32 = 137;
const MUTATION_FILES: &[&str] = &[
    "crates/core/src/lib.rs",
    "crates/ffi/src/lib.rs",
    "crates/verification/src/lib.rs",
];

#[derive(Clone, Copy)]
pub(crate) enum Campaign {
    Proofs,
    Mutations,
    MutationPlan,
    Fuzz,
}

impl Campaign {
    const fn command(self) -> &'static str {
        match self {
            Self::Proofs => "proofs-inner",
            Self::Mutations => "mutants-inner",
            Self::MutationPlan => "mutation-plan-inner",
            Self::Fuzz => "fuzz-inner",
        }
    }

    const fn report(self) -> &'static str {
        match self {
            Self::Proofs => "proofs.json",
            Self::Mutations => "mutations.json",
            Self::MutationPlan => "mutation-plan.json",
            Self::Fuzz => "fuzz.json",
        }
    }
}

pub(crate) fn prepare(cache: Option<&str>) -> TaskResult<()> {
    fs::create_dir_all("build/evidence")?;
    let cache_context;
    let mut arguments = vec!["build", "--platform", "linux/amd64", "--file"];
    if let Some(cache) = cache {
        let (reference, digest) = cache
            .rsplit_once("@sha256:")
            .ok_or("A tools cache must name its tag and exact image ID")?;
        if digest.len() != SHA256_HEX_LENGTH || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("Invalid tools cache digest".into());
        }
        let inspected = run(
            "docker",
            &["image", "inspect", reference, "--format", "{{.Id}}"],
            "inspect-tools-cache",
        )?;
        if String::from_utf8(inspected.stdout)?.trim() != format!("sha256:{digest}") {
            return Err("The tools cache tag does not match its declared image ID".into());
        }
        cache_context = format!("verification-tools=docker-image://{reference}");
        arguments.extend([
            "verification/cached-tools.Dockerfile",
            "--build-context",
            &cache_context,
        ]);
    } else {
        arguments.push("verification/Dockerfile");
    }
    arguments.extend([
        "--iidfile",
        IMAGE_RECORD,
        "--tag",
        "qrscanner-verification:local",
        ".",
    ]);
    run("docker", &arguments, "prepare-verification")?;
    fs::write(RECIPE_RECORD, recipe_digest()?)?;
    Ok(())
}

fn recipe_digest() -> TaskResult<String> {
    let mut input = Vec::new();
    for path in [
        "verification/Dockerfile",
        "verification/cached-tools.Dockerfile",
        ".dockerignore",
        "Cargo.lock",
        "Cargo.toml",
        "crates/core/Cargo.toml",
        "crates/ffi/Cargo.toml",
        "crates/verification/Cargo.toml",
        "fuzz/Cargo.toml",
        "xtask/Cargo.toml",
    ] {
        input.extend_from_slice(&fs::read(path)?);
    }
    Ok(format!("{:x}", sha2::Sha256::digest(input)))
}

pub(crate) fn image() -> TaskResult<String> {
    if fs::read_to_string(RECIPE_RECORD).ok().as_deref() != Some(&recipe_digest()?) {
        return Err(
            "The verification image is missing or stale; run mise x -- just prepare-verification"
                .into(),
        );
    }
    let image = fs::read_to_string(IMAGE_RECORD)?.trim().to_owned();
    if image.len() != IMAGE_ID_PREFIX.len() + SHA256_HEX_LENGTH
        || !image.starts_with(IMAGE_ID_PREFIX)
        || !image[IMAGE_ID_PREFIX.len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("The verification image must have an immutable SHA-256 identity".into());
    }
    Ok(image)
}

pub(crate) fn bounded(campaign: Campaign) -> TaskResult<()> {
    let command = campaign.command();
    let source = assurance::source_digest(Path::new("."))?;
    for record in [
        format!("build/evidence/{}", campaign.report()),
        format!("build/evidence/commands/{command}-container.json"),
    ] {
        if Path::new(&record).exists() {
            fs::remove_file(record)?;
        }
    }
    let image = image()?;
    let root = std::env::current_dir()?;
    let output_root = root.join("build/verification");
    let target = format!(
        "qrscanner-verification-{:x}",
        sha2::Sha256::digest(root.as_os_str().as_encoded_bytes())
    );
    fs::create_dir_all(&output_root)?;
    let source_mount = format!("{}:/source:ro", root.display());
    let output_mount = format!("{}:/source/build", output_root.display());
    let target_mount = format!("{target}:/source/target");
    let name = format!("qrscanner-verification-{}", std::process::id());
    let log_path = format!("build/logs/{command}-container.log");
    let log = fs::File::create(&log_path)?;
    let timeout = SECONDS.to_string();
    println!("[qrscanner] {command} in {image}; 2 CPUs, 2048 MiB, {SECONDS} second safety limit");
    let mut child = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--name",
            &name,
            "--platform",
            "linux/amd64",
            "--memory",
            "2048m",
            "--memory-swap",
            "2048m",
            "--cpus",
            "2",
            "--pids-limit",
            "256",
            "--network",
            "none",
            "--env",
            "CARGO_INCREMENTAL=0",
            "--security-opt",
            "no-new-privileges",
            "--cap-drop",
            "ALL",
            "--volume",
            &source_mount,
            "--volume",
            &output_mount,
            "--volume",
            &target_mount,
            "--workdir",
            "/source",
            &image,
            "timeout",
            &timeout,
            "cargo",
            "run",
            "--offline",
            "--locked",
            "-p",
            "xtask",
            "--",
            command,
        ])
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .spawn()?;
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            if status.code() == Some(CONTAINER_TIMEOUT_EXIT)
                || status.code() == Some(KILLED_PROCESS_EXIT)
            {
                return Err(format!(
                    "{command} is inconclusive after a container resource stop; see {log_path}"
                )
                .into());
            }
            if !status.success() {
                return Err(format!("{command} did not pass ({status}); see {log_path}").into());
            }
            break;
        }
        if started.elapsed() >= Duration::from_secs(SECONDS) {
            let cleanup = Command::new("docker").args(["kill", &name]).output()?;
            if !cleanup.status.success() {
                child.kill()?;
            }
            child.wait()?;
            return Err(format!(
                "{command} is inconclusive after the container safety limit; see {log_path}"
            )
            .into());
        }
        std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MILLISECONDS));
    }
    if source != assurance::source_digest(Path::new("."))? {
        return Err("Source changed during verification; no evidence was accepted".into());
    }
    assurance::record_command(
        "docker",
        &["run", &image, command],
        &format!("{command}-container"),
    )?;
    fs::copy(
        output_root.join("evidence").join(campaign.report()),
        root.join("build/evidence").join(campaign.report()),
    )?;
    Ok(())
}

fn mutation_arguments() -> Vec<&'static str> {
    let mut arguments = vec![
        "mutants",
        "--cargo-arg=--offline",
        "--cargo-arg=--locked",
        "--package",
        "qrscanner-core",
        "--package",
        "qrscanner-ffi",
        "--package",
        "qrscanner-verification",
    ];
    for file in MUTATION_FILES {
        arguments.extend(["--file", file]);
    }
    arguments
}

pub(crate) fn mutation_plan() -> TaskResult<()> {
    let mut arguments = mutation_arguments();
    arguments.extend(["--list", "--json"]);
    let output = run("cargo", &arguments, "mutation-plan")?;
    let inventory: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)?;
    if inventory.is_empty() {
        return Err("Mutation plan is empty".into());
    }
    if !mutation_files_complete(&inventory) {
        return Err("Mutation plan omits a required production file".into());
    }
    fs::create_dir_all("build/evidence")?;
    fs::write(
        "build/evidence/mutation-plan.json",
        serde_json::to_vec_pretty(&serde_json::json!({
            "source_sha256": assurance::source_digest(Path::new("."))?, "total": inventory.len(),
            "tool": format!("cargo-mutants {MUTATION_TOOL_VERSION}"), "files": MUTATION_FILES, "inventory": inventory
        }))?,
    )?;
    println!(
        "[qrscanner] planned {} mutants in {} production files",
        inventory.len(),
        MUTATION_FILES.len()
    );
    Ok(())
}

pub(crate) fn mutations() -> TaskResult<()> {
    let source = assurance::source_digest(Path::new("."))?;
    let evidence = Path::new("build/evidence/mutations.json");
    if evidence.exists() {
        fs::remove_file(evidence)?;
    }
    let mut arguments = mutation_arguments();
    arguments.extend([
        "--no-shuffle",
        "--jobs",
        "2",
        "--timeout",
        "60",
        "--output",
        "build/mutants",
    ]);
    let output = crate::invoke_with_environment(
        "cargo",
        &arguments,
        "mutation-campaign",
        &[("TMPDIR", MUTATION_TEMP_DIRECTORY)],
    )?;
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read("build/mutants/mutants.out/outcomes.json")?)?;
    let total = report["total_mutants"]
        .as_u64()
        .ok_or("Missing mutation inventory")?;
    if report["timeout"].as_u64().is_some_and(|count| count > 0) {
        return Err(
            "Mutation verification is inconclusive after a timeout; inspect build/mutants".into(),
        );
    }
    if !output.status.success() {
        return Err(
            "Mutation verification did not pass; inspect build/logs/mutation-campaign.log".into(),
        );
    }
    if !mutation_report_complete(&report) {
        return Err(
            "Mutation coverage is empty, incomplete, or has survivors; inspect build/mutants"
                .into(),
        );
    }
    let inventory: serde_json::Value =
        serde_json::from_slice(&fs::read("build/mutants/mutants.out/mutants.json")?)?;
    let entries = inventory
        .as_array()
        .ok_or("Missing mutation inventory entries")?;
    let names: std::collections::BTreeSet<_> = entries
        .iter()
        .filter_map(|entry| entry["name"].as_str())
        .collect();
    if u64::try_from(entries.len())? != total
        || !mutation_files_complete(entries)
        || names.len() != entries.len()
        || !report["outcomes"]
            .as_array()
            .ok_or("Missing mutation outcomes")?
            .iter()
            .filter_map(|outcome| outcome["scenario"]["Mutant"]["name"].as_str())
            .all(|name| names.contains(name))
    {
        return Err("Mutation outcomes do not match the complete inventory".into());
    }
    if source != assurance::source_digest(Path::new("."))? {
        return Err("Source changed during mutation verification; no evidence was issued".into());
    }
    assurance::record_command("cargo", &["mutants"], "mutation-campaign")?;
    fs::create_dir_all("build/evidence")?;
    fs::write(
        evidence,
        serde_json::to_vec_pretty(&serde_json::json!({
            "source_sha256": source,
            "tool": format!("cargo-mutants {MUTATION_TOOL_VERSION}"), "total": total, "report": report,
            "files": MUTATION_FILES
        }))?,
    )?;
    Ok(())
}

fn mutation_files_complete(entries: &[serde_json::Value]) -> bool {
    let files: std::collections::BTreeSet<_> = entries
        .iter()
        .filter_map(|entry| entry["file"].as_str())
        .collect();
    entries.iter().all(|entry| entry["file"].as_str().is_some())
        && files == MUTATION_FILES.iter().copied().collect()
}

fn mutation_report_complete(report: &serde_json::Value) -> bool {
    let Some(total) = report["total_mutants"].as_u64() else {
        return false;
    };
    let Some(caught) = report["caught"].as_u64() else {
        return false;
    };
    let Some(unviable) = report["unviable"].as_u64() else {
        return false;
    };
    let Some(outcomes) = report["outcomes"].as_array() else {
        return false;
    };
    if report["cargo_mutants_version"].as_str() != Some(MUTATION_TOOL_VERSION) {
        return false;
    }
    let mut baselines = 0;
    let mut caught_outcomes = 0;
    let mut unviable_outcomes = 0;
    let mut names = std::collections::BTreeSet::new();
    for outcome in outcomes {
        if outcome["scenario"].as_str() == Some("Baseline") {
            if outcome["summary"].as_str() != Some("Success") {
                return false;
            }
            baselines += 1;
        } else {
            let Some(name) = outcome["scenario"]["Mutant"]["name"].as_str() else {
                return false;
            };
            if name.is_empty() || !names.insert(name) {
                return false;
            }
            match outcome["summary"].as_str() {
                Some("CaughtMutant") => caught_outcomes += 1,
                Some("Unviable") => unviable_outcomes += 1,
                _ => return false,
            }
        }
    }
    let (Some(missed), Some(timed_out), Some(unexpected_success), Ok(outcomes)) = (
        report["missed"].as_u64(),
        report["timeout"].as_u64(),
        report["success"].as_u64(),
        u64::try_from(outcomes.len()),
    ) else {
        return false;
    };
    qrscanner_verification::mutations_complete(qrscanner_verification::MutationCounts {
        total,
        caught,
        unviable,
        missed,
        timed_out,
        unexpected_success,
        outcomes,
        baselines,
        caught_outcomes,
        unviable_outcomes,
    })
}

#[cfg(test)]
mod tests {
    use super::{MUTATION_FILES, mutation_files_complete, mutation_report_complete};
    use serde_json::json;

    fn complete_report() -> serde_json::Value {
        json!({
            "cargo_mutants_version": "27.1.0", "total_mutants": 1,
            "caught": 1, "missed": 0, "unviable": 0, "timeout": 0, "success": 0,
            "outcomes": [
                {"scenario": "Baseline", "summary": "Success"},
                {"scenario": {"Mutant": {"name": "contract"}}, "summary": "CaughtMutant"}
            ]
        })
    }

    #[test]
    fn mutation_evidence_requires_a_complete_nonempty_successful_campaign() {
        assert!(mutation_report_complete(&complete_report()));
        for (key, value) in [
            ("total_mutants", json!(0)),
            ("total_mutants", json!(2)),
            ("missed", json!(1)),
            ("timeout", json!(1)),
            ("caught", json!(0)),
            ("cargo_mutants_version", json!("unknown")),
            ("outcomes", json!([])),
            (
                "outcomes",
                json!([{"scenario": "Baseline", "summary": "Failure"}]),
            ),
        ] {
            let mut report = complete_report();
            report[key] = value;
            assert!(
                !mutation_report_complete(&report),
                "accepted incomplete {key}"
            );
        }
    }

    #[test]
    fn mutation_inventory_covers_each_required_file_and_refuses_unknown_files() {
        let entries: Vec<_> = MUTATION_FILES
            .iter()
            .map(|file| json!({"file": file}))
            .collect();
        assert!(mutation_files_complete(&entries));
        for index in 0..entries.len() {
            let mut missing = entries.clone();
            missing.remove(index);
            assert!(!mutation_files_complete(&missing));
        }
        for invalid in [json!({"file": "unselected.rs"}), json!({})] {
            let mut altered = entries.clone();
            altered.push(invalid);
            assert!(!mutation_files_complete(&altered));
        }
    }
}
