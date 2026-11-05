use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

mod acceptance;
mod assurance;
mod bundle;
mod fuzzing;
mod icon;
mod local_app;
mod localization;
mod package;
mod proof_controls;
mod verification;

type TaskResult<T> = Result<T, Box<dyn Error>>;

const FAILURE_LOG_TAIL_LINES: usize = 35;
const GIT_COMMIT_HEX_LENGTH: usize = 40;
const ADVISORY_ORIGIN: &str = "https://github.com/RustSec/advisory-db";

fn main() {
    if let Err(error) = execute() {
        eprintln!("qrscanner: {error}");
        std::process::exit(1);
    }
}

fn execute() -> TaskResult<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("missing project root")?;
    std::env::set_current_dir(root)?;
    fs::create_dir_all("build/logs")?;
    fs::create_dir_all("build/tmp")?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open("build/xtask.lock")?;
    lock.try_lock()
        .map_err(|_| "Another QRScanner build, check, or packaging command is running")?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let toolchain = std::env::var("QRSCANNER_RUST_TOOLCHAIN").ok();
    if let Some(name) = toolchain.as_deref() {
        let pinned = run("rustc", &["-vV"], "pinned-compiler")?;
        let selected = run(
            "rustup",
            &["run", name, "rustc", "-vV"],
            "selected-compiler",
        )?;
        if pinned.stdout != selected.stdout {
            return Err("The selected Rust toolchain differs from the pinned compiler".into());
        }
    }
    match args.first().map(String::as_str) {
        Some("app") => {
            let ad_hoc = match args.get(1).map(String::as_str) {
                None => false,
                Some("--ad-hoc") if args.len() == 2 => true,
                _ => return Err("Usage: just app [--ad-hoc]".into()),
            };
            let identity = local_app::interactive_identity(ad_hoc)?;
            build_native("Debug", toolchain.as_deref())?;
            local_app::stage("Debug", local_app::Artifact::Interactive, &identity)?;
            println!("Built {}", root.join("build/QRScanner.app").display());
            Ok(())
        }
        Some("check") => check(toolchain.as_deref()),
        Some("test-native") => native_tests(false, true, Instrumentation::Plain),
        Some("localize") => { build_native("Debug", toolchain.as_deref())?; localization::sync(false) },
        Some("check-localization") => localization::sync(true),
        Some("prepare-verification") => verification::prepare(args.get(1).map(String::as_str)),
        Some("proofs") => verification::bounded(verification::Campaign::Proofs),
        Some("mutants") => verification::bounded(verification::Campaign::Mutations),
        Some("mutation-plan") => verification::bounded(verification::Campaign::MutationPlan),
        Some("fuzz") => verification::bounded(verification::Campaign::Fuzz),
        Some("proofs-inner") => assurance::prove(None),
        Some("mutants-inner") => verification::mutations(),
        Some("mutation-plan-inner") => verification::mutation_plan(),
        Some("fuzz-inner") => fuzzing::run_campaign(),
        Some("verify-release") => verify_release(toolchain.as_deref()),
        Some("package") => {
            let development = match args.get(1).map(String::as_str) {
                None => false,
                Some("--development") if args.len() == 2 => true,
                _ => return Err("Usage: just package [--development]".into()),
            };
            package::package(
                package::Inputs {
                    development,
                    identity: std::env::var("QRSCANNER_SIGNING_IDENTITY").ok(),
                    team: std::env::var("QRSCANNER_TEAM_ID").ok(),
                    profile: std::env::var("QRSCANNER_NOTARY_PROFILE").ok(),
                },
                toolchain.as_deref(),
            )
        }
        _ => Err("Usage: cargo run -p xtask -- <app|check|test-native|prepare-verification|proofs|mutants|verify-release|package>".into()),
    }
}

fn run(program: &str, args: &[&str], log_name: &str) -> TaskResult<Output> {
    run_with_environment(program, args, log_name, &[])
}

