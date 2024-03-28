// TODO: Move main function to here.
use bytemuck::Pod;
use bytemuck::Zeroable;
use clap::{App, Arg};
use ethers::types::Signature as EthSig;
use ethers::types::H160;
use ethers::utils::hash_message;
use ethers::utils::keccak256;
use ethers::core::types::H256;
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::{fs, str::FromStr};
use csv::Writer;
#[derive(Copy, Clone, Pod, Serialize, Deserialize, Zeroable, Debug, PartialEq)]
#[repr(C)]
struct Felt([u64; 4]);
fn main() {
    let matches = App::new("zkrpgf")
        .version("1.0")
        .about("")
        .arg(
            Arg::with_name("badgeholder_count")
                .short('R')
                .long("badgeholder_count")
                .takes_value(true)
                .help("Specifies the number of records to process"),
        ).arg(
            Arg::with_name("votes_table")
                .long("votes_table")
                .takes_value(true)
                .help("Specifies the file path to write the votes table of (project) -> (votes amounts) to"),
        )
        .get_matches();

    let votes_table_path = matches
        .value_of("votes_table")
        .unwrap_or("votes_table.csv");

    let record_count = matches.value_of("badgeholder_count").unwrap_or("0");
    let record_count = record_count.parse::<usize>().unwrap();

    // Path to your CSV file
    let file_path = "./rpgf_ballots.csv";

    // Open the CSV file
    let file = File::open(file_path).expect("Could not find rpgf_ballots.csv, please run the ballot_generator binary to generate the file");

    // Create a CSV reader
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(file);

    // Create an iterator over records
    let records_iter = rdr.records();

    // Apply `.take()` conditionally based on `record_count`
    let records_boxed_iter: Box<dyn Iterator<Item = csv::Result<csv::StringRecord>>> =
        if record_count > 0 {
            Box::new(records_iter.take(record_count))
        } else {
            Box::new(records_iter)
        };

    let mut verifying_keys: Vec<Vec<u8>> = Vec::new();

    // Iterate over each record
    let tuples: Vec<(Vec<u8>, Vec<u8>)> = records_boxed_iter
        .map(|result| {
            // Deserialize each row into the Record struct
            let record = result.unwrap();
            // get the ethereum address, signature and json from the record
            let address = &record[0];
            let address: H160 = H160::from_str(&address[2..]).unwrap();

            let signature = &record[1];

            let ballot_data = record[2].to_owned();

            // get eth sig to be used to ensure the derived address from the verifying key matches the address in the csv
            let eth_signature = EthSig::from_str(signature).unwrap();

            // strip the 0x prefix from the signature and the last byte
            let signature = &signature[2..signature.len()];

            let sig_bytes = hex::decode(signature).unwrap();

            let sig_minus_rec = &sig_bytes[0..64];

            let sig = Signature::try_from(sig_minus_rec).unwrap();

            let recovery_id_byte: u8 = sig_bytes[64];

            let recovery_id_byte = if recovery_id_byte == 27 { 0 } else { 1 };

            let recid = RecoveryId::try_from(recovery_id_byte).unwrap();

            let message_hash = hash_message(&ballot_data);

            let mut verifying_key: VerifyingKey =
                VerifyingKey::recover_from_prehash(message_hash.as_bytes(), &sig, recid).unwrap();

            let address_derived = eth_signature.recover(message_hash).unwrap();
            println!("{:?}, {:?}",&address_derived, &address);

            if address_derived != address {
                println!("ballot message hash: {:?}", message_hash);
                let khash = keccak256(&ballot_data.as_bytes());
                let khash_as_h256 = H256::from(khash); 
                let message_hash = hash_message(format!("{:?}",khash_as_h256));

                verifying_key =
                    VerifyingKey::recover_from_prehash(message_hash.as_bytes(), &sig, recid)
                        .unwrap();

                let address_derived = eth_signature.recover(message_hash).unwrap();

                assert!(
                    address_derived == address,
                    "Address {:?} derived from signature does not match the address {:?} in the record",&address_derived,&address
                );
            } else {
                assert!(
                    address_derived == address,
                    "Address derived from signature does not match the address in the record"
                );
            }
            verifying_keys.push(verifying_key.to_sec1_bytes().to_vec());
            (ballot_data.as_bytes().to_owned(), sig_minus_rec.to_vec())
        })
        .collect();

        // Initialize the hashmap to store project IDs and their associated vote amounts
        let mut votes_table: BTreeMap<String, Vec<i128>> = BTreeMap::new();

        for (ballots, _) in tuples.iter() {
            let data: serde_json::Value = serde_json::from_slice(&ballots).unwrap();
            // Process the ballots to populate the votes_table hashmap
            if let Some(array) = data.as_array() {
                array.iter().for_each(|vote| {
                    let project_id = vote["projectId"].as_str().unwrap();
                    let amount_string = vote["amount"].as_str().unwrap();
                    let vote_amount = amount_string.parse::<f64>().unwrap() as i128;
                    let amounts_array = votes_table
                        .entry(project_id.to_string())
                        .or_insert_with(Vec::new);
                    amounts_array.push(vote_amount);
                });
            }
        }

        // Write the votes_table hashmap to a file
        fs::write("votes_table.json", serde_json::to_vec(&votes_table).unwrap()).unwrap();

        // Serialize the votes_table hashmap into CSV format and write to a file
        let mut wtr = Writer::from_path(votes_table_path).expect("Unable to create CSV writer");

        // Write CSV header
        wtr.write_record(&["Project ID", "Vote Amounts"]).expect("Unable to write header");

        // Iterate over the votes_table and write each entry as a CSV row
        for (project_id, vote_amounts) in votes_table.iter() {
            // Join all vote amounts with ";" to keep them in one column, or handle as needed
            let amounts_str = vote_amounts.iter()
                                            .map(|amount| amount.to_string())
                                            .collect::<Vec<String>>().join(";");
            wtr.write_record(&[project_id, &amounts_str]).expect("Unable to write record");
        }

        // Ensure all data is flushed to the file
        wtr.flush().expect("Failed to flush CSV writer");
    
        // collect all the keys of the votes_table hashmap
        let votes_table_keys: Vec<String> = votes_table.keys().cloned().collect();
    
        // Write the votes_table_keys to a file serde json serialized
        fs::write(
            "project_ids",
            serde_json::to_vec(&votes_table_keys).unwrap(),
        )
        .unwrap();

    
    // serialize the tuples to a json string using bincode
    let tuples_serialized = bincode::serialize(&tuples).unwrap();
    // write the tuples to a file
    fs::write("processed_inputs", tuples_serialized).expect("Unable to write file");

    let verifying_keys_serialized = bincode::serialize(&verifying_keys).unwrap();
    // write the verifying keys to a file
    fs::write("verifying_keys", verifying_keys_serialized).expect("Unable to write file");
}
