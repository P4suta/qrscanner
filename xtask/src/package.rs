use crate::assurance::{self, RELEASE_BINARY};
use crate::release_rules::{NextStep, NotaryState, next_step};
use crate::{TaskResult, build_native, cargo, run};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

const CERTIFICATE_SELECTOR_HEX_LENGTH: usize = 40;
const SUBMISSION_ID_LENGTH: usize = 36;
const SUBMISSION_ID_HYPHEN_POSITIONS: [usize; 4] = [8, 13, 18, 23];
const APPLE_TEAM_ID_LENGTH: usize = 10;
const APPLICATIONS_DIRECTORY: &str = "/Applications";
const SPDX_NAMESPACE_PREFIX: &str = "https://spdx.org/spdxdocs/qrscanner-";
const RUST_DOWNLOAD_ORIGIN: &str = "https://static.rust-lang.org/";

pub(crate) struct Inputs {
    pub(crate) development: bool,
    pub(crate) identity: Option<String>,
    pub(crate) team: Option<String>,
    pub(crate) profile: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Submission {
    id: String,
    source_sha256: String,
    archive_sha256: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SignedCandidate {
    source_sha256: String,
    unsigned_binary_sha256: String,
    signed_binary_sha256: String,
    identity: String,
    team: String,
}

fn validate_identity(value: &str) -> bool {
    value.len() == CERTIFICATE_SELECTOR_HEX_LENGTH
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_submission_id(value: &str) -> bool {
    value.len() == SUBMISSION_ID_LENGTH
        && value.bytes().enumerate().all(|(index, byte)| {
            if SUBMISSION_ID_HYPHEN_POSITIONS.contains(&index) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

pub(crate) fn package(inputs: Inputs, toolchain: Option<&str>) -> TaskResult<()> {
    if inputs.development {
        build_native("Release", toolchain)?;
        crate::copy_app("Release")?;
    } else {
        let receipt = assurance::checked_receipt()?;
        crate::acceptance::check(&receipt.source_sha256, &receipt.binary_sha256)?;
    }
    let identity = if inputs.development {
        None
    } else {
        let value = inputs.identity.as_deref().ok_or("Set QRSCANNER_SIGNING_IDENTITY to an existing Developer ID certificate’s 40-character selector")?;
        if !validate_identity(value) {
            return Err(
                "QRSCANNER_SIGNING_IDENTITY must be a certificate selector, not a name".into(),
            );
        }
        Some(value)
    };
    if identity.is_some() {
        let team = inputs
            .team
            .as_deref()
            .ok_or("Set QRSCANNER_TEAM_ID to the intended Apple Developer team")?;
        if team.len() != APPLE_TEAM_ID_LENGTH
            || !team.bytes().all(|byte| byte.is_ascii_alphanumeric())
        {
            return Err("Invalid Apple team ID".into());
        }
        let profile = inputs
            .profile
            .as_deref()
            .ok_or("Set QRSCANNER_NOTARY_PROFILE to an existing notarytool Keychain profile")?;
        if profile.is_empty() || profile.chars().any(char::is_control) {
            return Err("Invalid notarytool profile name".into());
        }
    }
    let source = assurance::source_digest(Path::new("."))?;
    let binary = assurance::hash_file(RELEASE_BINARY)?;
    let candidate = PathBuf::from("build/candidates").join(format!(
        "{}-{source}",
        if inputs.development {
            "development"
        } else {
            "signed"
        }
    ));
    fs::create_dir_all(&candidate)?;
    let app = candidate.join("QRScanner.app");
    let signed_receipt = candidate.join("signed-candidate.json");
    if !app.exists() || (identity.is_some() && !signed_receipt.exists()) {
        if candidate.join("app-submission.json").exists()
            || candidate.join("dmg-submission.json").exists()
        {
            return Err(
                "A pending candidate lost its signed source binding; preserve it for recovery"
                    .into(),
            );
        }
        if app.exists() {
            fs::remove_dir_all(&app)?;
        }
        run(
            "ditto",
            &[
                "build/xcode/Build/Products/Release/QRScanner.app",
                text(&app)?,
            ],
            "stage-app",
        )?;
        if let Some(identity) = identity {
            for license in ["LICENSE-MIT", "LICENSE-APACHE"] {
                fs::copy(license, app.join("Contents/Resources").join(license))
                    .map_err(|_| "Generate repository licenses with dotfiles repository generate before signing")?;
            }
            let team = inputs
                .team
                .as_deref()
                .ok_or("Set QRSCANNER_TEAM_ID to the intended Apple Developer team")?;
            run(
                "codesign",
                &[
                    "--force",
                    "--options",
                    "runtime",
                    "--timestamp",
                    "--entitlements",
                    "native/Resources/QRScanner.entitlements",
                    "--sign",
                    identity,
                    text(&app)?,
                ],
                "developer-id-sign",
            )?;
            verify_signer(&app, team, identity)?;
            fs::write(
                &signed_receipt,
                serde_json::to_vec_pretty(&SignedCandidate {
                    source_sha256: source.clone(),
                    unsigned_binary_sha256: binary.clone(),
                    signed_binary_sha256: assurance::hash_file(
                        app.join("Contents/MacOS/QRScanner"),
                    )?,
                    identity: identity.to_owned(),
                    team: team.to_owned(),
                })?,
            )?;
        } else {
            run(
                "codesign",
                &[
                    "--force",
                    "--sign",
                    "-",
                    "--entitlements",
                    "native/Resources/QRScanner.entitlements",
                    text(&app)?,
                ],
                "development-sign",
            )?;
        }
    }
    run(
        "codesign",
        &["--verify", "--strict", text(&app)?],
        "verify-app-signature",
    )?;
    crate::bundle::inspect(&app, true)?;
    let architectures = run(
        "xcrun",
        &[
            "lipo",
            "-archs",
            text(&app.join("Contents/MacOS/QRScanner"))?,
        ],
        "verify-architectures",
    )?;
    let architectures = String::from_utf8(architectures.stdout)?;
    let mut architectures: Vec<_> = architectures.split_whitespace().collect();
    architectures.sort_unstable();
    if architectures != ["arm64", "x86_64"] {
        return Err("The app must contain exactly arm64 and x86_64 slices".into());
    }
    if let Some(identity) = identity {
        let team = inputs
            .team
            .as_deref()
            .ok_or("QRSCANNER_TEAM_ID is required")?;
        let signed: SignedCandidate = serde_json::from_slice(&fs::read(&signed_receipt)?)?;
        if signed.source_sha256 != source
            || signed.unsigned_binary_sha256 != binary
            || signed.identity != identity
            || signed.team != team
            || signed.signed_binary_sha256
                != assurance::hash_file(app.join("Contents/MacOS/QRScanner"))?
        {
            return Err("The preserved signed candidate no longer matches the verified source and publisher".into());
        }
        verify_signer(&app, team, identity)?;
        let audit = cargo(&["audit", "--json"], "fresh-advisory-audit", toolchain)?;
        let audit: Value = serde_json::from_slice(&audit.stdout)?;
        if audit["database"]["last-commit"].as_str().is_none() {
            return Err("A fresh advisory database could not be established".into());
        }
        if let Some(disk) = completed_candidate(&candidate, &source, &binary)? {
            verify_distribution(&app, &disk)?;
            println!("Verified existing distribution: {}", disk.display());
            return Ok(());
        }
        let profile = inputs
            .profile
            .as_deref()
            .ok_or("Set QRSCANNER_NOTARY_PROFILE to an existing notarytool Keychain profile")?;
        let archive = candidate.join("signed-app.zip");
        if !archive.exists() {
            zip(&app, &archive)?;
        }
        notarize(
            &archive,
            &candidate.join("app-submission.json"),
            &source,
            profile,
            &app,
        )?;
        run("xcrun", &["stapler", "staple", text(&app)?], "staple-app")?;
        run(
            "xcrun",
            &["stapler", "validate", text(&app)?],
            "verify-app-ticket",
        )?;
        let disk = make_dmg(&app, &candidate, false)?;
        if !candidate.join("dmg-submission.json").exists() {
            run(
                "codesign",
                &["--force", "--timestamp", "--sign", identity, text(&disk)?],
                "sign-dmg",
            )?;
        }
        let submitted_disk = candidate.join("notarization-input.dmg");
        if !submitted_disk.exists() {
            fs::copy(&disk, &submitted_disk)?;
        }
        notarize(
            &submitted_disk,
            &candidate.join("dmg-submission.json"),
            &source,
            profile,
            &app,
        )?;
        run("xcrun", &["stapler", "staple", text(&disk)?], "staple-dmg")?;
        run(
            "xcrun",
            &["stapler", "validate", text(&disk)?],
            "verify-dmg-ticket",
        )?;
        verify_distribution(&app, &disk)?;
        evidence(&candidate, &disk, &source, &binary, true)?;
        println!("Verified distribution: {}", disk.display());
    } else {
        let archive = candidate.join(format!(
            "QRScanner-{}-development.zip",
            env!("CARGO_PKG_VERSION")
        ));
        zip(&app, &archive)?;
        fs::copy(&archive, "build/QRScanner-development.zip")?;
        evidence(&candidate, &archive, &source, &binary, false)?;
        println!("Development archive: {}", archive.display());
        let disk = make_dmg(&app, &candidate, true)?;
        evidence(&candidate, &disk, &source, &binary, false)?;
        println!("Development disk image: {}", disk.display());
    }
    Ok(())
}

fn text(path: &Path) -> TaskResult<&str> {
    path.to_str()
        .ok_or_else(|| "Artifact path is not UTF-8".into())
}

fn completed_candidate(
    candidate: &Path,
    source: &str,
    binary: &str,
) -> TaskResult<Option<PathBuf>> {
    let manifest = candidate.join("candidate.json");
    if !manifest.exists() {
        return Ok(None);
    }
    let metadata: Value = serde_json::from_slice(&fs::read(manifest)?)?;
    let name = format!("QRScanner-{}.dmg", env!("CARGO_PKG_VERSION"));
    let disk = candidate.join(&name);
    if metadata["distribution_ready"].as_bool() != Some(true)
        || metadata["source_sha256"].as_str() != Some(source)
        || metadata["unsigned_binary_sha256"].as_str() != Some(binary)
        || metadata["artifact"].as_str() != Some(&name)
        || metadata["artifact_sha256"].as_str() != Some(&assurance::hash_file(&disk)?)
    {
        return Err(
            "The completed distribution no longer matches its source and artifact manifest".into(),
        );
    }
    Ok(Some(disk))
}

fn verify_distribution(app: &Path, disk: &Path) -> TaskResult<()> {
    run(
        "xcrun",
        &["stapler", "validate", text(app)?],
        "verify-final-app-ticket",
    )?;
    run(
        "xcrun",
        &["stapler", "validate", text(disk)?],
        "verify-final-dmg-ticket",
    )?;
    run(
        "spctl",
        &["--assess", "--type", "execute", "--verbose=4", text(app)?],
        "gatekeeper-app",
    )?;
    run(
        "spctl",
        &[
            "--assess",
            "--type",
            "open",
            "--context",
            "context:primary-signature",
            "--verbose=4",
            text(disk)?,
        ],
        "gatekeeper-dmg",
    )?;
    Ok(())
}

fn zip(app: &Path, archive: &Path) -> TaskResult<()> {
    run(
        "ditto",
        &[
            "-c",
            "-k",
            "--sequesterRsrc",
            "--keepParent",
            text(app)?,
            text(archive)?,
        ],
        "archive-app",
    )?;
    Ok(())
}

fn verify_signer(app: &Path, team: &str, identity: &str) -> TaskResult<()> {
    if team.len() != APPLE_TEAM_ID_LENGTH || !team.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err("Invalid Apple team ID".into());
    }
    let result = run(
        "codesign",
        &["--display", "--verbose=4", text(app)?],
        "inspect-signer",
    )?;
    let output = String::from_utf8(result.stderr)?;
    if !output
        .lines()
        .any(|line| line == format!("TeamIdentifier={team}"))
        || !output.contains("Authority=Developer ID Application:")
        || !output.contains("Timestamp=")
        || !output.contains("runtime")
    {
        return Err("The app’s publisher, timestamp, or hardened runtime does not match".into());
    }
    let prefix = app
        .parent()
        .ok_or("Missing candidate directory")?
        .join("public-certificate-");
    run(
        "codesign",
        &[
            "--display",
            "--extract-certificates",
            text(&prefix)?,
            text(app)?,
        ],
        "extract-public-certificate",
    )?;
    let certificate = format!("{}0", text(&prefix)?);
    let fingerprint = run(
        "shasum",
        &["-a", "1", &certificate],
        "certificate-fingerprint",
    )?;
    let fingerprint = String::from_utf8(fingerprint.stdout)?;
    if !fingerprint
        .split_whitespace()
        .next()
        .is_some_and(|value| value.eq_ignore_ascii_case(identity))
    {
        return Err("The app uses a different signing certificate".into());
    }
    Ok(())
}

fn make_dmg(app: &Path, candidate: &Path, development: bool) -> TaskResult<PathBuf> {
    let suffix = if development { "-development" } else { "" };
    let disk = candidate.join(format!(
        "QRScanner-{}{suffix}.dmg",
        env!("CARGO_PKG_VERSION")
    ));
    if disk.exists() {
        return Ok(disk);
    }
    let staging = candidate.join("disk-content");
    fs::create_dir_all(&staging)?;
    run(
        "ditto",
        &[text(app)?, text(&staging.join("QRScanner.app"))?],
        "stage-disk",
    )?;
    let applications = staging.join("Applications");
    if !applications.exists() {
        std::os::unix::fs::symlink(APPLICATIONS_DIRECTORY, &applications)?;
    }
    fs::write(
        staging.join("Install.txt"),
        "Drag QRScanner to Applications, then open it.\nUse Scan Screen in the menu bar or press Control-Option-Q.\nScreen Recording permission is requested when you start a scan.\n",
    )?;
    run(
        "hdiutil",
        &[
            "create",
            "-volname",
            "QRScanner",
            "-srcfolder",
            text(&staging)?,
            "-format",
            "UDZO",
            text(&disk)?,
        ],
        "create-dmg",
    )?;
    run("hdiutil", &["verify", text(&disk)?], "verify-dmg")?;
    Ok(disk)
}

fn notarize(
    archive: &Path,
    receipt_path: &Path,
    source: &str,
    profile: &str,
    app: &Path,
) -> TaskResult<()> {
    let digest = assurance::hash_file(archive)?;
    let receipt = if receipt_path.exists() {
        serde_json::from_slice::<Submission>(&fs::read(receipt_path)?)?
    } else {
        let result = run(
            "xcrun",
            &[
                "notarytool",
                "submit",
                text(archive)?,
                "--keychain-profile",
                profile,
                "--no-wait",
                "--output-format",
                "json",
            ],
            "submit-notarization",
        )?;
        let response: Value = serde_json::from_slice(&result.stdout)?;
        let id = response["id"]
            .as_str()
            .ok_or("Apple did not return a submission ID")?;
        if !valid_submission_id(id) {
            return Err("Apple returned an invalid submission ID".into());
        }
        let receipt = Submission {
            id: id.to_owned(),
            source_sha256: source.to_owned(),
            archive_sha256: digest.clone(),
        };
        fs::write(receipt_path, serde_json::to_vec_pretty(&receipt)?)?;
        receipt
    };
    if !valid_submission_id(&receipt.id) {
        return Err("Saved submission ID is invalid".into());
    }
    let info = run(
        "xcrun",
        &[
            "notarytool",
            "info",
            &receipt.id,
            "--keychain-profile",
            profile,
            "--output-format",
            "json",
        ],
        "notarization-status",
    )?;
    let response: Value = serde_json::from_slice(&info.stdout)?;
    let state = match response["status"].as_str() {
        Some("In Progress") => NotaryState::Pending,
        Some("Accepted") => NotaryState::Accepted,
        Some("Invalid" | "Rejected") => NotaryState::Rejected,
        _ => return Err("Apple returned an unknown notarization state".into()),
    };
    let matches = receipt.source_sha256 == source
        && receipt.archive_sha256 == digest
        && response["id"].as_str() == Some(&receipt.id);
    match next_step(state, matches) {
        NextStep::Wait => {
            return Err(format!(
                "Notarization is pending ({}). Rerun the same package command to resume.",
                receipt.id
            )
            .into());
        }
        NextStep::Refuse => {
            return Err(
                "Notarization was rejected or the preserved source/archive no longer matches"
                    .into(),
            );
        }
        NextStep::Staple => {}
    }
    let log = run(
        "xcrun",
        &[
            "notarytool",
            "log",
            &receipt.id,
            "--keychain-profile",
            profile,
        ],
        "notarization-log",
    )?;
    let log: Value = serde_json::from_slice(&log.stdout)?;
    if log["issues"]
        .as_array()
        .is_some_and(|issues| !issues.is_empty())
    {
        return Err("Notarization log contains unresolved issues".into());
    }
    let tickets = log["ticketContents"]
        .as_array()
        .ok_or("Notarization log has no ticket inventory")?;
    for architecture in ["arm64", "x86_64"] {
        let result = run(
            "codesign",
            &[
                "--display",
                "--arch",
                architecture,
                "--verbose=4",
                text(app)?,
            ],
            &format!("code-hash-{architecture}"),
        )?;
        let output = String::from_utf8(result.stderr)?;
        let hash = output
            .lines()
            .find_map(|line| line.strip_prefix("CDHash="))
            .ok_or("Missing code-directory hash")?;
        if !tickets.iter().any(|ticket| {
            ticket["cdhash"].as_str() == Some(hash) && ticket["arch"].as_str() == Some(architecture)
        }) {
            return Err(
                format!("Notarization ticket does not cover the {architecture} binary").into(),
            );
        }
    }
    fs::write(
        receipt_path.with_extension("accepted.json"),
        serde_json::to_vec_pretty(&log)?,
    )?;
    Ok(())
}

fn evidence(
    candidate: &Path,
    artifact: &Path,
    source: &str,
    binary: &str,
    notarized: bool,
) -> TaskResult<()> {
    let digest = assurance::hash_file(artifact)?;
    let filename = artifact
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("Missing artifact name")?;
    fs::write(
        candidate.join(format!("{filename}.sha256")),
        format!("{digest}  {filename}\n"),
    )?;
    fs::write(
        candidate.join("candidate.json"),
        serde_json::to_vec_pretty(&json!({
            "product": "QRScanner", "version": env!("CARGO_PKG_VERSION"), "source_sha256": source,
            "unsigned_binary_sha256": binary, "artifact": filename, "artifact_sha256": digest,
            "architectures": ["arm64", "x86_64"], "notarized": notarized,
            "distribution_ready": notarized
        }))?,
    )?;
    let created = run("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"], "evidence-date")?;
    let created = String::from_utf8(created.stdout)?;
    fs::write(
        candidate.join("sbom.spdx.json"),
        serde_json::to_vec_pretty(&json!({
            "spdxVersion": "SPDX-2.3", "dataLicense": "CC0-1.0", "SPDXID": "SPDXRef-DOCUMENT",
            "name": "QRScanner", "documentNamespace": format!("{SPDX_NAMESPACE_PREFIX}{source}"),
            "creationInfo": {"creators": ["Tool: QRScanner xtask"], "created": created.trim()},
            "packages": [
                {"name": "QRScanner", "SPDXID": "SPDXRef-QRScanner", "versionInfo": env!("CARGO_PKG_VERSION"),
                 "downloadLocation": "NOASSERTION", "filesAnalyzed": false, "licenseDeclared": "MIT OR Apache-2.0",
                 "checksums": [{"algorithm": "SHA256", "checksumValue": digest}]},
                {"name": "Rust standard library", "SPDXID": "SPDXRef-RustRuntime", "versionInfo": "1.99.0",
                 "downloadLocation": RUST_DOWNLOAD_ORIGIN, "filesAnalyzed": false, "licenseConcluded": "NOASSERTION"}
            ],
            "relationships": [
                {"spdxElementId": "SPDXRef-DOCUMENT", "relationshipType": "DESCRIBES", "relatedSpdxElement": "SPDXRef-QRScanner"},
                {"spdxElementId": "SPDXRef-QRScanner", "relationshipType": "DEPENDS_ON", "relatedSpdxElement": "SPDXRef-RustRuntime"}
            ]
        }))?,
    )?;
    fs::copy("Cargo.lock", candidate.join("Cargo.lock"))?;
    if notarized {
        fs::copy(assurance::RECEIPT, candidate.join("verification.json"))?;
        fs::copy("build/evidence/proofs.json", candidate.join("proofs.json"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certificate_selector_and_submission_identifiers_are_exact() {
        assert!(validate_identity(&"A".repeat(40)));
        assert!(!validate_identity("Developer ID Application"));
        assert!(!validate_identity(&"G".repeat(40)));
        assert!(valid_submission_id("12345678-1234-1234-1234-123456789abc"));
        assert!(!valid_submission_id("12345678/1234-1234-1234-123456789abc"));
        assert!(!valid_submission_id("../submission"));
    }
}
