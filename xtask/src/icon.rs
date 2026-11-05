use crate::TaskResult;
use std::fs;

const CHUNK_HEADER_BYTES: u64 = 8;

fn chunk_size(length: u64) -> Option<u32> {
    length
        .checked_add(CHUNK_HEADER_BYTES)
        .and_then(|size| u32::try_from(size).ok())
}

pub(crate) fn package() -> TaskResult<()> {
    let entries = [
        ("icp4", "icon_16x16.png"),
        ("icp5", "icon_32x32.png"),
        ("icp6", "icon_32x32@2x.png"),
        ("ic07", "icon_128x128.png"),
        ("ic08", "icon_256x256.png"),
        ("ic09", "icon_512x512.png"),
        ("ic10", "icon_512x512@2x.png"),
        ("ic11", "icon_16x16@2x.png"),
        ("ic12", "icon_32x32@2x.png"),
        ("ic13", "icon_128x128@2x.png"),
        ("ic14", "icon_256x256@2x.png"),
    ];
    let mut chunks = Vec::new();
    for (kind, name) in entries {
        let png = fs::read(format!("build/assets/AppIcon.iconset/{name}"))?;
        if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err("The icon renderer did not produce PNG data".into());
        }
        let size = chunk_size(u64::try_from(png.len())?).ok_or("icon chunk is too large")?;
        chunks.extend_from_slice(kind.as_bytes());
        chunks.extend_from_slice(&size.to_be_bytes());
        chunks.extend_from_slice(&png);
    }
    let size = chunk_size(u64::try_from(chunks.len())?).ok_or("icon family is too large")?;
    let mut icon = b"icns".to_vec();
    icon.extend_from_slice(&size.to_be_bytes());
    icon.extend_from_slice(&chunks);
    fs::write("build/assets/AppIcon.icns", icon)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::chunk_size;

    #[test]
    fn lengths_include_the_header_and_reject_overflow() {
        assert_eq!(chunk_size(0), Some(8));
        assert_eq!(chunk_size(u64::from(u32::MAX) - 8), Some(u32::MAX));
        assert_eq!(chunk_size(u64::from(u32::MAX) - 7), None);
        assert_eq!(chunk_size(u64::MAX), None);
    }
}

#[cfg(kani)]
#[kani::proof]
fn icon_lengths_are_checked() {
    let length: u64 = kani::any();
    match chunk_size(length) {
        Some(size) => {
            assert!(length <= u64::from(u32::MAX) - 8);
            assert_eq!(u64::from(size), length + 8);
        }
        None => assert!(length > u64::from(u32::MAX) - 8),
    }
}
