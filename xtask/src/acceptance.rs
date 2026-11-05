use crate::TaskResult;
use qrscanner_verification::{AcceptanceFacts, MacOS27, acceptance_complete};
use serde::{Deserialize, Serialize};
use std::fs;

const RECORD: &str = "build/evidence/macos27-acceptance.json";
const CHECKS: &[&str] = &[
    "screen-capture",
    "application-exclusion",
    "permission-denial",
    "permission-revocation",
    "permission-recovery",
    "browser",
    "clipboard",
    "relaunch",
    "multiple-displays",
    "voiceover",
    "keyboard",
    "large-text",
    "contrast",
    "transparency",
    "appearances",
];

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct NativeAcceptance {
    source_sha256: String,
    binary_sha256: String,
    sessions: Vec<Session>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Session {
    os_version: String,
    os_build: String,
    architecture: String,
    hardware: String,
    checks: std::collections::BTreeMap<String, bool>,
}

impl NativeAcceptance {
    fn validate(&self, source: &str, binary: &str) -> TaskResult<()> {
        if self.source_sha256 != source || self.binary_sha256 != binary {
            return Err("macOS 27 acceptance belongs to a different source or binary".into());
        }
        let session_accepted = |architecture| {
            self.sessions.iter().any(|session| {
                session.architecture == architecture
                    && MacOS27::parse(session.os_version.as_bytes()).is_some()
                    && !session.os_build.trim().is_empty()
                    && !session.hardware.trim().is_empty()
                    && CHECKS
                        .iter()
                        .all(|name| session.checks.get(*name) == Some(&true))
            })
        };
        if !acceptance_complete(AcceptanceFacts {
            identities_match: self.source_sha256 == source && self.binary_sha256 == binary,
            arm64_session: session_accepted("arm64"),
        }) {
            return Err(
                "macOS 27 login-session acceptance requires arm64 and every interaction".into(),
            );
        }
        Ok(())
    }
}

pub(crate) fn check(source: &str, binary: &str) -> TaskResult<()> {
    let bytes = fs::read(RECORD).map_err(|_| "Signed distribution requires build/evidence/macos27-acceptance.json from real login-session acceptance")?;
    serde_json::from_slice::<NativeAcceptance>(&bytes)?.validate(source, binary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accepted() -> NativeAcceptance {
        NativeAcceptance {
            source_sha256: "source".to_owned(),
            binary_sha256: "binary".to_owned(),
            sessions: ["arm64"]
                .map(|architecture| Session {
                    os_version: "27.0".to_owned(),
                    os_build: "test-fixture".to_owned(),
                    hardware: "test-fixture".to_owned(),
                    architecture: architecture.to_owned(),
                    checks: CHECKS
                        .iter()
                        .map(|name| ((*name).to_owned(), true))
                        .collect(),
                })
                .into(),
        }
    }

    #[test]
    fn acceptance_requires_arm64_on_the_supported_os() {
        assert!(accepted().validate("source", "binary").is_ok());
        let mut value = accepted();
        value.sessions.pop();
        assert!(value.validate("source", "binary").is_err());
        let mut value = accepted();
        value.sessions[0].os_version = "26.0.1".to_owned();
        assert!(value.validate("source", "binary").is_err());
        assert!(accepted().validate("different", "binary").is_err());
        assert!(accepted().validate("source", "different").is_err());
    }

    #[test]
    fn distribution_accepts_only_a_macos27_arm64_login_session() {
        let mut value = accepted();
        value
            .sessions
            .retain(|session| session.architecture == "arm64");
        value.sessions[0].os_version = "27.0.1".to_owned();
        assert!(value.validate("source", "binary").is_ok());
        for version in ["26.0", "28.0"] {
            value.sessions[0].os_version = version.to_owned();
            assert!(value.validate("source", "binary").is_err());
        }
        value.sessions[0].os_version = "27.0.1".to_owned();
        value.sessions[0].architecture = "x86_64".to_owned();
        assert!(value.validate("source", "binary").is_err());
    }

    #[test]
    fn a_missing_or_failed_interaction_cannot_authorize_distribution() {
        for name in CHECKS {
            let mut value = accepted();
            value.sessions[0].checks.remove(*name);
            assert!(value.validate("source", "binary").is_err());
            value.sessions[0].checks.insert((*name).to_owned(), false);
            assert!(value.validate("source", "binary").is_err());
        }
    }

    #[test]
    fn malformed_os_versions_and_blank_metadata_are_refused() {
        for version in [
            "27.",
            "27.invalid",
            "27.0.",
            "27.0.1.2",
            "27.0\n",
            "27..1",
            "27.０",
        ] {
            let mut value = accepted();
            value.sessions[0].os_version = version.to_owned();
            assert!(
                value.validate("source", "binary").is_err(),
                "accepted {version:?}"
            );
        }
        for blank in ["", " ", "\t\n"] {
            let mut value = accepted();
            value.sessions[0].os_build = blank.to_owned();
            assert!(value.validate("source", "binary").is_err());
            let mut value = accepted();
            value.sessions[0].hardware = blank.to_owned();
            assert!(value.validate("source", "binary").is_err());
        }
        for version in ["27.0", "27.1", "27.0.1", "27.12.34"] {
            let mut value = accepted();
            value.sessions[0].os_version = version.to_owned();
            assert!(value.validate("source", "binary").is_ok());
        }
    }
}
