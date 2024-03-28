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

// hardcode the verifying keys
pub const VERIFYING_KEYS: &[u8] = include_bytes!("../../../../verifying_keys");

pub const POSEIDON_LEN_GRAPH: usize = 32;

// Alies for the string type that will contain the hex string ballot id

fn main() {
    let inputs: Vec<(Vec<u8>, Vec<u8>)> = env::read();
    // the project id we will hash the votes for
    let project_id: String = env::read();

    // Initialize the hashmap to store project IDs and their associated vote amounts
    let mut votes: Vec<Fp> = Vec::new();

    let vks: Vec<Vec<u8>> = bincode::deserialize(VERIFYING_KEYS).unwrap();

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
        if let Some(array) = data.as_array() {
            if let Some(vote) = array
                .iter()
                .find(|vote| vote["projectId"].as_str().unwrap() == project_id)
            {
                let amount_string = vote["amount"].as_str().unwrap();
                // Assume the rest of the code is correct and follows from here...
                let amount = amount_string.parse::<f64>().unwrap() as i128;
                // push the amount_felt to the project_votes vector.
                let amount_felt = poseidon::i128_to_felt(amount);
                // push the amount_felt to the project_votes vector.
                votes.push(amount_felt);
            }
        }
    }

    if votes.len() != 0 {
        let vote_amounts_hashes: Vec<Vec<Fp>> =
            poseidon::poseidon::<POSEIDON_LEN_GRAPH, poseidon::PoseidonSpec>(votes).unwrap();
        // Need to convert the hash to custom felt type with the Pod and Zeroable traits for
        // wasm serialization of the journal instances.
        let hash = Felt(vote_amounts_hashes[0][0].into());

        env::commit(&hash);
    } else {
        env::commit(&Felt([0, 0, 0, 0]));
    }
}
