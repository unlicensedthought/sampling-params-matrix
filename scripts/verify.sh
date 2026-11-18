#!/usr/bin/env bash
# Runs build, tests, lint and a CLI smoke check. The last stdout line is the verdict.
set -u
cd "$(dirname "$0")/.."
gates=0
fail() { echo "VERIFY {\"verdict\":\"fail\",\"failed\":\"$1\"}"; exit 1; }

cargo build --release >&2 || fail build
gates=$((gates+1))

cargo test >&2 || fail test
gates=$((gates+1))

cargo clippy --all-targets -- -D warnings >&2 || fail clippy
gates=$((gates+1))

bin=target/release/sampling-params-matrix
out=$("$bin" o3 --endpoint https://api.openai.com/v1 --temp 0.7) || fail cli_rejected
case "$out" in *'"send":false'*'rejected_while_thinking'*) ;; *) fail cli_rejected ;; esac
out=$("$bin" qwen3 --endpoint http://127.0.0.1:11434/v1) || fail cli_honored
case "$out" in *'"send":true'*) ;; *) fail cli_honored ;; esac
if "$bin" >/dev/null 2>&1; then fail cli_usage_exit_code; fi
gates=$((gates+1))

echo "VERIFY {\"verdict\":\"pass\",\"gates\":$gates}"
