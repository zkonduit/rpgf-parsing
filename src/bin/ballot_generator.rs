use clap::{App, Arg};
use csv::Writer;
use ethers::prelude::*;
use ethers::utils::keccak256;
use rand::{
    distributions::{Distribution, Uniform},
    thread_rng,
};
use serde::{Deserialize, Serialize};
use std::{error::Error, fs::File, vec};

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
struct BallotData {
    projectId: String,
    amount: String,
}

/// Generates fake ballot data of a given shape (number of ballots and number of projects voted on per ballot)
/// and writes it to a CSV file. The format of the ballot data is the same as the one used in the real RPGF ballot data.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("Ballot Generator")
        .version("1.0")
        .about("Generates a CSV with Ethereum addresses, signatures, and ballots")
        .arg(
            Arg::with_name("badgeholder_count")
                .long("badgeholder_count")
                .help("Specifies the number of rows in the CSV")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("ballot_count")
                .long("ballot_count")
                .help("Specifies the length of the ballots data vector")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("csv_path")
                .long("csv_path")
                .help("Specifies the file path to write the CSV ballot data to")
                .takes_value(true)
                .required(false),
        )
        .get_matches();

    let badgeholder_count: usize = matches.value_of("badgeholder_count").unwrap().parse()?;
    let ballot_count: usize = matches.value_of("ballot_count").unwrap().parse()?;

    let mut project_ids = vec![];

    // Generate random project ids
    for _ in 0..ballot_count {
        let project_id = format!("0x{}", hex::encode(rand::random::<[u8; 32]>()));
        project_ids.push(project_id);
    }

    // Setup CSV writer
    let file_path = matches.value_of("csv_path").unwrap_or("rpgf_ballots.csv");
    let file = File::create(file_path)?;
    let mut wtr = Writer::from_writer(file);

    for i in 0..badgeholder_count {
        // Generate a new wallet from random private key
        let wallet = LocalWallet::new(&mut rand::thread_rng());

        // Generate ballot data
        let ballots = generate_ballots(ballot_count, &project_ids)?;

        // create json ballot data
        let ballot_data_str = serde_json::to_string(&ballots)?;

        // double hash for two specific records
        let message = if i == 0 || i == 1 {
            let k_hash_hash = keccak256(ballot_data_str.as_bytes());
            let k_hash_as_h256 = H256::from(k_hash_hash);
            format!("{:?}", k_hash_as_h256)
        } else {
            ballot_data_str.clone()
        };

        // Sign the hash
        let signature = wallet.sign_message(message.clone()).await?;

        // generate a random boolean value and add that as a column to the csv
        let random_bool = rand::random::<bool>();

        // convert the random bool to uppercase string
        let random_bool_str = random_bool.to_string().to_uppercase();

        // Write the data to the CSV file
        wtr.write_record(&[
            &format!("{:?}", wallet.address()),
            &format!("0x{}", signature.to_string()),
            &ballot_data_str,
            &random_bool_str,
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

fn generate_ballots(
    ballot_count: usize,
    project_ids: &Vec<String>,
) -> Result<Vec<BallotData>, Box<dyn Error>> {
    let mut ballots = Vec::new();
    let mut rng = thread_rng();
    let amount_range = Uniform::from(0..=30_000_000);
    let digit = Uniform::from(0..=9);

    for i in 0..ballot_count {
        let amount = amount_range.sample(&mut rng).to_string();

        ballots.push(BallotData {
            projectId: project_ids[i].clone(),
            amount: format!(
                "{}.{}{}",
                amount,
                digit.sample(&mut rng),
                digit.sample(&mut rng)
            ),
        });
    }

    Ok(ballots)
}