fn run_with_environment(
    program: &str,
    args: &[&str],
    log_name: &str,
    environment: &[(&str, &str)],
) -> TaskResult<Output> {
    let output = invoke_with_environment(program, args, log_name, environment)?;
    if !output.status.success() {
        let mut log = output.stdout.clone();
        log.extend_from_slice(&output.stderr);
        let text = String::from_utf8_lossy(&log);
        let tail: Vec<_> = text.lines().rev().take(FAILURE_LOG_TAIL_LINES).collect();
        for line in tail.into_iter().rev() {
            eprintln!("{line}");
        }
        return Err(format!(
            "{log_name} failed ({}); see build/logs/{log_name}.log",
            output.status
        )
        .into());
    }
    assurance::record_command(program, args, log_name)?;
    Ok(output)
}

fn invoke(program: &str, args: &[&str], log_name: &str) -> TaskResult<Output> {
    invoke_with_environment(program, args, log_name, &[])
}

fn invoke_with_environment(
    program: &str,
    args: &[&str],
    log_name: &str,
    environment: &[(&str, &str)],
) -> TaskResult<Output> {
    println!("[qrscanner] {log_name}");
    let output = Command::new(program)
        .args(args)
        .env("MACOSX_DEPLOYMENT_TARGET", "27.0")
        .env("CARGO_NET_OFFLINE", "true")
        .env("TMPDIR", std::env::current_dir()?.join("build/tmp"))
        .envs(environment.iter().copied())
        .output()?;
    let mut log = output.stdout.clone();
    log.extend_from_slice(&output.stderr);
    fs::write(format!("build/logs/{log_name}.log"), &log)?;
    Ok(output)
}

fn cargo(args: &[&str], log_name: &str, toolchain: Option<&str>) -> TaskResult<Output> {
    if let Some(name) = toolchain {
        let mut command = vec!["run", name, "cargo"];
        command.extend_from_slice(args);
        run("rustup", &command, log_name)
    } else {
        run("cargo", args, log_name)
    }
}

fn build_native(configuration: &str, toolchain: Option<&str>) -> TaskResult<()> {
    fs::create_dir_all("build/rust/lib")?;
    fs::create_dir_all("build/assets/AppIcon.iconset")?;
    run(
        "xcrun",
        &[
            "--sdk",
            "macosx",
            "swiftc",
            "-parse-as-library",
            "-swift-version",
            "6",
            "-warnings-as-errors",
            "-module-cache-path",
            "build/swift-module-cache",
            "native/Tools/RenderIcon.swift",
            "-o",
            "build/render-icon",
        ],
        "compile-icon-renderer",
    )?;
    run(
        "build/render-icon",
        &["build/assets/AppIcon.iconset"],
        "render-icon",
    )?;
    icon::package()?;
    cargo(
        &[
            "build",
            "--offline",
            "--locked",
            "--release",
            "-p",
            "qrscanner-ffi",
            "--target",
            "aarch64-apple-darwin",
        ],
        "rust-aarch64-apple-darwin",
        toolchain,
    )?;
    run(
        "ditto",
        &[
            "target/aarch64-apple-darwin/release/libqrscanner_ffi.a",
            "build/rust/lib/libqrscanner_ffi.a",
        ],
        "stage-core",
    )?;
    run(
        "xcodegen",
        &["--spec", "project.yml", "--project", "build"],
        "generate-project",
    )?;
    run(
        "xcodebuild",
        &[
            "build",
            "-project",
            "build/QRScanner.xcodeproj",
            "-scheme",
            "QRScanner",
            "-configuration",
            configuration,
            "-derivedDataPath",
            "build/xcode",
            "CODE_SIGNING_ALLOWED=NO",
            "ARCHS=arm64",
            "ONLY_ACTIVE_ARCH=NO",
        ],
        &format!("native-{configuration}"),
    )?;
    Ok(())
}

