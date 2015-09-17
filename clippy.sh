#!/usr/bin/env bash

# change symlink to point at nightly build
TARGET=$(readlink target)
rm target
ln -s target-nightly target

# run clippy
multirust run nightly cargo clippy

# cleanup
rm target
ln -s $TARGET target

