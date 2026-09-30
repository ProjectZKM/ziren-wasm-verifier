//! A simple wrapper around the `zkm_verifier` crate.

use std::borrow::Cow;
use std::io::Read;

use wasm_bindgen::prelude::wasm_bindgen;
use zkm_verifier::{
    Groth16Verifier, PlonkVerifier, StarkVerifier, GROTH16_VK_BYTES, PLONK_VK_BYTES,
};

/// zstd frame magic (little-endian 0xFD2FB528).
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

/// Upper bound on a decompressed proof; a compressed V2.0 proof is about 1.1 MB.
const MAX_DECODED_LEN: u64 = 64 << 20;

/// Unwraps the zstd frame a STARK proof may be published in.
///
/// Provers may publish `bincode::serialize(&proof)` compressed as a single zstd frame, which
/// takes it to 68-80% of its size. A frame is recognised by its magic and decoded; any other
/// input is returned untouched, so uncompressed proofs verify as before. A frame that fails to
/// decode, or decodes to more than [`MAX_DECODED_LEN`] bytes, is returned untouched as well and
/// the verifier rejects it, so the framing can never turn an invalid proof into a valid one.
fn decode(proof: &[u8]) -> Cow<'_, [u8]> {
    if proof.len() < 4 || proof[..4] != ZSTD_MAGIC {
        return Cow::Borrowed(proof);
    }
    let mut src = proof;
    let Ok(dec) = ruzstd::StreamingDecoder::new(&mut src) else {
        return Cow::Borrowed(proof);
    };
    let mut out = Vec::new();
    match dec.take(MAX_DECODED_LEN + 1).read_to_end(&mut out) {
        Ok(n) if (n as u64) <= MAX_DECODED_LEN => Cow::Owned(out),
        _ => Cow::Borrowed(proof),
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
