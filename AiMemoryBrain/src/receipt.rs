//! Receipt evidence model.
//!
//! A fixture may test parsing and predicate code. It must never be represented
//! as a real D-authoritative Receipt sample.

#[derive(Debug, Clone)]
pub struct ReceiptSample {
    pub receipt_id: String,
    pub source_system: String,
    pub source_sequence: u64,
    pub observed_high_water: u64,
    pub linearization_reference: String,
    pub provenance_reference: String,
    pub is_fixture: bool,
}

/// Validate that the supplied Receipt sample has required provenance.
///
/// Operational proof must reject `is_fixture == true`.
pub fn validate_operational_receipt(
    receipt: &ReceiptSample,
) -> Result<(), ReceiptError> {
    todo!("Step 2: implement Receipt provenance and fixture rejection")
}

#[derive(Debug)]
pub enum ReceiptError {
    MissingField(String),
    FixtureNotOperational,
    InvalidProvenance(String),
}
