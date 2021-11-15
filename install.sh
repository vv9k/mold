#!/bin/sh

set -e

cargo b --release
cp ./target/release/mold ~/bin/
