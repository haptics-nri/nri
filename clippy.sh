#!/usr/bin/env bash

# save Cargo config
cp Cargo.toml{,.bak}

# save multirust state
HAD_OVERRIDE=1
multirust show-override | grep -q 'multirust: no override' && HAD_OVERRIDE=
OLD_OVERRIDE=stable
rustc -V | grep -q beta && OLD_OVERRIDE=beta
rustc -V | grep -q nightly && OLD_OVERRIDE=nightly

# save main.rs
cp src/main.rs{,.bak}

# go!
echo 'clippy = "*"' >>Cargo.toml
echo -e '0a\n#![feature(plugin)]\n#![plugin(clippy)]\n.\n,wq' | ed src/main.rs >/dev/null
multirust override nightly >/dev/null
cargo build

# recover
mv src/main.rs{.bak,}
mv Cargo.toml{.bak,}
if [ $HAD_OVERRIDE ]; then
    multirust override $OLD_OVERRIDE >/dev/null
else
    multirust remove-override >/dev/null
fi

