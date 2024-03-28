use risc0_zkvm::get_prover_server;
use risc0_zkvm::recursion::identity_p254;
use risc0_zkvm::seal_to_json;
use risc0_zkvm::CompactReceipt;
use risc0_zkvm::ExecutorEnv;
use risc0_zkvm::ExecutorImpl;
use risc0_zkvm::Groth16ProofJson;
use risc0_zkvm::Groth16Seal;
use risc0_zkvm::InnerReceipt;
use risc0_zkvm::ProverOpts;
use risc0_zkvm::Receipt;
use risc0_zkvm::VerifierContext;

pub fn stark_to_groth16(env: ExecutorEnv, image_id: &[u32; 8], elf: &[u8]) -> Receipt {
    let mut exec = ExecutorImpl::from_elf(env, elf).unwrap();
    let session = exec.run().unwrap();
    let opts = ProverOpts::default();
    let ctx = VerifierContext::default();
    let prover = get_prover_server(&opts).unwrap();
    let time = std::time::Instant::now();
    let receipt = prover.prove_session(&ctx, &session).unwrap();
    let proving_time = time.elapsed();
    println!("Proving time: {:?}", proving_time);
    let claim = receipt.get_claim().unwrap();
    let composite_receipt = receipt.inner.composite().unwrap();
    let succinct_receipt = prover.compress(composite_receipt).unwrap();
    let journal = session.journal.unwrap().bytes;
    let ident_receipt = identity_p254(&succinct_receipt).unwrap();
    let seal_bytes = ident_receipt.get_seal_bytes();
    let seal = stark_to_snark(&seal_bytes).unwrap().to_vec();
    let receipt = Receipt::new(
        InnerReceipt::Compact(CompactReceipt { seal, claim }),
        journal,
    );
    receipt.verify(*image_id).unwrap();
    receipt
}

use std::{
    fs::File,
    io::{Cursor, Read},
    path::Path,
    process::Command,
};

use anyhow::{bail, Result};
use tempfile::tempdir;

/// Compact a given seal of an `identity_p254` receipt into a Groth16 `Seal`.
/// Requires running Docker on an x86 architecture.
fn stark_to_snark(identity_p254_seal_bytes: &[u8]) -> Result<Groth16Seal> {
    if !is_docker_installed() {
        bail!("Please install docker first.")
    }

    let tmp_dir = tempdir()?;
    let work_dir = std::env::var("RISC0_WORK_DIR");
    let work_dir = work_dir
        .as_ref()
        .map(|x| Path::new(x))
        .unwrap_or(tmp_dir.path());

    std::fs::write(work_dir.join("seal.r0"), &identity_p254_seal_bytes)?;
    let seal_path = work_dir.join("input.json");
    let proof_path = work_dir.join("proof.json");
    let seal_json = File::create(&seal_path)?;
    let mut seal_reader = Cursor::new(&identity_p254_seal_bytes);
    seal_to_json(&mut seal_reader, &seal_json)?;

    let status = Command::new("docker")
        .arg("run")
        .arg("--rm")
        .arg("-v")
        .arg(&format!("{}:/mnt", work_dir.to_string_lossy()))
        .arg("risc0-groth16-prover")
        .status()?;
    if !status.success() {
        panic!("docker returned failure exit code: {:?}", status.code());
    }
    let mut proof_file = File::open(proof_path)?;
    let mut contents = String::new();
    proof_file.read_to_string(&mut contents)?;
    let proof_json: Groth16ProofJson = serde_json::from_str(&contents)?;
    proof_json.try_into()
}

fn is_docker_installed() -> bool {
    Command::new("docker")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
