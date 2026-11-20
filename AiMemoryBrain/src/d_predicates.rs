//! D dual-high-water predicates for F-43-01/F1.
//!
//! The exact six normative predicates must be mapped clause-by-clause from
//! D-contract-1.3.1. Do not rename, weaken, combine, or infer their semantics.
//! Step 1 deliberately reserves C1..C6 until that exact clause mapping is
//! transcribed and independently checked.

use crate::authority::DAuthoritativeSource;
use crate::receipt::ReceiptSample;

#[derive(Debug, Clone)]
pub struct DHighWaterSnapshot {
    pub authoritative_high_water: u64,
    pub observed_high_water: u64,
    pub source_sequence: u64,
    pub linearization_token: String,
    pub provenance: String,
}

/// Result of evaluating one normative predicate.
#[derive(Debug, Clone)]
pub struct PredicateResult {
    pub condition_id: &'static str,
    pub passed: bool,
    pub evidence_reference: String,
}

/// Exactly six condition results are required for the F1 proof.
#[derive(Debug, Clone)]
pub struct SixConditionReport {
    pub c1: PredicateResult,
    pub c2: PredicateResult,
    pub c3: PredicateResult,
    pub c4: PredicateResult,
    pub c5: PredicateResult,
    pub c6: PredicateResult,
}

/// Evaluate all six D-contract predicates using a real authoritative source.
///
/// Contract:
/// - Read from the injected DAuthoritativeSource; do not accept caller-supplied
///   booleans as substitutes for authoritative observations.
/// - Preserve the exact-equality and linearization evidence required by the
///   normative D contract.
/// - Require the Receipt sample and ten scenario results.
/// - Any missing callback, provenance, receipt, scenario, or predicate
///   evidence is BLOCKED/FAIL, never PASS by default.
pub fn evaluate_six_conditions(
    source: &dyn DAuthoritativeSource,
    receipt: &ReceiptSample,
) -> Result<SixConditionReport, DProofError> {
    todo!("Step 2: map and implement the six exact D-contract predicates")
}

/// Explicitly reject an evidence package that uses only a test double.
///
/// Test doubles may be used in unit tests, but this function must not report
/// operational proof as complete for them.
pub fn require_operational_source(
    source: &dyn DAuthoritativeSource,
) -> Result<(), DProofError> {
    todo!("Step 2: enforce real-source provenance and adapter identity")
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
