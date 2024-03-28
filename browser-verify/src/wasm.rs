use bytemuck::Pod;
use bytemuck::Zeroable;
use js_sys::Promise;
use risc0_zkvm::serde::to_vec;
use risc0_zkvm::sha::Digest;
use risc0_zkvm::{Journal, Receipt};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
// import felt from halo2
use halo2curves::bn256::Fr as Fp;
use halo2curves::ff::PrimeField;

#[derive(Copy, Clone, Pod, Serialize, Deserialize, Zeroable, Debug, PartialEq)]
#[repr(C)]
struct Felt([u64; 4]);

fn to_hex_string(values: &Felt) -> String {
    // Map each u64 value to its hexadecimal string representation and concatenate them.
    let bytes: [u8; 32] = values
        .0
        .iter()
        .flat_map(|&value| value.to_le_bytes().to_vec())
        .collect::<Vec<u8>>()
        .try_into()
        .unwrap();

    let hex_str = hex::encode(bytes);

    // Prepend the '0x' prefix to the concatenated string.
    format!("0x{}", hex_str)
}

fn from_hex_string(hex_str: &str) -> Felt {
    // Remove the '0x' prefix from the input string.
    let hex_str = hex_str.trim_start_matches("0x");

    // Split the input string into 16-character chunks, then hex decode each chunk, convert them to u64 from le bytes and collect them into an array.
    let chunks: [u64; 4] = hex_str
        .as_bytes()
        .chunks(16)
        .map(|chunk| u64::from_le_bytes(hex::decode(chunk).unwrap().try_into().unwrap()))
        .collect::<Vec<u64>>()
        .try_into()
        .unwrap();

    // Parse each chunk as a u64 value and collect them into an array.
    Felt(chunks)
}

#[wasm_bindgen]
pub fn big_endian_to_little_endian(hex_str: &str) -> String {
    // Remove the '0x' prefix from the input string.
    let hex_str = hex_str.trim_start_matches("0x");

    let bytes = hex::decode(hex_str).unwrap();

    let bytes_slices: [u8; 32] = bytes[..32].try_into().unwrap();

    let felt = Fp::from_repr(bytes_slices).unwrap();

    format!("{:?}", felt)
}

