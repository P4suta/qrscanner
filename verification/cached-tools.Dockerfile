FROM docker.io/library/rust:1.99.0-bookworm@sha256:114c7a4425406451c2866b6aafe69fe29b1b298832db1277d411ac73c82d04d6
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
