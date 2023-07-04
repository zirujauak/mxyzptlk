#!/bin/zsh

cargo clean
RUSTFLAGS="-C instrument-coverage" cargo --quiet test --tests
xcrun llvm-profdata merge -sparse default_*.profraw -o json5format.profdata
xcrun llvm-cov report --use-color --ignore-filename-regex='/.cargo/registry' \
    --instr-profile=json5format.profdata \
    --object target/debug/deps/mxyzptlk-f9babb35b31800c3 \
    --show-instantiation-summary \
    --Xdemangler=rustfilt | less -R