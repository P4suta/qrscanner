set shell := ["zsh", "-cu"]

default:
    @just --list

app:
    cargo run --offline --locked -p xtask -- app

check:
    cargo run --offline --locked -p xtask -- check

localize:
    cargo run --offline --locked -p xtask -- localize

prepare-verification *args:
    cargo run --offline --locked -p xtask -- prepare-verification {{ args }}

verify-release:
    cargo run --offline --locked -p xtask -- verify-release

proofs:
    cargo run --offline --locked -p xtask -- proofs

mutants:
    cargo run --offline --locked -p xtask -- mutants

package *args:
    cargo run --offline --locked -p xtask -- package {{ args }}
