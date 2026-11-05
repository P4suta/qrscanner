use crate::{TaskResult, run};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub(crate) fn sync(check: bool) -> TaskResult<()> {
    let source = Path::new("native/Resources/Localizable.xcstrings");
    let destination = if check {
        Path::new("build/localization/Localizable.xcstrings")
    } else {
        source
    };
    if check {
        fs::create_dir_all("build/localization")?;
        fs::copy(source, destination)?;
    }
    let directory = Path::new(
        "build/xcode/Build/Intermediates.noindex/QRScanner.build/Debug/QRScanner.build/Objects-normal/arm64",
    );
    let mut files = fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    files.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "stringsdata")
    });
    files.sort();
    if files.is_empty() {
        return Err("No compiler-extracted localization keys; build the Debug app first".into());
    }
    let destination = destination
        .to_str()
        .ok_or("The catalog path is not UTF-8")?;
    let names = files
        .iter()
        .map(|path| {
            path.to_str()
                .ok_or("An extracted localization path is not UTF-8")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut arguments = vec!["xcstringstool", "sync", destination, "--stringsdata"];
    arguments.extend(names);
    run(
        "xcrun",
        &arguments,
        if check {
            "localization-check"
        } else {
            "localize"
        },
    )?;
    if check
        && serde_json::from_slice::<Value>(&fs::read(source)?)?
            != serde_json::from_slice::<Value>(&fs::read(destination)?)?
    {
        return Err("The String Catalog differs from compiler-extracted UI text; run mise x -- just localize".into());
    }
    Ok(())
}
