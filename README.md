# Ziren Wasm verification example

This repo demonstrates how to verify Groth16 and Plonk proofs in browser. We wrap the [`ziren-verifier`](https://github.com/ProjectZKM/Ziren.git) crate in wasm bindings, and invoke it from javascript.

## Repo overview

- `verifier`: The rust Ziren verifier crate with wasm bindings.
- `example/eth_wasm`: A short javascript example that verifies an ETH proof in wasm.
- `example/guest`: A simple fibonacci Ziren guest to verify.
- `example/host`: A simple host to generate proofs in a json format.
- `example/wasm_example`: A short javascript example that verifies proofs in wasm.

## Ziren version and toolchains

This branch tracks Ziren `feat/upgrade-plonky3` (workspace version `2.0.0`, the
Plonky3-based jagged-over-WHIR stack, KoalaBear).  The pinned revision is in the workspace
`Cargo.toml` (`rev = ...`); bump it there and run `cargo update -p zkm-verifier -p zkm-sdk
-p zkm-zkvm -p zkm-build`.

- The host and the wasm crate need the nightly pinned in `rust-toolchain.toml` (the same one
  Ziren uses); `wasm32-unknown-unknown` must be added to it
  (`rustup target add wasm32-unknown-unknown`).
- The guest is built by `zkm-build` from the host's build script with the Ziren toolchain
  (`zkmup`), which must be first on `PATH` (`source ~/.zkm-toolchain/env`).  Building the guest
  with a plain nightly produces an ELF the executor rejects.
- `zkm-recursion-gnark-ffi` (pulled in by `zkm-sdk`) needs Go 1.23+ on `PATH` to build the host.

### What the verifier accepts

`zkm-verifier` only accepts a compressed STARK proof whose recursion verifying key is in the
allowed key map compiled into the crate (`crates/prover/vk_map.bin` in Ziren).  In the
`2.0.0` stack that map holds the enumerated compose/shrink keys plus the leaf keys collected
for the production programs (the reth block guest), so:

- ETH block proofs from the Ziren prover network verify (see `example/eth_wasm`);
- a proof of an arbitrary guest such as `example/guest` is rejected with
  `Recursion(Invalid verification key)` until its keys are added to the map.  The fibonacci
  fixture in `example/json` is kept for exercising the bindings; `example/wasm_example`
  prints the verifier's answer for it instead of asserting.
- Groth16 / Plonk proofs go through the same map inside the wrap circuit, so they exist only
  for programs the map covers.  The Groth16 key in `zkm-verifier` is the August 2026 ceremony
  key for the `v2.0.0` wrap circuit; there is no `v2.0.0` Plonk artifact set yet.

## Usage

### Wasm Bindings

First, generate the wasm library for the verifier. From the `verifier` directory, run

```bash
wasm-pack build --target nodejs --dev 
```

This will generate wasm bindings for the rust functions in [`verifier/src/lib.rs`](verifier/src/lib.rs).
> [!Note]
> Generating wasm bindings in dev mode will result in drastically slower verification times.
> Generate bindings in release mode by replacing `--dev` with `--release`.

As an example, the following snippet provides wasm bindings for the `verify_groth16` function:

```rust,noplayground
#[wasm_bindgen]
pub fn verify_groth16(proof: &[u8], public_inputs: &[u8], zkm_vk_hash: &str) -> bool {
    Groth16Verifier::verify(proof, public_inputs, zkm_vk_hash, *GROTH16_VK_BYTES).is_ok()
}
```

### Generate proofs

Next, run the host to generate `fibonacci_groth16_proof.json` and `fibonacci_plonk_proof.json`. From the `example/host` directory, run:

```bash
cargo run --release -- --mode stark
cargo run --release -- --mode groth16
cargo run --release -- --mode plonk
```

By default, this will *not* generate fresh proofs from the program in `example/guest`. To generate fresh proofs, run:

```bash
cargo run --release -- --mode stark --prove
cargo run --release -- --mode groth16 --prove
cargo run --release -- --mode plonk --prove
```

Here, stark, groth16 and plonk proofs are generated using `client.prove(&pk, stdin).compressed().run()`, `client.prove(&pk, stdin).groth16().run()` and `client.prove(&pk, stdin).plonk().run()`, respectively.
See the [Ziren docs](https://docs.zkm.io/dev/prover.html#proof-types) for more details.

From a [`ZKMProofWithPublicValues`](https://github.com/ProjectZKM/Ziren/blob/main/crates/sdk/src/proof.rs#L37),
we extract the proof and public inputs, and serialize the appropriate fields. See the following snippet for details:

```rust,noplayground
// Load the proof and extract the proof and public inputs.
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
```

### Verify fibonacci proofs in wasm

To verify proofs in wasm, run the following command from the `example/wasm_example` directory:

```bash
pnpm install
pnpm run test
```

This runs [`main.js`](example/wasm_example/main.js), which verifies the proofs in `example/json`.
The proofs are decoded from hex strings and verified using the wasm bindings. In addition, the public inputs
are deserialized into 32-bit integers and printed. See the following snippet for details:

```javascript
// Read and parse the JSON content of the file
const fileContent = fs.readFileSync(path.join("../json", file), 'utf8');
const proof_json = JSON.parse(fileContent);

// Determine the ZKP type (Groth16 or Plonk) based on the filename
const zkpType = file_name.includes('groth16') ? 'groth16' : file_name.includes('plonk')? 'plonk' : 'stark';
const proof = fromHexString(proof_json.proof);
const public_inputs = fromHexString(proof_json.public_inputs);
const vkey_hash = proof_json.vkey_hash;

// Get the values using DataView.
const view = new DataView(public_inputs.buffer);

// Read each 32-bit (4 byte) integer as little-endian
const n = view.getUint32(0, true);
const a = view.getUint32(4, true);
const b = view.getUint32(8, true);

console.log(`n: ${n}`);
console.log(`a: ${a}`);
console.log(`b: ${b}`);

if (zkpType == 'stark') {
    const vkey = fromHexString(proof_json.vkey);

    const startTime = performance.now();
    const result = wasm.verify_stark(proof, public_inputs, vkey);
    const endTime = performance.now();
    console.log(`${zkpType} verification took ${endTime - startTime}ms`);
    assert(result);
    console.log(`Proof in ${file} is valid.`);
} else {
    // Select the appropriate verification function and verification key based on ZKP type
    const verifyFunction = zkpType === 'groth16' ? wasm.verify_groth16 : wasm.verify_plonk;

    const startTime = performance.now();
    const result = verifyFunction(proof, public_inputs, vkey_hash);
    const endTime = performance.now();
    console.log(`${zkpType} verification took ${endTime - startTime}ms`);
    assert(result);
    console.log(`Proof in ${file} is valid.`);
}
```

### Verify ETH proofs in wasm

To verify an ETH proof in wasm, run the following command from the `example/eth_wasm` directory:

```bash
pnpm install
pnpm run test
```

This runs [`main.js`](example/eth_wasm/main.js), which verifies an ETH proof in `example/binaries`.
The proof is downloaded from https://ethproofs.org (`GET /api/v0/proofs/download/<proof_id>`). The vk is
`bincode(vk)` of the reth block guest the cluster runs (`client.setup(ELF)`), stored as `eth_vk.bin`.
See the following snippet for details:

```javascript
import * as wasm from "../../verifier/pkg/zkm_wasm_verifier.js"
import fs from 'node:fs'

const vkey = fs.readFileSync('../binaries/eth_vk.bin');

// Block 25921700, proved by the Ziren cluster on https://ethproofs.org
// (cluster 84a01f4b-8078-44cf-b463-90ddcd124960, proof 22393276, 617,622 bytes).
const proof = fs.readFileSync('../binaries/zkm_84a01f4b-8078-44cf-b463-90ddcd124960_25921700.bin');

const startTime = performance.now();
const result = wasm.verify_stark_proof(proof, vkey);
const endTime = performance.now();

console.log(`stark verification took ${endTime - startTime}ms`);
console.assert(result, "result:", result, "proof should be valid");
console.log(`ETH proof is valid.`);
```

The wasm STARK verifier is used by [@ethproofs/ziren-wasm-stark-verifier](https://www.npmjs.com/package/@ethproofs/ziren-wasm-stark-verifier)