fn check(toolchain: Option<&str>) -> TaskResult<()> {
    let interactive = local_app::interactive_fingerprint()?;
    run("actionlint", &[], "workflow-lint")?;
    cargo(&["fmt", "--all", "--", "--check"], "rust-format", toolchain)?;
    cargo(
        &[
            "clippy",
            "--offline",
            "--locked",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        "rust-clippy",
        toolchain,
    )?;
    cargo(
        &["test", "--offline", "--locked", "--workspace"],
        "rust-tests",
        toolchain,
    )?;
    cargo(
        &["deny", "--offline", "check", "bans", "licenses", "sources"],
        "dependency-policy",
        toolchain,
    )?;
    audit_snapshot(toolchain)?;
    run(
        "gitleaks",
        &["dir", "--redact", "--no-banner", "."],
        "secrets",
    )?;
    if cfg!(target_os = "macos") {
        run(
            "swiftlint",
            &["lint", "--strict", "--cache-path", "build/swiftlint-cache"],
            "swift-lint",
        )?;
        build_native("Debug", toolchain)?;
        localization::sync(true)?;
        native_tests(false, false, Instrumentation::Plain)?;
        local_app::stage("Debug", local_app::Artifact::Verification, "-")?;
    }
    if interactive != local_app::interactive_fingerprint()? {
        return Err("Verification changed the interactive app or its signing identity".into());
    }
    println!("[qrscanner] fast checks passed");
    Ok(())
}

fn audit_snapshot(toolchain: Option<&str>) -> TaskResult<()> {
    let revision = fs::read_to_string("verification/advisory-revision.txt")?;
    let revision = revision.trim();
    if revision.len() != GIT_COMMIT_HEX_LENGTH
        || !revision.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("The advisory snapshot must be an exact commit".into());
    }
    let database = "build/advisory-db";
    if !Path::new(database).join(".git").exists() {
        run("git", &["init", database], "initialize-advisories")?;
        run(
            "git",
            &["-C", database, "remote", "add", "origin", ADVISORY_ORIGIN],
            "advisory-origin",
        )?;
    }
    let head = invoke(
        "git",
        &["-C", database, "rev-parse", "HEAD"],
        "advisory-revision",
    )?;
    if !head.status.success() || String::from_utf8(head.stdout)?.trim() != revision {
        run(
            "git",
            &["-C", database, "fetch", "--depth", "1", "origin", revision],
            "fetch-advisories",
        )?;
        run(
            "git",
            &["-C", database, "checkout", "--detach", revision],
            "select-advisories",
        )?;
    }
    cargo(
        &["audit", "--db", database, "--no-fetch", "--json"],
        "advisories-pinned",
        toolchain,
    )?;
    Ok(())
}

fn verify_release(toolchain: Option<&str>) -> TaskResult<()> {
    if !cfg!(target_os = "macos") {
        return Err("Release verification requires a logged-in Mac".into());
    }
    if Path::new(assurance::RECEIPT).exists() {
        fs::remove_file(assurance::RECEIPT)?;
    }
    let source = assurance::source_digest(Path::new("."))?;
    check(toolchain)?;
    build_native("Release", toolchain)?;
    bundle::inspect(
        Path::new("build/xcode/Build/Products/Release/QRScanner.app"),
        false,
    )?;
    for instrumentation in [
        Instrumentation::Plain,
        Instrumentation::Address,
        Instrumentation::Thread,
    ] {
        native_tests(true, true, instrumentation)?;
    }
    verification::bounded(verification::Campaign::Proofs)?;
    verification::bounded(verification::Campaign::Mutations)?;
    verification::bounded(verification::Campaign::Fuzz)?;
    if source != assurance::source_digest(Path::new("."))? {
        return Err("Source changed during release verification; no receipt was issued".into());
    }
    assurance::save_receipt()?;
    println!("[qrscanner] full automated release verification passed");
    Ok(())
}

#[derive(Clone, Copy)]
enum Instrumentation {
    Plain,
    Address,
    Thread,
}

