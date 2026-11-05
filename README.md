# QRScanner

Scan QR codes on your Mac screen and open their links in your browser.

## Installation

```sh
mise trust
mise install
mise x -- rustup component add rustfmt clippy
rustup target add --toolchain 1.99.1 aarch64-apple-darwin x86_64-apple-darwin
cargo fetch --locked
QRSCANNER_DEVELOPMENT_IDENTITY=YOUR_CERTIFICATE_SELECTOR mise x -- just app
```

## Usage

```sh
open build/QRScanner.app
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE-APACHE) or the [Massachusetts Institute of Technology (MIT) license](LICENSE-MIT), at your option.

Contributions use both licenses unless you state other terms.
