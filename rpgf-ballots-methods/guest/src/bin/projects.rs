use bytemuck::Pod;
use bytemuck::Zeroable;
use ethers_core::types::H256;
use ethers_core::utils::hash_message;
use ethers_core::utils::keccak256;
use halo2curves::bn256::Fr as Fp;
use k256::ecdsa::signature::hazmat::PrehashVerifier;
use k256::ecdsa::{Signature, VerifyingKey};
use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Copy, Clone, Pod, Serialize, Deserialize, Zeroable, Debug, PartialEq)]
#[repr(C)]
struct Felt([u64; 4]);

mod poseidon;

// hardcode the verifying keys and project ids
pub const VERIFYING_KEYS: &[u8] = include_bytes!("../../../../verifying_keys");

pub const POSEIDON_LEN_GRAPH: usize = 32;

pub const PROJECT_IDS_FIXED: &[u8] = include_bytes!("../../../../project_ids");

fn main() {
    let inputs: Vec<(Vec<u8>, Vec<u8>)> = env::read();

    // Array of project ids to filter by. If empty, all projects will be processed
    let project_ids_filter: Vec<String> = env::read();

    let vks: Vec<Vec<u8>> = bincode::deserialize(VERIFYING_KEYS).unwrap();

    let project_ids_fixed: Vec<String> = serde_json::from_slice(&PROJECT_IDS_FIXED).unwrap();

    // Initialize the project_votes vector. The vector length is equal to the number of projects
    let mut project_votes: Vec<Vec<Fp>> = vec![vec![]; project_ids_fixed.len()];

    for ((ballots, signature), verifying_key) in inputs.iter().zip(vks.iter()) {
        let signature = Signature::try_from(signature.as_ref()).unwrap();

        let verifying_key = VerifyingKey::from_sec1_bytes(verifying_key.as_ref()).unwrap();

        let message_hash = hash_message(&ballots);

        // if the signature verification fails the first time around, then double hash the ballot data
        // and try the sig check again
        let sig_check = verifying_key.verify_prehash(message_hash.as_bytes(), &signature);
        // Unwrap the result of the sig check, or redo the sig check with the double hashed ballot data
        match sig_check {
            Ok(_) => (),
            Err(_) => {
                let khash = keccak256(&ballots);
                let khash_as_h256 = H256::from(khash);
                let message_hash = hash_message(format!("{:?}", khash_as_h256));
                let sig_check = verifying_key.verify_prehash(message_hash.as_bytes(), &signature);
                sig_check.unwrap();
            }
        };

        let data: serde_json::Value = serde_json::from_slice(&ballots).unwrap();
        if project_ids_filter.len() == 0 {
            // Process the ballots to populate the project_votes hashmap
            if let Some(array) = data.as_array() {
                array.iter().for_each(|vote| {
                    populate_project_votes(
                        &project_ids_fixed,
                        &mut project_votes,
                        vote["projectId"].as_str().unwrap(),
                        vote["amount"].as_str().unwrap(),
                    );
                });
            }
        } else {
            // Process the ballots to populate the project_votes hashmap, filtering by project_ids
            if let Some(array) = data.as_array() {
                array.iter().for_each(|vote| {
                    let project_id = vote["projectId"].as_str().unwrap();
                    if project_ids_filter.contains(&project_id.to_string()) {
                        populate_project_votes(
                            &project_ids_fixed,
                            &mut project_votes,
                            project_id,
                            vote["amount"].as_str().unwrap(),
                        );
                    }
                });
            }
        }
    }

    let vote_amounts_hashes: Vec<Felt> = project_votes
        .iter()
        .filter(|vote_amounts| !vote_amounts.is_empty())
        .map(|vote_amounts| {
            Felt(
                poseidon::poseidon::<POSEIDON_LEN_GRAPH, poseidon::PoseidonSpec>(
                    vote_amounts.clone(),
                )
                .unwrap()[0][0]
                    .into(),
            )
        })
        .collect();

    env::commit(&vote_amounts_hashes);

    println!(
        "Total cycles for guest code execution: {}",
        env::cycle_count()
    );
}

//inline
#[inline]
pub fn populate_project_votes(
    project_ids_fixed: &Vec<String>,
    project_votes: &mut Vec<Vec<Fp>>,
    project_id: &str,
    vote_amount_string: &str,
) {
    let vote_amount = vote_amount_string.parse::<f64>().unwrap() as i128;
    let vote_amount_felt = poseidon::i128_to_felt(vote_amount);

    let project_index = project_ids_fixed
        .iter()
        .position(|x| x == project_id)
        .unwrap();

    project_votes[project_index].push(vote_amount_felt);
}
