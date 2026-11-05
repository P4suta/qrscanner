use crate::{TaskResult, assurance, run};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub(crate) fn inspect(app: &Path, signed: bool) -> TaskResult<()> {
    let resources = app.join("Contents/Resources");
    let privacy = plist(&resources.join("PrivacyInfo.xcprivacy"), "bundle-privacy")?;
    if privacy["NSPrivacyTracking"] != false
        || privacy["NSPrivacyCollectedDataTypes"] != serde_json::json!([])
        || privacy["NSPrivacyAccessedAPITypes"] != serde_json::json!([])
    {
        return Err("The bundled privacy manifest must declare no tracking, collection, or required-reason API claims".into());
    }
    let migration = plist(
        &resources.join("container-migration.plist"),
        "bundle-migration",
    )?;
    if migration["Move"]
        != serde_json::json!([[
            "${Library}/Preferences/${BundleId}.plist",
            "${ApplicationSupport}/QRScanner/ImportedPreferences.plist"
        ]])
    {
        return Err(
            "The container migration resource differs from the preference migration contract"
                .into(),
        );
    }
    let info = plist(&app.join("Contents/Info.plist"), "bundle-info")?;
    if info["LSMinimumSystemVersion"] != "27.0" || info["LSUIElement"] != true {
        return Err("The bundle must target macOS 27 as a menu bar app".into());
    }
    if info["CFBundleShortVersionString"] != env!("CARGO_PKG_VERSION") {
        return Err("The app version differs from the Rust workspace version".into());
    }
    if !resources.join("en.lproj/Localizable.stringsdict").is_file() {
        return Err("The bundle lacks compiled English plural rules".into());
    }
    let binary = app.join("Contents/MacOS/QRScanner");
    let architectures = run(
        "xcrun",
        &["lipo", "-archs", path(&binary)?],
        "bundle-architectures",
    )?;
    let text = String::from_utf8(architectures.stdout)?;
    let mut architectures: Vec<_> = text.split_whitespace().collect();
    architectures.sort_unstable();
    if architectures != ["arm64"] {
        return Err("The app must contain exactly arm64".into());
    }
    if signed {
        let output = run(
            "codesign",
            &["--display", "--entitlements", ":-", path(app)?],
            "bundle-entitlements",
        )?;
        fs::write("build/evidence/signed-entitlements.plist", output.stdout)?;
        let entitlements = plist(
            Path::new("build/evidence/signed-entitlements.plist"),
            "bundle-entitlements-json",
        )?;
        let entries = entitlements
            .as_object()
            .ok_or("Invalid signed entitlements")?;
        if entitlements["com.apple.security.app-sandbox"] != true
            || entries.keys().any(|key| {
                key.starts_with("com.apple.security.") && key != "com.apple.security.app-sandbox"
            })
        {
            return Err(
                "The signed app must use App Sandbox without extra security capabilities".into(),
            );
        }
    }
    fs::create_dir_all("build/evidence")?;
    fs::write(
        "build/evidence/bundle.json",
        serde_json::to_vec_pretty(&serde_json::json!({
            "source_sha256": assurance::source_digest(Path::new("."))?,
            "binary_sha256": assurance::hash_file(binary)?, "resources_sha256": assurance::source_digest(&resources)?,
            "architectures": architectures, "privacy": privacy, "migration": migration,
            "signed_entitlements_inspected": signed
        }))?,
    )?;
    Ok(())
}

fn path(path: &Path) -> TaskResult<&str> {
    path.to_str()
        .ok_or_else(|| "A bundle path is not UTF-8".into())
}

fn plist(file: &Path, log: &str) -> TaskResult<Value> {
    Ok(serde_json::from_slice(
        &run("plutil", &["-convert", "json", "-o", "-", path(file)?], log)?.stdout,
    )?)
}