fn native_tests(
    release: bool,
    all_architectures: bool,
    instrumentation: Instrumentation,
) -> TaskResult<()> {
    let developer = run("xcode-select", &["-p"], "developer-directory")?;
    let frameworks = format!(
        "{}/Platforms/MacOSX.platform/Developer/Library/Frameworks",
        String::from_utf8(developer.stdout)?.trim()
    );
    let support = format!("{}/../../usr/lib", frameworks);
    let sdk = run(
        "xcrun",
        &["--sdk", "macosx", "--show-sdk-path"],
        "sdk-directory",
    )?;
    let sdk = String::from_utf8(sdk.stdout)?;
    let mut sources = Vec::new();
    for directory in ["native/App", "native/Shared", "native/Tests"] {
        for item in fs::read_dir(directory)? {
            let path = item?.path();
            if path.extension().is_some_and(|value| value == "swift")
                && path
                    .file_name()
                    .is_some_and(|value| value != "QRScannerApp.swift")
            {
                sources.push(path.to_string_lossy().into_owned());
            }
        }
    }
    sources.sort();
    sources.push("native/Tools/TestRunner.swift".to_owned());
    let mut failures = Vec::new();
    for architecture in ["arm64"] {
        if !all_architectures
            && architecture
                != if cfg!(target_arch = "aarch64") {
                    "arm64"
                } else {
                    "x86_64"
                }
        {
            continue;
        }
        let target = format!("{architecture}-apple-macos27.0");
        let mode = match instrumentation {
            Instrumentation::Plain => {
                if release {
                    "release"
                } else {
                    "debug"
                }
            }
            Instrumentation::Address => "address",
            Instrumentation::Thread => "thread",
        };
        let executable = format!("build/native-tests-{mode}-{architecture}");
        let mut args = vec![
            "swiftc",
            "-parse-as-library",
            "-swift-version",
            "6",
            "-strict-concurrency=complete",
            "-warnings-as-errors",
            "-sdk",
            sdk.trim(),
            "-module-cache-path",
            "build/test-module-cache",
            "-module-name",
            "QRScannerVerification",
            "-import-objc-header",
            "native/Bridge/QRScannerCore.h",
            "-L",
            "build/rust/lib",
            "-lqrscanner_ffi",
            "-F",
            &frameworks,
            "-framework",
            "XCTest",
            "-Xlinker",
            "-rpath",
            "-Xlinker",
            &frameworks,
            "-I",
            &support,
            "-L",
            &support,
            "-lXCTestSwiftSupport",
            "-Xlinker",
            "-rpath",
            "-Xlinker",
            &support,
            "-target",
            &target,
            "-o",
            &executable,
        ];
        args.extend(sources.iter().map(String::as_str));
        if release {
            args.push("-O");
        }
        match instrumentation {
            Instrumentation::Plain => {}
            Instrumentation::Address => args.extend(["-g", "-sanitize=address"]),
            Instrumentation::Thread => args.extend(["-g", "-sanitize=thread"]),
        }
        let environment: &[(&str, &str)] = match instrumentation {
            Instrumentation::Plain => &[],
            Instrumentation::Address => &[(
                "ASAN_OPTIONS",
                "halt_on_error=1:abort_on_error=1:detect_leaks=0",
            )],
            Instrumentation::Thread => &[("TSAN_OPTIONS", "halt_on_error=1:exitcode=66")],
        };
        if let Err(error) = run(
            "xcrun",
            &args,
            &format!("compile-tests-{mode}-{architecture}"),
        ) {
            failures.push(error.to_string());
            continue;
        }
        if let Err(error) = run_with_environment(
            "arch",
            &[
                &format!("-{architecture}"),
                &executable,
                &format!(
                    "build/xcode/Build/Products/{}/QRScanner.app",
                    if release { "Release" } else { "Debug" }
                ),
            ],
            &format!("native-tests-{mode}-{architecture}"),
            environment,
        ) {
            failures.push(error.to_string());
        }
    }
    if !failures.is_empty() {
        return Err(failures.join("\n").into());
    }
    Ok(())
}
