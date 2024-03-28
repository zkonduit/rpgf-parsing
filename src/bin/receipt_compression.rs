use clap::{App, Arg};
use risc0_zkvm::get_prover_server;
use risc0_zkvm::recursion::identity_p254;
use risc0_zkvm::stark_to_snark;
use risc0_zkvm::CompactReceipt;
use risc0_zkvm::InnerReceipt;
use risc0_zkvm::ProverOpts;
use risc0_zkvm::Receipt;
use std::fs;
use tokio;

#[tokio::main]
async fn main() {
    let matches = App::new("receipt_compression")
        .version("1.0")
        .author("Ethan Cemer")
        .about("")
        .arg(
            Arg::with_name("tags")
                .short('T')
                .long("tags")
                .takes_value(true)
                .help("Specifies the tag to download the receipt and image id from"),
        )
        .arg(
            Arg::with_name("receipt")
                .short('R')
                .long("receipt")
                .takes_value(true)
                .help("Specifies the file path to write the receipt to"),
        )
        .get_matches();
    let tags = matches.value_of("tags").unwrap_or_default();
    let tags: Vec<&str> = tags.split(',').collect();
    for (i, tag) in tags.iter().enumerate() {
        println!("Processing tag {}/{}: {}", i + 1, tags.len(), tag);
        let default_receipt_path = format!("receipt_{}", tag);
        let receipt_path = matches.value_of("receipt").unwrap_or(&default_receipt_path);
        let receipt_link = format!("https://orpgf-3.s3.us-east-va.perf.cloud.ovh.us/orpgf-public/browser-verify-{}/receipt_aggr", tag);
        let image_id_link = format!("https://orpgf-3.s3.us-east-va.perf.cloud.ovh.us/orpgf-public/browser-verify-{}/image_id_aggr", tag);

        // Download the files
        let receipt_buffer = download_content_as_bytes(&receipt_link).await.unwrap();
        let image_id_buffer = download_content_as_bytes(&image_id_link).await.unwrap();
        let receipt: Receipt = bincode::deserialize(&receipt_buffer).unwrap();
        let image_id: [u32; 8] = bincode::deserialize(&image_id_buffer).unwrap();
        let opts = ProverOpts::default();
        let prover = get_prover_server(&opts).unwrap();
        let claim = receipt.get_claim().unwrap();
        let composite_receipt = receipt.inner.composite().unwrap();
        let succinct_receipt = prover.compress(composite_receipt).unwrap();
        let journal = receipt.journal.bytes;
        let ident_receipt = identity_p254(&succinct_receipt).unwrap();
        let seal_bytes = ident_receipt.get_seal_bytes();
        let seal = stark_to_snark(&seal_bytes).unwrap().to_vec();
        let receipt = Receipt::new(
            InnerReceipt::Compact(CompactReceipt { seal, claim }),
            journal,
        );
        receipt.verify(image_id).unwrap();
        // write the receipt to the fileii
        let receipt_ser = bincode::serialize(&receipt).unwrap();
        fs::write(receipt_path, receipt_ser).unwrap();
    }
}

async fn download_content_as_bytes(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;

    if response.status().is_success() {
        let bytes = response.bytes().await?.to_vec();
        Ok(bytes)
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Failed to download content from {}: {}",
                url,
                response.status()
            ),
        )))
    }
}
