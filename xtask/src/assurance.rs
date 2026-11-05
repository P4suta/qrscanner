use crate::{TaskResult, cargo, invoke};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const SHA256_HEX_LENGTH: usize = 64;
pub(crate) const RUST_RELEASE: &str = "1.99.1";

const REQUIRED_CHECKS: &[&str] = &[
    "rust-format",
    "rust-clippy",
    "rust-tests",
    "dependency-policy",
    "advisories-pinned",
    "secrets",
    "swift-lint",
    "localization-check",
    "native-Release",
    "compile-tests-release-arm64",
    "native-tests-release-arm64",
    "compile-tests-release-x86_64",
    "native-tests-release-x86_64",
    "proofs-inner-container",
    "mutants-inner-container",
];

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommandRecord {
    name: String,
    command: Vec<String>,
    source_sha256: String,
    log_sha256: String,
}

pub(crate) fn record_command(program: &str, args: &[&str], name: &str) -> TaskResult<()> {
    let record = CommandRecord {
        name: name.to_owned(),
        command: std::iter::once(program)
            .chain(args.iter().copied())
            .map(str::to_owned)
            .collect(),
        source_sha256: source_digest(Path::new("."))?,
        log_sha256: hash_file(format!("build/logs/{name}.log"))?,
    };
    fs::create_dir_all("build/evidence/commands")?;
    fs::write(
        format!("build/evidence/commands/{name}.json"),
        serde_json::to_vec_pretty(&record)?,
    )?;
    Ok(())
}

pub(crate) const RELEASE_BINARY: &str =
    "build/xcode/Build/Products/Release/QRScanner.app/Contents/MacOS/QRScanner";
pub(crate) const RECEIPT: &str = "build/evidence/verification.json";
const HARNESS_NAMES: [&str; 10] = [
    "opening_requires_current_confirmed_web_result",
    "cancellation_rejects_every_late_completion",
    "stale_events_have_no_effect",
    "generation_is_monotonic_and_never_wraps",
    "busy_scans_cannot_capture_again",
    "asynchronous_effects_require_the_current_expected_phase",
    "wire_inputs_are_validated_and_opening_is_confirmed",
    "icon_lengths_are_checked",
    "release_receipts_must_match_and_be_accepted",
    "incomplete_verification_cannot_authorize_signing",
];

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VerificationReceipt {
    pub(crate) source_sha256: String,
    pub(crate) binary_sha256: String,
    pub(crate) checks: Vec<String>,
    pub(crate) advisory_freshness: String,
    pub(crate) resources_sha256: String,
    pub(crate) commands: Vec<CommandRecord>,
    pub(crate) tools: serde_json::Value,
    pub(crate) proofs_sha256: String,
    pub(crate) mutations_sha256: String,
}

impl VerificationReceipt {
    fn validate(&self, source: &str, binary: &str) -> TaskResult<()> {
        let facts = crate::release_rules::VerificationFacts {
            identities_match: self.source_sha256 == source && self.binary_sha256 == binary,
            checks_complete: self.checks.len() == REQUIRED_CHECKS.len()
                && REQUIRED_CHECKS
                    .iter()
                    .all(|name| self.checks.iter().filter(|item| item == name).count() == 1),
            commands_match: self.commands.len() == REQUIRED_CHECKS.len()
                && REQUIRED_CHECKS.iter().all(|name| {
                    self.commands
                        .iter()
                        .filter(|command| command.name == *name)
                        .count()
                        == 1
                })
                && self.commands.iter().all(|command| {
                    command.source_sha256 == source
                        && !command.command.is_empty()
                        && command.log_sha256.len() == SHA256_HEX_LENGTH
                }),
            tools_match: self.tools["rust"]
                .as_str()
                .is_some_and(|value| value.contains(&format!("rustc {RUST_RELEASE}")))
                && self.tools["swift"]
                    .as_str()
                    .is_some_and(|value| !value.is_empty())
                && self.tools["sdk"]
                    .as_str()
                    .is_some_and(|value| !value.is_empty())
                && self.tools["image"]
                    .as_str()
                    .is_some_and(|value| value.starts_with("sha256:")),
            artifacts_bound: self.proofs_sha256.len() == SHA256_HEX_LENGTH
                && self.mutations_sha256.len() == SHA256_HEX_LENGTH
                && self.resources_sha256.len() == SHA256_HEX_LENGTH,
        };
        if !crate::release_rules::verification_complete(facts) {
            return Err(
                "Verification evidence is incomplete or stale; run mise x -- just verify-release"
                    .into(),
            );
        }
        Ok(())
    }
}

