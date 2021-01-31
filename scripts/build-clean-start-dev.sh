#!/usr/bin/env bash

set -e

cargo build --release
./target/release/dark-node purge-chain --dev -y
./target/release/dark-node --dev
