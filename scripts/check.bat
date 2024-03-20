call cargo +nightly udeps
call cargo +stable fmt --all -- --check
call cargo +stable clippy --all-targets -- -D warnings