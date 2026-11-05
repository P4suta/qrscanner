use crate::{TaskResult, assurance, bundle, run};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub(crate) enum Artifact {
    Interactive,
    Verification,
}

impl Artifact {
    const fn destination(self) -> &'static str {
        match self {
            Self::Interactive => "build/QRScanner.app",
            Self::Verification => "build/check/QRScanner.app",
        }
    }
}

pub(crate) fn interactive_identity(ad_hoc: bool) -> TaskResult<String> {
    if ad_hoc {
        return Ok("-".to_owned());
    }
    let identity = std::env::var("QRSCANNER_DEVELOPMENT_IDENTITY")
        .map_err(|_| "Set QRSCANNER_DEVELOPMENT_IDENTITY to an existing Apple Development certificate selector, or use just app --ad-hoc")?;
    if !crate::package::validate_identity(&identity) {
        return Err("The development identity must be an exact certificate selector".into());
    }
    let catalog = run(
        "security",
        &["find-identity", "-v", "-p", "codesigning"],
        "development-identities",
    )?;
    if !String::from_utf8(catalog.stdout)?.lines().any(|line| {
        line.split_whitespace()
            .nth(1)
            .is_some_and(|value| value.eq_ignore_ascii_case(&identity))
            && line.contains("\"Apple Development:")
    }) {
        return Err("The selected Apple Development identity is unavailable".into());
    }
    Ok(identity)
}

fn replace_with(
    root: &Path,
    artifact: Artifact,
    source: &Path,
    copy: impl FnOnce(&Path, &Path) -> TaskResult<()>,
) -> TaskResult<PathBuf> {
    let destination = root.join(artifact.destination());
    if destination.try_exists()? {
        fs::remove_dir_all(&destination)?;
    }
    fs::create_dir_all(destination.parent().ok_or("Missing artifact parent")?)?;
    copy(source, &destination)?;
    Ok(destination)
}

pub(crate) fn stage(configuration: &str, artifact: Artifact, identity: &str) -> TaskResult<()> {
    let source = PathBuf::from(format!(
        "build/xcode/Build/Products/{configuration}/QRScanner.app"
    ));
    let app = replace_with(Path::new("."), artifact, &source, |source, destination| {
        run("ditto", &[text(source)?, text(destination)?], "copy-app")?;
        Ok(())
    })?;
    run(
        "codesign",
        &[
            "--force",
            "--sign",
            identity,
            "--timestamp=none",
            "--entitlements",
            "native/Resources/QRScanner.entitlements",
            text(&app)?,
        ],
        "local-signature",
    )?;
    run(
        "codesign",
        &["--verify", "--deep", "--strict", text(&app)?],
        "verify-local-signature",
    )?;
    if identity != "-" {
        let signature = run(
            "codesign",
            &["--display", "--verbose=4", text(&app)?],
            "development-signature",
        )?;
        if !String::from_utf8(signature.stderr)?
            .lines()
            .any(|line| line.starts_with("Authority=Apple Development:"))
        {
            return Err("The app was not signed with Apple Development".into());
        }
    }
    bundle::inspect(&app, true)
}

pub(crate) fn interactive_fingerprint() -> TaskResult<Option<String>> {
    let path = Path::new(Artifact::Interactive.destination());
    if path.try_exists()? {
        Ok(Some(assurance::source_digest(path)?))
    } else {
        Ok(None)
    }
}

fn text(path: &Path) -> TaskResult<&str> {
    path.to_str()
        .ok_or_else(|| "An app path is not UTF-8".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacing_verification_artifacts_preserves_the_interactive_bundle() -> TaskResult<()> {
        let root = std::env::temp_dir().join(format!("qrscanner-artifacts-{}", std::process::id()));
        fs::create_dir(&root)?;
        let interactive = root.join(Artifact::Interactive.destination());
        fs::create_dir_all(&interactive)?;
        fs::write(
            interactive.join("identity"),
            "authorized development signature",
        )?;
        let source = root.join("source.app");
        fs::create_dir(&source)?;
        fs::write(source.join("identity"), "verification signature")?;
        for _ in 0..2 {
            let output = replace_with(
                &root,
                Artifact::Verification,
                &source,
                |source, destination| {
                    fs::create_dir(destination)?;
                    fs::copy(source.join("identity"), destination.join("identity"))?;
                    Ok(())
                },
            )?;
            assert_eq!(
                fs::read_to_string(output.join("identity"))?,
                "verification signature"
            );
            assert_eq!(
                fs::read_to_string(interactive.join("identity"))?,
                "authorized development signature"
            );
        }
        fs::remove_dir_all(root)?;
        Ok(())
    }
}

#[cfg(kani)]
#[kani::proof]
fn verification_artifacts_have_distinct_destinations() {
    assert_ne!(
        Artifact::Verification.destination(),
        Artifact::Interactive.destination()
    );
}
