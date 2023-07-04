#!/bin/zsh
target=$(find target/debug/deps -regex ".*/mxyzptlk-[^.]*")

cargo clean
RUSTFLAGS="-C instrument-coverage" cargo --quiet test --no-default-features --tests
xcrun llvm-profdata merge -sparse default_*.profraw -o json5format.profdata
xcrun llvm-cov show --use-color \
    --ignore-filename-regex='/.cargo/registry' \
    --instr-profile=json5format.profdata \
    --object ${target} \
    --show-instantiations \
    --show-line-counts-or-regions \
    --Xdemangler=rustfilt | less -R

rm *.profraw
rm json5format.profdata