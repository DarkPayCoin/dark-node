#!/usr/bin/env bash

set -e

./target/release/dark-node purge-chain --dev -y
./target/release/dark-node --dev
