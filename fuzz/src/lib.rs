use qrscanner_verification::MacOS27;

pub fn check_version(data: &[u8]) {
    let expected = std::str::from_utf8(data).ok().is_some_and(|text| {
        let Some(tail) = text.strip_prefix("27.") else {
            return false;
        };
        let components: Vec<_> = tail.split('.').collect();
        matches!(components.len(), 1 | 2)
            && components.iter().all(|component| {
                !component.is_empty() && component.bytes().all(|byte| byte.is_ascii_digit())
            })
    });
    let parsed = MacOS27::parse(data);
    assert_eq!(parsed.is_some(), expected);
    if let Some(value) = parsed {
        assert_eq!(value.as_bytes(), data);
        assert_eq!(MacOS27::parse(value.as_bytes()), Some(value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_every_committed_case_and_check_round_trips() -> Result<(), Box<dyn std::error::Error>>
    {
        let seeds: Vec<Vec<u8>> =
            serde_json::from_slice(include_bytes!("../corpus/macos_version/seeds.json"))?;
        assert!(!seeds.is_empty());
        assert!(seeds.iter().any(|seed| MacOS27::parse(seed).is_some()));
        assert!(seeds.iter().any(|seed| MacOS27::parse(seed).is_none()));
        for seed in seeds {
            check_version(&seed);
        }
        Ok(())
    }
}
