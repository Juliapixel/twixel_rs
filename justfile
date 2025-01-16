_default:
    just --list

fmt:
    cargo fmt

test:
    cargo test

run:
    cargo run

run_example:
    cd ./examples/raw_connection && cargo run

bench:
    cd ./twixel_core && cargo bench
