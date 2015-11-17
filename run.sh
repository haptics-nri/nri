#!/bin/bash

export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:crates/drivers/optoforce
export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:crates/drivers/biotac/src/wrapper
export RUST_BACKTRACE=1

if [ "$#" -ne 0 ]; then 
    DEV=$1
    shift
    rlwrap cargo run --release --example read$DEV -- "$@"
else
    rlwrap cargo run --release
fi

