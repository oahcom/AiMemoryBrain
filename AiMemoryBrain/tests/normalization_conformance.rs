//! Full Unicode 15.1.0 normalization conformance suite.
//!
//! The test must consume the entire locked NormalizationTest.txt file.
//! It must fail if the file is missing, truncated, malformed, has an
//! unexpected hash, or any required assertion fails.

#[test]
#[ignore = "Step 2 implementation pending"]
fn full_unicode_15_1_0_normalization_conformance() {
    todo!("Step 2: load locked corpus and run all NFC/NFD assertions")
}
