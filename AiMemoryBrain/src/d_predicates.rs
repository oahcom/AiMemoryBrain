//! D dual-high-water proof gate for F-43-01/F1.
//!
//! D-contract-1.3.1 §5 requires six conjunctive conditions. The existing
//! snapshot/receipt API cannot carry all required typed bindings, so this
//! module refuses to synthesize a successful report.

use crate::authority::DAuthoritativeSource;
use crate::receipt::{ReceiptSample, validate_operational_receipt};

#[derive(Debug, Clone)]
pub struct DHighWaterSnapshot {
    pub authoritative_high_water: u64,
    pub observed_high_water: u64,
    pub source_sequence: u64,
    pub linearization_token: String,
    pub provenance: String,
}

#[derive(Debug, Clone)]
pub struct PredicateResult {
    pub condition_id: &'static str,
    pub passed: bool,
    pub evidence_reference: String,
}

#[derive(Debug, Clone)]
pub struct SixConditionReport {
    pub c1: PredicateResult,
    pub c2: PredicateResult,
    pub c3: PredicateResult,
    pub c4: PredicateResult,
    pub c5: PredicateResult,
    pub c6: PredicateResult,
}

pub fn evaluate_six_conditions(
    source: &dyn DAuthoritativeSource,
    receipt: &ReceiptSample,
) -> Result<SixConditionReport, DProofError> {
    validate_operational_receipt(receipt).map_err(|error| match error {
        crate::receipt::ReceiptError::MissingProvenance => DProofError::MissingProvenance,
        other => DProofError::PredicateFailed(format!("invalid receipt: {other:?}")),
    })?;
    require_operational_source(source)?;
    Err(DProofError::ContractMappingIncomplete(
        "§5 identity, operation, fence, immutable snapshot, ledger consistency, and serialization evidence are absent from the current API".to_owned(),
    ))
}

/// The canonical D endpoint, trust root, transport, and operational key
/// registry are unspecified. A public trait implementation cannot self-attest.
pub fn require_operational_source(_source: &dyn DAuthoritativeSource) -> Result<(), DProofError> {
    Err(DProofError::SourceUnavailable(
        "canonical D-authoritative endpoint and trust configuration are not specified".to_owned(),
    ))
}

pub fn require_evidence_gate(
    source: Option<&dyn DAuthoritativeSource>,
    receipt: Option<&ReceiptSample>,
) -> Result<(), DProofError> {
    let source = source.ok_or_else(|| {
        DProofError::SourceUnavailable("D-authoritative source is not configured".to_owned())
    })?;
    let receipt = receipt.ok_or(DProofError::MissingReceipt)?;
    validate_operational_receipt(receipt).map_err(|error| match error {
        crate::receipt::ReceiptError::MissingProvenance => DProofError::MissingProvenance,
        other => DProofError::PredicateFailed(format!("invalid receipt: {other:?}")),
    })?;
    require_operational_source(source)
}

#[derive(Debug)]
pub enum DProofError {
    SourceUnavailable(String),
    TestDoubleOnly,
    MissingReceipt,
    MissingProvenance,
    PredicateFailed(String),
    MissingScenarioEvidence(String),
    ContractMappingIncomplete(String),
}
