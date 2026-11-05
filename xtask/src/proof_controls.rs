use crate::{TaskResult, assurance};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn expected_failure(success: bool, output: &str, assertion: &str) -> bool {
    qrscanner_verification::counterexample_rejected(qrscanner_verification::CounterexampleFacts {
        process_failed: !success,
        named_assertion_failed: output
            .lines()
            .any(|line| line.trim() == format!("Failed Checks: {assertion}")),
        verification_failed: output
            .lines()
            .any(|line| line.trim() == "VERIFICATION:- FAILED"),
        unwinding_failed: output.contains("unwinding assertion"),
    })
}

struct Fault {
    name: &'static str,
    file: &'static str,
    before: &'static str,
    after: &'static str,
    package: &'static str,
    harness: &'static str,
    assertion: &'static str,
}

const FAULTS: &[Fault] = &[
    Fault {
        name: "suppressed-capture",
        file: "crates/core/src/lib.rs",
        before: "effect: Effect::Capture,",
        after: "effect: Effect::None,",
        package: "qrscanner-core",
        harness: "begin_is_total_and_makes_progress",
        assertion: "BEGIN_PROGRESS",
    },
    Fault {
        name: "wrapped-generation",
        file: "crates/core/src/lib.rs",
        before: "self.0.checked_add(1)",
        after: "Some(self.0.wrapping_add(1))",
        package: "qrscanner-core",
        harness: "begin_is_total_and_makes_progress",
        assertion: "BEGIN_PROGRESS",
    },
    Fault {
        name: "automatic-open",
        file: "crates/core/src/lib.rs",
        before: "effect: Effect::PresentResults,",
        after: "effect: Effect::OpenSelected,",
        package: "qrscanner-core",
        harness: "completion_and_failure_have_exact_outcomes",
        assertion: "COMPLETION_CONTRACT",
    },
    Fault {
        name: "stale-completion",
        file: "crates/core/src/lib.rs",
        before: "Event::Completed { id, found } => {\n            if id.raw() != state.id.raw() || !matches!(state.phase, Phase::Scanning)",
        after: "Event::Completed { id: _, found } => {\n            if !matches!(state.phase, Phase::Scanning)",
        package: "qrscanner-core",
        harness: "completion_and_failure_have_exact_outcomes",
        assertion: "COMPLETION_CONTRACT",
    },
    Fault {
        name: "incorrect-wire-tag",
        file: "crates/ffi/src/lib.rs",
        before: "const QR_EVENT_CANCEL: u32 = 1;",
        after: "const QR_EVENT_CANCEL: u32 = 7;",
        package: "qrscanner-ffi",
        harness: "wire_decoders_match_the_documented_numeric_contract",
        assertion: "WIRE_EVENT_MAPPING",
    },
    Fault {
        name: "missing-artifact-binding",
        file: "crates/verification/src/lib.rs",
        before: "&& facts.artifacts_bound",
        after: "&& true",
        package: "qrscanner-verification",
        harness: "incomplete_verification_cannot_authorize_signing",
        assertion: "VERIFICATION_GATE",
    },
    Fault {
        name: "accepted-mutation-timeout",
        file: "crates/verification/src/lib.rs",
        before: "&& counts.timed_out == 0",
        after: "&& true",
        package: "qrscanner-verification",
        harness: "mutation_counts_are_complete_without_arithmetic_overflow",
        assertion: "MUTATION_COMPLETENESS",
    },
];

#[derive(Serialize)]
pub(crate) struct RejectedFault {
    name: &'static str,
    production_file: &'static str,
    original_sha256: String,
    mutated_sha256: String,
    harness: &'static str,
    assertion: &'static str,
    log_sha256: String,
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            eprintln!(
                "Proof control cleanup failed for {}: {error}",
                self.0.display()
            );
        }
    }
}

fn copy_source(from: &Path, to: &Path) -> TaskResult<()> {
    fs::create_dir(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_source(&entry.path(), &to.join(entry.file_name()))?;
        } else if kind.is_file() {
            fs::copy(entry.path(), to.join(entry.file_name()))?;
        } else {
            return Err("Proof control source contains a symlink or special file".into());
        }
    }
    Ok(())
}

