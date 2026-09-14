//! A simple wrapper around the `zkm_verifier` crate.

use std::borrow::Cow;
use std::io::Read;

use wasm_bindgen::prelude::wasm_bindgen;
use zkm_verifier::{
    Groth16Verifier, PlonkVerifier, StarkVerifier, GROTH16_VK_BYTES, PLONK_VK_BYTES,
};

/// zstd frame magic (little-endian 0xFD2FB528).
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

/// Unwrap the published proof's transport framing.
///
/// The artifact eth-proofs stores is `bincode::serialize(&proof.proof)`, and
/// the only thing that decodes it is this verifier -- so the wire format is
/// ours to choose.  MEASURED on three real proofs (617,622 B production,
/// 617,656 B fibonacci, 1,109,739 B as published to eth-proofs): zstd takes it
/// to 68-80% of the bincode bytes, with no change to the proof system, no new
/// verifying key and no ceremony.  Level 3 lands within 0.5% of level 19, so
/// the prover side costs nothing worth measuring and this side is one pass.
///
/// The redundancy is framing and Merkle path nodes duplicated across
/// overlapping queries, not the field elements -- a byte-shuffle filter makes
/// it worse, and 15.2% of the file is zero bytes.  So this is not worth
/// hand-rolling as a bespoke codec.
///
/// A frame is recognised by its magic and unwrapped; anything else is returned
/// untouched, so every proof published before this change still verifies.  On
/// any decode error the original bytes are passed through and the verifier
/// rejects them: framing must never be able to turn a bad proof good.
fn decode(proof: &[u8]) -> Cow<'_, [u8]> {
    if proof.len() < 4 || proof[..4] != ZSTD_MAGIC {
        return Cow::Borrowed(proof);
    }
    let mut src = proof;
    let Ok(mut dec) = ruzstd::StreamingDecoder::new(&mut src) else {
        return Cow::Borrowed(proof);
    };
    let mut out = Vec::new();
    match dec.read_to_end(&mut out) {
        Ok(_) => Cow::Owned(out),
        Err(_) => Cow::Borrowed(proof),
    }
}

/// Wrapper around [`zkm_verifier::StarkVerifier::verify`].
///
/// # Arguments
///
/// * `proof` - The proof bytes.
/// * `public_inputs` - The Ziren public inputs, which are committed by the guest as a bincode-serialized byte array.
///     For example:
///     ```
///     // Write the output of the program.
///     //
///     // Behind the scenes, this also compiles down to a system call which handles writing
///     // outputs to the prover.
///     // zkm_zkvm::io::commit(&block_hash);
///     ```
/// * `zkm_vk` - The Ziren vkey bytes.
///   This is generated in the following manner:
///     ```ignore
///     use zkm_sdk::ProverClient;
///     let client = ProverClient::new();
///     let (pk, vk) = client.setup(ELF);
///     ```
///
/// # Returns
///
/// Returns true if verification succeeds, or false if verification fails.
///
/// Compared to `verify_stark_proof()`, it performs a consistency check between
/// user-supplied public values and those committed in the proof.
#[wasm_bindgen]
pub fn verify_stark(proof: &[u8], public_inputs: &[u8], zkm_vk: &[u8]) -> bool {
    StarkVerifier::verify(&decode(proof), public_inputs, zkm_vk).is_ok()
}

/// Wrapper around [`zkm_verifier::StarkVerifier::verify`].
#[wasm_bindgen]
pub fn verify_stark_proof(proof: &[u8], zkm_vk: &[u8]) -> bool {
    StarkVerifier::verify_proof(&decode(proof), zkm_vk).is_ok()
}

/// Wrapper around [`zkm_verifier::Groth16Verifier::verify`].
///
/// We hardcode the Groth16 VK bytes to only verify Ziren proofs.
#[wasm_bindgen]
pub fn verify_groth16(proof: &[u8], public_inputs: &[u8], zkm_vk_hash: &str) -> bool {
    Groth16Verifier::verify(proof, public_inputs, zkm_vk_hash, *GROTH16_VK_BYTES).is_ok()
}

/// Wrapper around [`zkm_verifier::PlonkVerifier::verify`].
///
/// We hardcode the Plonk VK bytes to only verify Ziren proofs.
#[wasm_bindgen]
pub fn verify_plonk(proof: &[u8], public_inputs: &[u8], zkm_vk_hash: &str) -> bool {
    PlonkVerifier::verify(proof, public_inputs, zkm_vk_hash, *PLONK_VK_BYTES).is_ok()
}
