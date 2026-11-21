use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct VectorFile {
    schema_version: u32,
    unicode_version: String,
    contract: String,
    status: String,
    vectors: Vec<serde_json::Value>,
}

#[test]
fn d_contract_vectors_are_loaded_and_empty_set_fails_closed() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/vectors/d-normalization-vectors.json");
    let bytes = std::fs::read(path).expect("D vector file must be readable");
    let file: VectorFile = serde_json::from_slice(&bytes).expect("D vector JSON must parse");
    assert_eq!(file.schema_version, 1);
    assert_eq!(file.unicode_version, "15.1.0");
    assert_eq!(file.contract, "D-contract-1.3.1");
    assert!(
        file.vectors.is_empty(),
        "D vectors are not supplied; do not substitute UCD corpus"
    );
    assert_eq!(file.status, "NOT_RUN_NOT_EVIDENCED");
}
