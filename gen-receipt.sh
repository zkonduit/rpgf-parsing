#!/bin/bash

# Initialize variables
DEV_MODE=0
PROJECT_ID=""
FILTER_BY_AMOUNTS=""

# Parse command line arguments
while [[ "$#" -gt 0 ]]; do
    case $1 in
        --dev) DEV_MODE=1 ;;
        --project-id) PROJECT_ID="$2"; shift ;;
        --filter-by-amounts) FILTER_BY_AMOUNTS="$2"; shift ;;
    esac
    shift
done

# Error handling for incompatible arguments
if [ -n "$PROJECT_ID" ] && [ -n "$FILTER_BY_AMOUNTS" ]; then
    echo "Error: --project-id and --filter_by_amounts cannot be used together."
    exit 1
fi

# First command
cargo build --release

# Second command
./target/release/private_processing

# Third command
cargo build --release

# Fourth command, conditional on flags and arguments
if [ $DEV_MODE -eq 1 ]; then
    PREFIX="RISC0_DEV_MODE=1 "
else
    PREFIX="RISC0_DEV_MODE=0 "
fi

if [ -n "$PROJECT_ID" ]; then
    CMD="${PREFIX}cargo run --release -F metal --bin op-rpgf -- --project_id=$PROJECT_ID"
elif [ -n "$FILTER_BY_AMOUNTS" ]; then
    CMD="${PREFIX}cargo run --release -F metal --bin op-rpgf -- --aggregate --filter_by_amounts=$FILTER_BY_AMOUNTS"
else
    CMD="${PREFIX}cargo run --release -F metal --bin op-rpgf -- --aggregate"
fi

eval $CMD
