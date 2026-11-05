set shell := ["sh", "-eu", "-c"]

default:
    @just --list

app *args:
    cargo run --offline --locked -p xtask -- app {{ args }}

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

mutation-plan:
    cargo run --offline --locked -p xtask -- mutation-plan

fuzz:
    cargo run --offline --locked -p xtask -- fuzz

package *args:
    cargo run --offline --locked -p xtask -- package {{ args }}
