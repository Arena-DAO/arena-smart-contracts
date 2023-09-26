call cargo +nightly udeps
call cargo +nightly fmt --all -- --check
call cargo +nightly clippy --all-targets -- -D warnings