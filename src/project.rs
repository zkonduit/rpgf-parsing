use bytemuck::Pod;
use bytemuck::Zeroable;
use clap::ArgMatches;
use risc0_zkvm::ExecutorEnv;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Pod, Serialize, Deserialize, Zeroable, Debug, PartialEq)]
#[repr(C)]
struct Felt([u64; 4]);

pub fn single_project_votes(
    matches: &ArgMatches,
    inputs: Vec<(Vec<u8>, Vec<u8>)>,
) -> ExecutorEnv<'_> {
    let project_id = matches.value_of("project_id").unwrap_or("0");
    // get the project id from the inputs
    let project_id_hex = if project_id == "0" {
        let ballot_data_bytes = inputs[0].0.clone();

        let ballot_data = String::from_utf8(ballot_data_bytes).unwrap();

        let ballot_data: serde_json::Value = serde_json::from_str(&ballot_data).unwrap();

        ballot_data[0]["projectId"].as_str().unwrap().to_owned()
    } else {
        project_id.to_string()
    };

    let env: ExecutorEnv<'_> = ExecutorEnv::builder()
        .write(&inputs)
        .expect("Failed to deserialize inputs")
        .write(&project_id_hex)
        .expect("Failed to deserialize project_id")
        .build()
        .unwrap();
    env
}