// Batch verifies receipts generates from single hash guest code (project.rs)
#[wasm_bindgen]
pub fn verify(
    receipts: wasm_bindgen::Clamped<Vec<u8>>,
    image_id: wasm_bindgen::Clamped<Vec<u8>>,
) -> Result<bool, JsError> {
    let receipts: Vec<Receipt> = bincode::deserialize(&receipts[..]).unwrap();
    let image_id: Digest = bincode::deserialize(&image_id[..]).unwrap();
    let result = receipts
        .iter()
        .map(|receipt| receipt.verify(image_id.clone()))
        .collect::<Result<Vec<()>, _>>();
    match result {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

// Extracts the hashes from receipts generated from single hash guest code (project.rs)
#[wasm_bindgen]
pub fn extract_hashes(
    receipts: wasm_bindgen::Clamped<Vec<u8>>,
) -> Result<wasm_bindgen::Clamped<Vec<u8>>, JsError> {
    let receipts: Vec<Receipt> = bincode::deserialize(&receipts[..]).unwrap();
    let hashes = receipts
        .iter()
        .map(|receipt| receipt.journal.decode::<Felt>().unwrap())
        .collect::<Vec<Felt>>();

    // get the string hashes to get Vec<String>
    let hash_strings = hashes
        .iter()
        .map(|felt| to_hex_string(felt))
        .collect::<Vec<String>>();

    Ok(wasm_bindgen::Clamped(
        serde_json::to_vec(&hash_strings).unwrap(),
    ))
}

// Modifies the hashes in the receipts generated from single hash guest code (project.rs)
#[wasm_bindgen]
pub fn modify_hashes(
    receipts: wasm_bindgen::Clamped<Vec<u8>>,
    hashes_string: wasm_bindgen::Clamped<Vec<u8>>,
) -> Result<wasm_bindgen::Clamped<Vec<u8>>, JsError> {
    // receipts vector length doesn't match the hashes vector length
    // return an error
    let mut receipts: Vec<Receipt> = bincode::deserialize(&receipts[..]).unwrap();
    let hashes_string: Vec<String> = serde_json::from_slice(&hashes_string[..]).unwrap();
    if receipts.len() != hashes_string.len() {
        return Err(JsError::new("Receipts and hashes length mismatch"));
    }
    let hashes: Vec<Felt> = hashes_string
        .iter()
        .map(|felt| from_hex_string(felt))
        .collect::<Vec<Felt>>();
    // modify the hashes in the receipts
    for (receipt, felt) in receipts.iter_mut().zip(hashes.iter()) {
        let data = bytemuck::bytes_of(felt);
        let journal = Journal::new(data.to_vec());
        receipt.journal = journal;
    }

    Ok(wasm_bindgen::Clamped(
        bincode::serialize(&receipts).unwrap(),
    ))
}

#[wasm_bindgen]
pub async fn verify_aggr_async(
    receipt: wasm_bindgen::Clamped<Vec<u8>>,
    image_id: wasm_bindgen::Clamped<Vec<u8>>,
) -> Result<JsValue, JsValue> {
    // Note the change in the error type to JsValue.
    let receipt_result = async move {
        let receipt: Receipt = bincode::deserialize(&receipt[..]).unwrap();
        let image_id: Digest = bincode::deserialize(&image_id[..]).unwrap();
        match receipt.verify(image_id) {
            Ok(_) => Ok(JsValue::from_bool(true)),
            Err(_) => Err(JsValue::from_str("Verification failed")),
        }
    };

    receipt_result.await
}

// Wrapper function to convert Rust Future into JavaScript Promise
#[wasm_bindgen]
pub fn verify_aggr(
    receipt: wasm_bindgen::Clamped<Vec<u8>>,
    image_id: wasm_bindgen::Clamped<Vec<u8>>,
) -> Promise {
    future_to_promise(verify_aggr_async(receipt, image_id))
}

// Extracts the hashes from the receipt generated from the aggregate guest code (projects.rs)
#[wasm_bindgen]
pub fn extract_hashes_aggr(
    receipt: wasm_bindgen::Clamped<Vec<u8>>,
) -> Result<wasm_bindgen::Clamped<Vec<u8>>, JsError> {
    let receipt: Receipt = bincode::deserialize(&receipt[..]).unwrap();
    let hashes = receipt.journal.decode::<Vec<Felt>>().unwrap();
    // get the string hashes to get Vec<String>
    let hash_strings = hashes
        .iter()
        .map(|felt| to_hex_string(felt))
        .collect::<Vec<String>>();

    Ok(wasm_bindgen::Clamped(
        serde_json::to_vec(&hash_strings).unwrap(),
    ))
}

// Modifies the hashes in the receipt generated from the aggregate guest code (projects.rs)
#[wasm_bindgen]
pub fn modify_hashes_aggr(
    receipt: wasm_bindgen::Clamped<Vec<u8>>,
    hashes_string: wasm_bindgen::Clamped<Vec<u8>>,
) -> Result<wasm_bindgen::Clamped<Vec<u8>>, JsError> {
    let mut receipt: Receipt = bincode::deserialize(&receipt[..]).unwrap();
    let hashes_string: Vec<String> = serde_json::from_slice(&hashes_string[..]).unwrap();
    let hashes: Vec<Felt> = hashes_string
        .iter()
        .map(|felt| from_hex_string(felt))
        .collect::<Vec<Felt>>();
    let data_u32_words = to_vec(&hashes).unwrap();
    let bytes = bytemuck::try_cast_slice(data_u32_words.as_slice()).unwrap();
    // modify the hashes in the receipt
    let journal = Journal::new(bytes.to_vec());
    receipt.journal = journal;

    Ok(wasm_bindgen::Clamped(bincode::serialize(&receipt).unwrap()))
}