pub(crate) fn hash_file(path: impl AsRef<Path>) -> TaskResult<String> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

fn source_files(root: &Path, relative: &Path, files: &mut Vec<PathBuf>) -> TaskResult<()> {
    for entry in fs::read_dir(root.join(relative))? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".DS_Store" {
            continue;
        }
        if relative.as_os_str().is_empty()
            && ["build", "target", ".git", ".cache"]
                .iter()
                .any(|item| name == *item)
        {
            continue;
        }
        let path = relative.join(&name);
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(format!("Source symlinks are not supported: {}", path.display()).into());
        }
        if kind.is_dir() {
            source_files(root, &path, files)?;
        }
        if kind.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

pub(crate) fn source_digest(root: &Path) -> TaskResult<String> {
    let mut files = Vec::new();
    source_files(root, Path::new(""), &mut files)?;
    files.sort();
    if files.is_empty() {
        return Err("Source inventory is empty".into());
    }
    let mut hash = Sha256::new();
    for path in files {
        let name = path.to_str().ok_or("Source path is not UTF-8")?.as_bytes();
        let bytes = fs::read(root.join(&path))?;
        hash.update(u64::try_from(name.len())?.to_be_bytes());
        hash.update(name);
        hash.update(u64::try_from(bytes.len())?.to_be_bytes());
        hash.update(bytes);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub(crate) fn save_receipt() -> TaskResult<()> {
    let source = source_digest(Path::new("."))?;
    let mut commands = Vec::new();
    for name in REQUIRED_CHECKS {
        commands.push(serde_json::from_slice::<CommandRecord>(&fs::read(
            format!("build/evidence/commands/{name}.json"),
        )?)?);
    }
    for record in ["proofs", "mutations"] {
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(format!("build/evidence/{record}.json"))?)?;
        if value["source_sha256"].as_str() != Some(&source) {
            return Err(format!("The {record} evidence belongs to a different source").into());
        }
    }
    let rust = crate::run("rustc", &["-vV"], "release-rust-version")?;
    let swift = crate::run(
        "xcrun",
        &["--sdk", "macosx", "swift", "--version"],
        "release-swift-version",
    )?;
    let sdk = crate::run(
        "xcrun",
        &["--sdk", "macosx", "--show-sdk-version"],
        "release-sdk-version",
    )?;
    let os = crate::run("sw_vers", &[], "release-os-version")?;
    let receipt = VerificationReceipt {
        source_sha256: source.clone(),
        binary_sha256: hash_file(RELEASE_BINARY)?,
        checks: REQUIRED_CHECKS
            .iter()
            .map(|name| (*name).to_owned())
            .collect(),
        advisory_freshness: fs::read_to_string("verification/advisory-revision.txt")?
            .trim()
            .to_owned(),
        resources_sha256: source_digest(Path::new(
            "build/xcode/Build/Products/Release/QRScanner.app/Contents/Resources",
        ))?,
        commands,
        tools: serde_json::json!({"rust": String::from_utf8(rust.stdout)?, "swift": String::from_utf8(swift.stdout)?,
            "sdk": String::from_utf8(sdk.stdout)?, "os": String::from_utf8(os.stdout)?, "image": crate::verification::image()?,
            "architectures": ["arm64", "x86_64"]}),
        proofs_sha256: hash_file("build/evidence/proofs.json")?,
        mutations_sha256: hash_file("build/evidence/mutations.json")?,
    };
    receipt.validate(&source, &hash_file(RELEASE_BINARY)?)?;
    fs::create_dir_all("build/evidence")?;
    fs::write(RECEIPT, serde_json::to_vec_pretty(&receipt)?)?;
    Ok(())
}

pub(crate) fn checked_receipt() -> TaskResult<VerificationReceipt> {
    let receipt: VerificationReceipt = serde_json::from_slice(
        &fs::read(RECEIPT)
            .map_err(|_| "No completed verification receipt; run mise x -- just verify-release")?,
    )?;
    receipt.validate(&source_digest(Path::new("."))?, &hash_file(RELEASE_BINARY)?)?;
    if receipt.resources_sha256
        != source_digest(Path::new(
            "build/xcode/Build/Products/Release/QRScanner.app/Contents/Resources",
        ))?
        || receipt.proofs_sha256 != hash_file("build/evidence/proofs.json")?
        || receipt.mutations_sha256 != hash_file("build/evidence/mutations.json")?
        || receipt.tools["image"].as_str() != Some(&crate::verification::image()?)
    {
        return Err(
            "The verified resources, proof evidence, mutation evidence, or tool image changed"
                .into(),
        );
    }
    Ok(receipt)
}

pub(crate) fn prove(toolchain: Option<&str>) -> TaskResult<()> {
    let compiler = crate::run("rustc", &["-vV"], "proof-compiler")?;
    let verifier = crate::run("cargo", &["kani", "--version"], "proof-verifier")?;
    if !String::from_utf8_lossy(&verifier.stdout).contains("0.68.0") {
        return Err("The proof verifier differs from the pinned version".into());
    }
    let result = cargo(
        &[
            "kani",
            "-p",
            "qrscanner-core",
            "-p",
            "qrscanner-ffi",
            "-p",
            "xtask",
        ],
        "kani-proofs",
        toolchain,
    )?;
    let output = String::from_utf8(result.stdout)?;
    let count = HARNESS_NAMES.len();
    let summary =
        format!("Complete - {count} successfully verified harnesses, 0 failures, {count} total.");
    if !output.contains(&summary) || HARNESS_NAMES.iter().any(|name| !output.contains(name)) {
        return Err("Kani did not complete every required production harness".into());
    }
    let arguments = [
        "kani",
        "-p",
        "qrscanner-core",
        "--features",
        "negative-probe",
        "--harness",
        "reject_automatic_open_probe",
    ];
    let output = if let Some(name) = toolchain {
        let mut args = vec!["run", name, "cargo"];
        args.extend_from_slice(&arguments);
        invoke("rustup", &args, "kani-negative-control")?
    } else {
        invoke("cargo", &arguments, "kani-negative-control")?
    };
    let text = String::from_utf8(output.stdout)?;
    if output.status.success()
        || !text.contains("NEGATIVE_CONTROL_AUTOMATIC_OPEN")
        || !text.contains("VERIFICATION:- FAILED")
    {
        return Err(
            "The expected automatic-opening counterexample was not rejected by Kani".into(),
        );
    }
    println!(
        "[qrscanner] {count} production proofs passed; automatic-opening counterexample rejected"
    );
    fs::create_dir_all("build/evidence")?;
    fs::write(
        "build/evidence/proofs.json",
        serde_json::to_vec_pretty(&serde_json::json!({
            "source_sha256": source_digest(Path::new("."))?,
            "verifier": String::from_utf8(verifier.stdout)?, "compiler": String::from_utf8(compiler.stdout)?,
            "target": "x86_64-unknown-linux-gnu",
            "harnesses": HARNESS_NAMES, "negative_control_rejected": true,
            "native_integration_verified": false
        }))?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt() -> VerificationReceipt {
        VerificationReceipt {
            source_sha256: "source".to_owned(),
            binary_sha256: "binary".to_owned(),
            checks: vec![],
            advisory_freshness: "unknown".to_owned(),
            resources_sha256: String::new(),
            commands: vec![],
            tools: serde_json::Value::Null,
            proofs_sha256: String::new(),
            mutations_sha256: String::new(),
        }
    }

    #[test]
    fn missing_and_stale_evidence_cannot_sign() {
        let receipt = receipt();
        assert!(receipt.validate("source", "binary").is_err());
        assert!(receipt.validate("other-source", "binary").is_err());
        assert!(receipt.validate("source", "other-binary").is_err());
    }

    #[test]
    fn a_fast_check_receipt_cannot_authorize_signing() {
        let mut receipt = receipt();
        receipt.checks = [
            "rust-format",
            "rust-clippy",
            "rust-tests",
            "swift-lint",
            "kani-proofs",
            "kani-negative-control",
            "native-tests-arm64",
            "native-tests-x86_64",
            "dependency-policy",
            "secrets",
            "repository-documents",
        ]
        .map(str::to_owned)
        .to_vec();
        assert!(receipt.validate("source", "binary").is_err());
    }

    fn complete_receipt() -> VerificationReceipt {
        let mut value = receipt();
        value.checks = REQUIRED_CHECKS
            .iter()
            .map(|name| (*name).to_owned())
            .collect();
        value.commands = REQUIRED_CHECKS
            .iter()
            .map(|name| CommandRecord {
                name: (*name).to_owned(),
                command: vec!["checked-command".to_owned()],
                source_sha256: "source".to_owned(),
                log_sha256: "a".repeat(64),
            })
            .collect();
        value.tools = serde_json::json!({"rust": "rustc 1.99.1", "swift": "Swift 6", "sdk": "27.2", "image": "sha256:abc"});
        value.resources_sha256 = "b".repeat(64);
        value.proofs_sha256 = "c".repeat(64);
        value.mutations_sha256 = "d".repeat(64);
        value
    }

    #[test]
    fn complete_evidence_passes_and_each_missing_check_fails() {
        assert!(complete_receipt().validate("source", "binary").is_ok());
        for index in 0..REQUIRED_CHECKS.len() {
            let mut value = complete_receipt();
            value.checks.remove(index);
            assert!(value.validate("source", "binary").is_err());
            let mut value = complete_receipt();
            value.commands.remove(index);
            assert!(value.validate("source", "binary").is_err());
        }
    }

    #[test]
    fn duplicate_checks_and_stale_command_sources_are_refused() {
        let mut value = complete_receipt();
        value.checks[0] = value.checks[1].clone();
        assert!(value.validate("source", "binary").is_err());
        let mut value = complete_receipt();
        value.commands[0].source_sha256 = "different".to_owned();
        assert!(value.validate("source", "binary").is_err());
        assert!(complete_receipt().validate("changed", "binary").is_err());
        assert!(complete_receipt().validate("source", "changed").is_err());
    }

    #[test]
    fn source_changes_invalidate_the_digest() -> TaskResult<()> {
        let root = std::env::temp_dir().join(format!("qrscanner-source-{}", std::process::id()));
        fs::create_dir_all(&root)?;
        fs::write(root.join("source.rs"), "first")?;
        let first = source_digest(&root)?;
        fs::write(root.join("source.rs"), "second")?;
        assert_ne!(first, source_digest(&root)?);
        fs::create_dir_all(root.join("build"))?;
        fs::write(root.join("build/output"), "generated")?;
        let source = source_digest(&root)?;
        fs::write(root.join("build/output"), "new generated")?;
        assert_eq!(source, source_digest(&root)?);
        fs::remove_dir_all(root)?;
        Ok(())
    }
}
