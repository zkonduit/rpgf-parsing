// Copyright 2024 RISC Zero, Inc.
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
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[cfg(test)]
mod wasm32 {
    use bytemuck::Pod;
    use bytemuck::Zeroable;
    use risc0_zkvm::Receipt;
    use serde::{Deserialize, Serialize};
    use wasm_bindgen_futures::JsFuture;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    wasm_bindgen_test_configure!(run_in_browser);

    // Need to redfine the Felt struct found in halo2curves, applying the `Pod` and `Zeroable` traits
    #[derive(Copy, Clone, Pod, Serialize, Deserialize, Zeroable, Debug, PartialEq)]
    #[repr(C)]
    struct Felt([u64; 4]);

    // `test_verify` runs a unit test in the browser, so it can use browser APIs.
    #[wasm_bindgen_test]
    fn test_verify() {
        let result = browser_verify::wasm::verify(
            wasm_bindgen::Clamped(include_bytes!("../receipts").to_vec()),
            wasm_bindgen::Clamped(include_bytes!("../image_id").to_vec()),
        );
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(result, true);
        }
    }
    #[wasm_bindgen_test]
    fn test_extract_and_modify_hashes() {
        let result = browser_verify::wasm::extract_hashes(wasm_bindgen::Clamped(
            include_bytes!("../receipts").to_vec(),
        ));
        let hash_new = "0x7e4afb6fb51fb155eae312d64657351c68a4b5d9ce6992b303f57b35fa89d35f";
        assert!(result.is_ok());
        if let Ok(result) = result {
            let hashes_ref: Vec<String> = serde_json::from_slice(&result[..]).unwrap();
            let result = browser_verify::wasm::modify_hashes(
                wasm_bindgen::Clamped(include_bytes!("../receipts").to_vec()),
                wasm_bindgen::Clamped(serde_json::to_vec(&hashes_ref).unwrap()),
            );
            assert!(result.is_ok());
            if let Ok(result) = result {
                let receipts_modified: Vec<Receipt> = bincode::deserialize(&result[..]).unwrap();
                let felt: Felt = receipts_modified[0].journal.decode().unwrap();
                let receipts_ref: Vec<Receipt> =
                    bincode::deserialize(include_bytes!("../receipts")).unwrap();
                let felt_ref: Felt = receipts_ref[0].journal.decode().unwrap();
                assert_eq!(felt, felt_ref);
                let result_verification = browser_verify::wasm::verify(
                    wasm_bindgen::Clamped(bincode::serialize(&receipts_modified).unwrap()),
                    wasm_bindgen::Clamped(include_bytes!("../image_id").to_vec()),
                );
                assert!(result_verification.is_ok());
                if let Ok(result_verification) = result_verification {
                    assert_eq!(result_verification, true);
                }
            }
            let result = browser_verify::wasm::modify_hashes(
                wasm_bindgen::Clamped(include_bytes!("../receipts").to_vec()),
                wasm_bindgen::Clamped(serde_json::to_vec(&vec![hash_new]).unwrap()),
            );
            assert!(result.is_ok());
            if let Ok(result) = result {
                let receipts_modified: Vec<Receipt> = bincode::deserialize(&result[..]).unwrap();
                let result_verification = browser_verify::wasm::verify(
                    wasm_bindgen::Clamped(bincode::serialize(&receipts_modified).unwrap()),
                    wasm_bindgen::Clamped(include_bytes!("../image_id").to_vec()),
                );
                assert!(result_verification.is_ok());
                if let Ok(result_verification) = result_verification {
                    assert_eq!(result_verification, false);
                }
            }
        }
    }
    #[wasm_bindgen_test]
    async fn test_verify_aggr() {
        let result = browser_verify::wasm::verify_aggr(
            wasm_bindgen::Clamped(include_bytes!("../receipt_aggr").to_vec()),
            wasm_bindgen::Clamped(include_bytes!("../image_id_aggr").to_vec()),
        );
        let future = JsFuture::from(result);
        // Await the future to get the result of the JavaScript Promise
        let result = match future.await {
            Ok(value) => Ok(value),
            Err(error) => Err(error),
        };
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(result, true);
        }
    }
    #[wasm_bindgen_test]
    async fn test_extract_and_modify_hashes_aggr() {
        let result = browser_verify::wasm::extract_hashes_aggr(wasm_bindgen::Clamped(
            include_bytes!("../receipt_aggr").to_vec(),
        ));
        assert!(result.is_ok());
        if let Ok(result) = result {
            let mut hashes_ref: Vec<String> = serde_json::from_slice(&result[..]).unwrap();
            let result = browser_verify::wasm::modify_hashes_aggr(
                wasm_bindgen::Clamped(include_bytes!("../receipt_aggr").to_vec()),
                wasm_bindgen::Clamped(serde_json::to_vec(&hashes_ref).unwrap()),
            );
            assert!(result.is_ok());
            if let Ok(result) = result {
                let receipts_modified: Receipt = bincode::deserialize(&result[..]).unwrap();
                let felts: Vec<Felt> = receipts_modified.journal.decode().unwrap();
                let receipts_ref: Receipt =
                    bincode::deserialize(include_bytes!("../receipt_aggr")).unwrap();
                let felts_ref: Vec<Felt> = receipts_ref.journal.decode().unwrap();
                assert_eq!(felts, felts_ref);
                let result_verification = browser_verify::wasm::verify_aggr(
                    wasm_bindgen::Clamped(bincode::serialize(&receipts_modified).unwrap()),
                    wasm_bindgen::Clamped(include_bytes!("../image_id_aggr").to_vec()),
                );
                let future = JsFuture::from(result_verification);
                // Await the future to get the result of the JavaScript Promise
                let result_verification = match future.await {
                    Ok(value) => Ok(value),
                    Err(error) => Err(error),
                };
                assert!(result_verification.is_ok());
                if let Ok(result_verification) = result_verification {
                    assert_eq!(result_verification, true);
                }
            }
            // modify first element in hashes_ref, changing one character in the string
            hashes_ref[0] = hashes_ref[0][..hashes_ref[0].len() - 1].to_string() + "f";
            let result = browser_verify::wasm::modify_hashes_aggr(
                wasm_bindgen::Clamped(include_bytes!("../receipt_aggr").to_vec()),
                wasm_bindgen::Clamped(serde_json::to_vec(&hashes_ref).unwrap()),
            );
            assert!(result.is_ok());
            if let Ok(result) = result {
                let receipts_modified: Receipt = bincode::deserialize(&result[..]).unwrap();
                let result_verification = browser_verify::wasm::verify_aggr(
                    wasm_bindgen::Clamped(bincode::serialize(&receipts_modified).unwrap()),
                    wasm_bindgen::Clamped(include_bytes!("../image_id_aggr").to_vec()),
                );
                let future = JsFuture::from(result_verification);
                // Await the future to get the result of the JavaScript Promise
                let result_verification = match future.await {
                    Ok(value) => Ok(value),
                    Err(error) => Err(error),
                };
                assert!(result_verification.is_err());
            }
        }
    }
}