pub(crate) fn copy_workspace(destination: &Path) -> TaskResult<()> {
    fs::create_dir(destination)?;
    for name in ["Cargo.toml", "Cargo.lock"] {
        fs::copy(name, destination.join(name))?;
    }
    for name in ["crates", "fuzz", "xtask"] {
        copy_source(Path::new(name), &destination.join(name))?;
    }
    Ok(())
}

fn inject(source: &str, fault: &Fault) -> TaskResult<String> {
    if source.matches(fault.before).count() != 1 {
        return Err(format!(
            "Proof control must match one production location: {}",
            fault.name
        )
        .into());
    }
    Ok(source.replacen(fault.before, fault.after, 1))
}

pub(crate) fn run() -> TaskResult<Vec<RejectedFault>> {
    let root = std::env::current_dir()?;
    let scratch = Scratch(root.join(format!("build/tmp/proof-controls-{}", std::process::id())));
    copy_workspace(&scratch.0)?;
    let mut rejected = Vec::new();
    for fault in FAULTS {
        let original = fs::read_to_string(fault.file)?;
        let destination = scratch.0.join(fault.file);
        fs::write(&destination, inject(&original, fault)?)?;
        let mutated_sha256 = assurance::hash_file(&destination)?;
        let output = Command::new("cargo")
            .args(["kani", "-p", fault.package, "--harness", fault.harness])
            .current_dir(&scratch.0)
            .env("CARGO_TARGET_DIR", root.join("target/proof-controls"))
            .env("CARGO_NET_OFFLINE", "true")
            .output()?;
        fs::write(&destination, original)?;
        let mut log = output.stdout;
        log.extend(output.stderr);
        let log_path = format!("build/logs/kani-control-{}.log", fault.name);
        fs::write(&log_path, &log)?;
        if !expected_failure(
            output.status.success(),
            &String::from_utf8(log)?,
            fault.assertion,
        ) {
            return Err(format!(
                "Production fault lacked its expected counterexample: {}; see {log_path}",
                fault.name
            )
            .into());
        }
        println!(
            "[qrscanner] rejected production fault: {} ({})",
            fault.name, fault.assertion
        );
        rejected.push(RejectedFault {
            name: fault.name,
            production_file: fault.file,
            original_sha256: assurance::hash_file(fault.file)?,
            mutated_sha256,
            harness: fault.harness,
            assertion: fault.assertion,
            log_sha256: assurance::hash_file(log_path)?,
        });
    }
    fs::remove_dir_all(&scratch.0)?;
    Ok(rejected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controls_require_the_named_counterexample_instead_of_a_tool_error() {
        let rejected = "Failed Checks: BEGIN_PROGRESS\nVERIFICATION:- FAILED\n";
        assert!(expected_failure(false, rejected, "BEGIN_PROGRESS"));
        assert!(!expected_failure(true, rejected, "BEGIN_PROGRESS"));
        assert!(!expected_failure(
            false,
            "error: compilation failed",
            "BEGIN_PROGRESS"
        ));
        assert!(!expected_failure(false, rejected, "OTHER_ASSERTION"));
        assert!(!expected_failure(
            false,
            "Check: BEGIN_PROGRESS\nVERIFICATION:- FAILED\n",
            "BEGIN_PROGRESS"
        ));
        assert!(!expected_failure(
            false,
            "Failed Checks: BEGIN_PROGRESS\nunwinding assertion\nVERIFICATION:- FAILED\n",
            "BEGIN_PROGRESS"
        ));
    }

    #[test]
    fn fault_injection_refuses_missing_or_ambiguous_locations() {
        let fault = &FAULTS[0];
        assert!(inject("unrelated", fault).is_err());
        assert!(inject(&fault.before.repeat(2), fault).is_err());
        assert_ne!(
            inject(fault.before, fault).ok().as_deref(),
            Some(fault.before)
        );
    }
}
