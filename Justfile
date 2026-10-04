profile CRATE="piru" *ARGS:
    cargo build --bin {{CRATE}} --profile=profiling
    samply record --address 127.0.0.1 -o /tmp/profile.json.gz -- ${CARGO_TARGET_DIR:-./target}/profiling/{{CRATE}} {{ARGS}}

measure-size CRATE="piru":
    cargo build --bin {{CRATE}} --release
    du -h ${CARGO_TARGET_DIR:-./target}/release/{{CRATE}}
