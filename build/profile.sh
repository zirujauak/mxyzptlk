#!/bin/zsh

## Build and run tests with instrumentation
cargo clean
RUSTFLAGS="-C instrument-coverage" cargo --quiet test --tests

## Find the target binary
target=$(find target/debug/deps -regex ".*/mxyzptlk-[^.]*")

## Generate and view report
xcrun llvm-profdata merge -sparse default_*.profraw -o json5format.profdata
xcrun llvm-cov show --use-color \
    --ignore-filename-regex='/.cargo/registry' \
    --instr-profile=json5format.profdata \
    --object ${target} \
    --show-instantiations \
    --show-line-counts-or-regions \
    --Xdemangler=rustfilt | less -R

## Clean up profiling data
rm *.profraw
rm json5format.profdata