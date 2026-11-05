#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    qrscanner_fuzz::check_version(data);
});
