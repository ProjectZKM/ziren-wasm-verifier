//! A simple host to generate proofs for the fibonacci guest, and serialize them to JSON.

use clap::Parser;
use serde::{Deserialize, Serialize};
use zkm_sdk::{include_elf, utils, HashableKey, ProverClient, ZKMProofWithPublicValues, ZKMStdin};
use zkm_verifier::{
    Groth16Verifier, PlonkVerifier, StarkVerifier, GROTH16_VK_BYTES, PLONK_VK_BYTES,
};

/// The ELF (executable and linkable format) file for the fibonacci guest.
pub const FIBONACCI_ELF: &[u8] = include_elf!("fibonacci-guest");

#[derive(Serialize, Deserialize)]
struct ProofData {
    proof: String,         // hex string
    public_inputs: String, // hex string
    vkey_hash: String,     // vk.bytes32()
    vkey: String,
    zkm_version: String,
    mode: String,
}

#[derive(clap::Parser)]
#[command(name = "zkVM Proof Generator")]
struct Cli {
    #[arg(
        long,
        value_name = "prove",
        default_value_t = false,
        help = "Whether to generate a proof or use the pregenerated proof"
    )]
    prove: bool,

    #[arg(
        long,
        value_name = "mode",
        default_value = "plonk",
        help = "Specifies the proof mode to use (e.g., stark, groth16, plonk)"
    )]
    mode: String,
}

fn main() {
    // Setup logging for the application
    utils::setup_logger();

    // Parse command line arguments
    let args = Cli::parse();
    let mut stdin = ZKMStdin::new();
    stdin.write(&1000u32);

    // Initialize the prover client.
    let client = ProverClient::new();
    let (pk, vk) = client.setup(FIBONACCI_ELF);

    // These are the output paths.
    let proof_path = format!("../binaries/fibonacci_{}_proof.bin", args.mode);
    let json_path = format!("../json/fibonacci_{}_proof.json", args.mode);

    if args.prove {
        // Generate a proof for the specified program
        let proof = match args.mode.as_str() {
            "stark" => {
                let proof = client
                    .prove(&pk, stdin)
                    .compressed()
                    .run()
                    .expect("Stark proof generation failed");

                StarkVerifier::verify(
                    proof.bytes().as_ref(),
                    proof.public_values.as_ref(),
                    bincode::serialize(&vk).unwrap().as_ref(),
                )
                .expect("Stark proof verification failed");

                proof
            }
            "groth16" => {
                let proof = client
                    .prove(&pk, stdin)
                    .groth16()
                    .run()
                    .expect("Groth16 proof generation failed");

                Groth16Verifier::verify(
                    proof.bytes().as_ref(),
                    proof.public_values.as_ref(),
                    vk.bytes32().as_ref(),
                    *GROTH16_VK_BYTES,
                )
                .expect("Groth16 proof verification failed");

                proof
            }
            "plonk" => {
                let proof = client
                    .prove(&pk, stdin)
                    .plonk()
                    .run()
                    .expect("Plonk proof generation failed");

                PlonkVerifier::verify(
                    proof.bytes().as_ref(),
                    proof.public_values.as_ref(),
                    vk.bytes32().as_ref(),
                    *PLONK_VK_BYTES,
                )
                .expect("Plonk proof verification failed");

                proof
            }
            _ => panic!("Invalid proof mode. Use 'groth16' or 'plonk'."),
        };

        proof.save(&proof_path).expect("Failed to save proof");
    }

    let vkey = if args.mode.as_str() == "stark" {
        let vk_bytes = bincode::serialize(&vk).unwrap();
        hex::encode(vk_bytes)
    } else {
        "0x".to_owned()
    };

    // Load the proof, extract the proof and public inputs, and serialize the appropriate fields.
    let proof = ZKMProofWithPublicValues::load(&proof_path).expect("Failed to load proof");
    let fixture = ProofData {
        proof: hex::encode(proof.bytes()),
        public_inputs: hex::encode(proof.public_values),
        vkey_hash: vk.bytes32(),
        vkey,
        zkm_version: proof.zkm_version,
        mode: args.mode,
    };

    // Serialize the proof data to a JSON file.
    let json_proof = serde_json::to_string(&fixture).expect("Failed to serialize proof");
    std::fs::write(json_path, json_proof).expect("Failed to write JSON proof");

    println!("Successfully generated json proof for the program!")
}
