#!/bin/bash

export LD_LIBRARY_PATH=src/optoforce
export RUST_BACKTRACE=1

if [ "$#" -ne 0 ]; then 
    DEV=$1
    shift
    rlwrap cargo run --verbose --example read$DEV -- "$@"
else
    rlwrap cargo run --verbose
fi

