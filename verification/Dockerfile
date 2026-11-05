FROM docker.io/library/rust:1.99.1-bookworm@sha256:c9155318d07341301a78b45ecdd81af2e43b8b297d64e8fd4c5e80d7735607c3
ENV CARGO_BUILD_JOBS=2
RUN cargo install --locked kani-verifier --version 0.68.0 \
    && cargo kani setup \
    && cargo install --locked cargo-mutants --version 27.1.0 \
    && cargo install --locked cargo-fuzz --version 0.13.1
WORKDIR /source
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY fuzz ./fuzz
COPY xtask ./xtask
RUN cargo fetch --locked
