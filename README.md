# zk-rpgf-ballots

## Setup

1. **Install Rust** 
2. **Install Risc0 toolchain**:
```bash
cargo install cargo-binstall
yes | cargo binstall cargo-risczero
cargo risczero install
```
3. **Install GNU Time**:

for Linux:
```bash
sudo apt-get install time
```
for MacOS:
```bash
brew install gnu-time
```
4. **Add wasm32-unknown-unknown target**:
```bash
rustup target add wasm32-unknown-unknown
```
5. **Add rust-src**:
```bash
rustup component add rust-src --toolchain nightly-2024-01-16
```
6. **Install Node version 18.12.1**

## Build the project
    
```bash
cargo build --release
```

## Generate sample ballot data

```bash
target/release/ballot_generator --badgeholder_count <usize> --ballot_count <usize>
```

## Run private pre-processing step on the ballot data.

```bash
target/release/ballot_preprocess --badgeholder_count <uszie> --processed_inputs <path_to_output_file>
```

## Generate the proof (aka receipt) by running the Guest and verifying it in the Host.

This command will run the guest code that hashes vote amounts for the specified project id, commiting the single hash.

```bash
cargo run --release --bin op-rpgf -- --project_id <hex_string>
--receipt <path_to_store_receipt_file> --image_id <path_to_store_image_file> --votes_table <path_to_votes_table_file>
```

Pass the `--aggregate` flag to hash the vote amounts for all of the projects, commiting to a vector of hashes.

```bash
cargo run --release --bin op-rpgf -- --aggregate --receipt <path_to_receipt_file> --votes_table <path_to_votes_table_file>
```  

## Test in-browser verification and receipt instances parsing.

```bash
npm test -- --<BROWSER>
```

## Run benchmarks (with WASM testing)

```bash
 cargo nextest run benchmarking_tests::run_rpgf_ballots_benchmarks_wasm_ --no-capture
```

## Run benchmarks (without WASM testing)

```bash
 cargo nextest run benchmarking_tests::run_rpgf_ballots_benchmarks_native_ --no-capture
```

## Profiling the guest code (projects.rs)

Make sure to install go first before running the following command:

```bash
RISC0_PPROF_OUT=op_rpgf.pb RISC0_DEV_MODE=1 cargo run --release -F metal --bin op-rpgf -- --aggregate
go tool pprof -http 127.0.0.1:8000 op_rpgf.pb
```

## Stark to groth16 proof conversion (This feat is only for x86 architecture. For Mac you will need to run the following commands in a Rosetta 2 terminal)

First, build the prover image:

```bash
cd compact_proof
bash ./scripts/install_prover.sh
```

Then, generate the groth16 proof by passing the `--groth16` flag to the `op-rpgf` command:

```bash
cargo run --release --bin op-rpgf -- --aggregate --groth16 --receipt <path_to_receipt_file> --votes_table <path_to_votes_table_file>
```




