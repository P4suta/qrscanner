use crate::TaskResult;
use serde::{Deserialize, Serialize};
use std::fs;

const RECORD: &str = "build/evidence/macos26-acceptance.json";
const CHECKS: &[&str] = &[
    "screen-capture",
    "application-exclusion",
    "permission-denial",
    "permission-revocation",
    "permission-recovery",
    "browser",
    "clipboard",
    "relaunch",
    "shortcut-conflict",
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
            return Err("macOS 26 acceptance belongs to a different source or binary".into());
        }
        for architecture in ["arm64", "x86_64"] {
            if !self.sessions.iter().any(|session| {
                session.architecture == architecture
                    && session.os_version.starts_with("26.")
                    && !session.os_build.is_empty()
                    && !session.hardware.is_empty()
                    && CHECKS
                        .iter()
                        .all(|name| session.checks.get(*name) == Some(&true))
            }) {
                return Err(format!(
                    "macOS 26 login-session acceptance is incomplete for {architecture}"
                )
                .into());
            }
        }
        Ok(())
    }
}

pub(crate) fn check(source: &str, binary: &str) -> TaskResult<()> {
    let bytes = fs::read(RECORD).map_err(|_| "Signed distribution requires build/evidence/macos26-acceptance.json from real login-session acceptance")?;
    serde_json::from_slice::<NativeAcceptance>(&bytes)?.validate(source, binary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accepted() -> NativeAcceptance {
        NativeAcceptance {
            source_sha256: "source".to_owned(),
            binary_sha256: "binary".to_owned(),
            sessions: ["arm64", "x86_64"]
                .map(|architecture| Session {
                    os_version: "26.0".to_owned(),
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
    fn acceptance_requires_both_architectures_on_the_supported_os() {
        assert!(accepted().validate("source", "binary").is_ok());
        let mut value = accepted();
        value.sessions.pop();
        assert!(value.validate("source", "binary").is_err());
        let mut value = accepted();
        value.sessions[0].os_version = "27.0.1".to_owned();
        assert!(value.validate("source", "binary").is_err());
        assert!(accepted().validate("different", "binary").is_err());
        assert!(accepted().validate("source", "different").is_err());
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
}
