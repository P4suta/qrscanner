FROM docker.io/library/rust:1.100.0-bookworm@sha256:ade39a710ef85f3506cffcfec3e8a24ba090badef1dff2fb798f71d878833fea
ENV CARGO_BUILD_JOBS=2
COPY --from=verification-tools /usr/local/cargo/bin/cargo-kani /usr/local/cargo/bin/cargo-kani
COPY --from=verification-tools /usr/local/cargo/bin/kani /usr/local/cargo/bin/kani
COPY --from=verification-tools /usr/local/cargo/bin/cargo-mutants /usr/local/cargo/bin/cargo-mutants
COPY --from=verification-tools /root/.kani /root/.kani
COPY --from=verification-tools /usr/local/rustup/toolchains/nightly-2026-08-21-x86_64-unknown-linux-gnu /usr/local/rustup/toolchains/nightly-2026-08-21-x86_64-unknown-linux-gnu
RUN cargo kani --version && cargo mutants --version
RUN cargo install --locked cargo-fuzz --version 0.13.1
WORKDIR /source
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY fuzz ./fuzz
COPY xtask ./xtask
RUN cargo fetch --locked
