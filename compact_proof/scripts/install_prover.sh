#!/bin/bash

set -eoux

docker build -no-cache -f docker/prover.Dockerfile . -t risc0-groth16-prover
