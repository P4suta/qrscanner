FROM docker.io/library/rust:1.100.0-bookworm@sha256:ade39a710ef85f3506cffcfec3e8a24ba090badef1dff2fb798f71d878833fea
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
