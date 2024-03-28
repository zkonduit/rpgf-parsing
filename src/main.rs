// Copyright 2024 Zkonduit Inc.,
// based on examples which are
// Copyright 2023 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use bytemuck::Pod;
use bytemuck::Zeroable;
use clap::{App, Arg};
use risc0_zkvm::default_prover;
use rpgf_ballots_methods::{PROJECTS_ELF, PROJECTS_ID, PROJECT_ELF, PROJECT_ID};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::time::Instant;
use std::vec;
#[derive(Copy, Clone, Pod, Serialize, Deserialize, Zeroable, Debug, PartialEq)]
#[repr(C)]
struct Felt([u64; 4]);

mod groth16;
mod project;
mod projects;

fn main() {
    let matches = App::new("zkrpgf")
        .version("1.0")
        .about("")
        .arg(
            Arg::with_name("project_id")
                .short('N')
                .long("project_id")
                .takes_value(true)
                .help("Specifies the project id that will be hashed)"),
        )
        .arg(
            Arg::with_name("receipt")
                .short('R')
                .long("receipt")
                .takes_value(true)
                .help("Specifies the file path to write the receipt to"),
        )
        .arg(
            Arg::with_name("image_id")
                .short('I')
                .long("image_id")
                .takes_value(true)
                .help("Specifies the file path to write the image id to"),
        )
        .arg(
            Arg::with_name("aggregate")
                .long("aggregate")
                .takes_value(false)
                .help("If set, the application will run with PROJECTS_ELF instead of PROJECT_ELF"),
        )
        .arg(
            Arg::with_name("groth16")
                .long("groth16")
                .takes_value(false)
                .help("If set, the generated receipt will be compressed from a stark proof to a groth16 proof"),
        )
        .arg(
            Arg::with_name("votes_table")
                .long("votes_table")
                .takes_value(true)
                .help("Specifies the file path to write the votes table of (project) -> (votes amounts) to"),
        ).arg(
            Arg::with_name("filter_by_amounts")
                .long("filter_by_amounts")
                .takes_value(true)
                .help("Specifies the ballot count range to filter the projects by"),
        )
        .get_matches();

    let votes_table_path = matches
        .value_of("votes_table")
        .unwrap_or("votes_table.json");

    // Open the processed inputs file without hardcoding the path
    let file = File::open("./processed_inputs").expect("Could not find processed_inputs file");

    // Read the processed inputs file
    let inputs: Vec<(Vec<u8>, Vec<u8>)> = bincode::deserialize_from(file).unwrap();

    let votes_table: BTreeMap<String, Vec<i128>> =
        serde_json::from_slice(&fs::read(votes_table_path).unwrap()).unwrap();

    // Determine which ELF to use
    if matches.is_present("aggregate") {
        let env = projects::aggregate_project_votes(&matches, votes_table, inputs);
        // If the groth16 flag is set, convert the receipt to a groth16 proof
        let receipt = if matches.is_present("groth16") {
            groth16::stark_to_groth16(env, &PROJECTS_ID, &PROJECTS_ELF)
        } else {
            prove_default_prover(env, &PROJECTS_ELF, &PROJECTS_ID)
        };
        let receipt_ser = bincode::serialize(&receipt).unwrap();
        let image_id = bincode::serialize(&PROJECTS_ID).unwrap();
        let receipt_path = matches
            .value_of("receipt")
            .unwrap_or("./browser-verify/receipt_aggr");
        let image_id_path = matches
            .value_of("image_id")
            .unwrap_or("./browser-verify/image_id_aggr");
        fs::write(receipt_path, receipt_ser).unwrap();
        fs::write(image_id_path, image_id).unwrap();
    } else {
        let env = project::single_project_votes(&matches, inputs);
        // If the groth16 flag is set, convert the receipt to a groth16 proof
        let receipt = if matches.is_present("groth16") {
            groth16::stark_to_groth16(env, &PROJECT_ID, &PROJECT_ELF)
        } else {
            prove_default_prover(env, &PROJECT_ELF, &PROJECT_ID)
        };
        let receipts_vec = vec![receipt.clone()];
        // store the reciept and image id in a file
        let receipts_ser = bincode::serialize(&receipts_vec).unwrap();
        let image_id = bincode::serialize(&PROJECT_ID).unwrap();
        let receipt_path = matches
            .value_of("receipt")
            .unwrap_or("./browser-verify/receipts");
        let image_id_path = matches
            .value_of("image_id")
            .unwrap_or("./browser-verify/image_id");
        fs::write(receipt_path, receipts_ser).unwrap();
        fs::write(image_id_path, image_id).unwrap();
    };
}

fn prove_default_prover(
    env: risc0_zkvm::ExecutorEnv,
    elf: &[u8],
    image_id: &[u32; 8],
) -> risc0_zkvm::Receipt {
    let prover = default_prover();
    let start_time = Instant::now();
    let receipt = prover.prove(env, elf).unwrap();
    let proving_time = start_time.elapsed();
    println!("Proving time: {:?}", proving_time);
    receipt.verify(*image_id).unwrap();
    receipt
}
