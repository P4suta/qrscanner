FROM docker.io/library/rust:1.99.1-bookworm@sha256:c9155318d07341301a78b45ecdd81af2e43b8b297d64e8fd4c5e80d7735607c3
ENV CARGO_BUILD_JOBS=2
COPY --from=verification-tools /usr/local/cargo/bin/cargo-kani /usr/local/cargo/bin/cargo-kani
COPY --from=verification-tools /usr/local/cargo/bin/kani /usr/local/cargo/bin/kani
COPY --from=verification-tools /usr/local/cargo/bin/cargo-mutants /usr/local/cargo/bin/cargo-mutants
COPY --from=verification-tools /root/.kani /root/.kani
COPY --from=verification-tools /usr/local/rustup/toolchains/nightly-2026-08-21-x86_64-unknown-linux-gnu /usr/local/rustup/toolchains/nightly-2026-08-21-x86_64-unknown-linux-gnu
RUN cargo kani --version && cargo mutants --version
WORKDIR /source
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY xtask ./xtask
RUN cargo fetch --locked
