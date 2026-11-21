//! D dual-high-water proof gate for F-43-01/F1.
//!
//! The six-condition evaluator is a pure predicate over supplied evidence.
//! It does not authenticate that evidence or establish an operational D source.
//! `require_operational_source` and `require_evidence_gate` remain the separate,
//! fail-closed operational boundary. Evaluate one namespace per evidence value.

use crate::authority::DAuthoritativeSource;
use crate::receipt::{ReceiptSample, validate_operational_receipt};

/// Legacy source snapshot shape retained for the authority adapter boundary.
/// It is not accepted as a substitute for `SixConditionEvidence`.
#[derive(Debug, Clone)]
pub struct DHighWaterSnapshot {
    pub authoritative_high_water: u64,
    pub observed_high_water: u64,
    pub source_sequence: u64,
    pub linearization_token: String,
    pub provenance: String,
}

/// Nominal cutoff type for producer-registration high-water values.
///
/// This deliberately implements equality but not `PartialOrd` or `Ord`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProducerRegistrationHighWater(u64);

impl ProducerRegistrationHighWater {
    pub const MAX: u64 = i64::MAX as u64;

    /// Construct a cutoff only when it is in the contract's permitted range.
    pub const fn new(value: u64) -> Option<Self> {
        if value <= Self::MAX {
            Some(Self(value))
        } else {
            None
        }
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

impl TryFrom<&str> for ProducerRegistrationHighWater {
    type Error = HighWaterParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(HighWaterParseError);
        }

        let parsed = value
            .parse::<u64>()
            .map_err(|_| HighWaterParseError)?;
        Self::new(parsed).ok_or(HighWaterParseError)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighWaterParseError;

impl std::fmt::Display for HighWaterParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(
            "producer-registration high-water must be an unsigned decimal integer in 0..=2^63-1",
        )
    }
}

impl std::error::Error for HighWaterParseError {}

/// A fence generation is a different nominal domain from producer high-water.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ForgetFenceGeneration(pub u64);

/// One registration considered by the D serialization proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreFenceRegistration {
    pub sequence: u64,
    pub accepted: bool,
    pub included_in_barrier: bool,
}

/// The only permitted post-fence outcomes for an ordinary covered registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostFenceDisposition {
    Rejected,
    Quarantined,
    CleanupOnly,
    Published,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostFenceRegistration {
    pub disposition: PostFenceDisposition,
}

/// Serialization-order evidence for one namespace and one fence linearization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializationProof {
    pub fence_linearization_reference: String,
    /// True only when the source proves both lists are exhaustive for this fence.
    pub ordering_is_exhaustive: bool,
    pub pre_fence_registrations: Vec<PreFenceRegistration>,
    pub post_fence_registrations: Vec<PostFenceRegistration>,
}

/// All fields are optional so absence is distinct from a legitimate zero value.
///
/// This structure represents one namespace only. Multi-namespace operations
/// must construct and evaluate an independent value for each namespace; callers
/// must not aggregate or substitute high-water values across namespaces.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SixConditionEvidence {
    // Condition 1: authority, ledger identity/namespace, and registry version.
    pub brain_authority_id: Option<String>,
    pub producer_registration_ledger_identity: Option<String>,
    pub producer_registration_ledger_namespace: Option<String>,
    pub producer_registry_version: Option<String>,

    // Condition 2: immutable operation identity and exact request/target digests.
    pub forget_operation_id: Option<String>,
    pub idempotency_identity: Option<String>,
    pub canonical_request_digest: Option<String>,
    pub target_set_digest: Option<String>,

    // Condition 3: fence identity and installation commit cursor.
    pub forget_fence_cursor: Option<String>,
    pub forget_fence_generation: Option<ForgetFenceGeneration>,
    pub installation_canonical_commit_cursor: Option<String>,
    pub immutable_snapshot_id: Option<String>,

    // Condition 4: same nominal type, same immutable snapshot, equality only.
    pub captured_producer_registration_high_water: Option<ProducerRegistrationHighWater>,
    pub barrier_snapshot_producer_registration_high_water: Option<ProducerRegistrationHighWater>,

    // Condition 5: barrier receipt binds the complete identity set and proof.
    pub barrier_brain_authority_id: Option<String>,
    pub barrier_producer_registration_ledger_identity: Option<String>,
    pub barrier_producer_registration_ledger_namespace: Option<String>,
    pub barrier_producer_registry_version: Option<String>,
    pub barrier_forget_operation_id: Option<String>,
    pub barrier_idempotency_identity: Option<String>,
    pub barrier_canonical_request_digest: Option<String>,
    pub barrier_target_set_digest: Option<String>,
    pub barrier_forget_fence_cursor: Option<String>,
    pub barrier_forget_fence_generation: Option<ForgetFenceGeneration>,
    pub barrier_installation_canonical_commit_cursor: Option<String>,
    pub barrier_immutable_snapshot_id: Option<String>,
    pub ledger_consistency_proof_reference: Option<String>,

    // Condition 6: authoritative serialization-order proof.
    pub forget_fence_linearization_reference: Option<String>,
    pub serialization_proof: Option<SerializationProof>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SixConditionVerdict {
    Valid,
    Invalid(String),
    Stale(String),
    Unknown(String),
}

/// Pure six-condition predicate. This does not establish source authenticity,
/// currentness, or revocation status; those remain independent operational gates.
///
/// Explicit contradictions take precedence over missing evidence. A complete
/// result is therefore necessary, but not sufficient, for operational proof.
pub fn evaluate_six_conditions(evidence: &SixConditionEvidence) -> SixConditionVerdict {
    macro_rules! mismatch {
        ($left:expr, $right:expr, $label:literal) => {
            if let (Some(left), Some(right)) = (&$left, &$right) {
                if left != right {
                    return SixConditionVerdict::Invalid(
                        concat!("explicit mismatch: ", $label).to_owned(),
                    );
                }
            }
        };
    }

    // Check all available explicit identity/operation/cursor/snapshot mismatches
    // before classifying any absent evidence as UNKNOWN.
    mismatch!(
        evidence.brain_authority_id,
        evidence.barrier_brain_authority_id,
        "BrainAuthorityId"
    );
    mismatch!(
        evidence.producer_registration_ledger_identity,
        evidence.barrier_producer_registration_ledger_identity,
        "ProducerRegistrationLedger identity"
    );
    mismatch!(
        evidence.producer_registration_ledger_namespace,
        evidence.barrier_producer_registration_ledger_namespace,
        "ProducerRegistrationLedger namespace"
    );
    mismatch!(
        evidence.producer_registry_version,
        evidence.barrier_producer_registry_version,
        "ProducerRegistryVersion"
    );
    mismatch!(
        evidence.forget_operation_id,
        evidence.barrier_forget_operation_id,
        "ForgetOperationId"
    );
    mismatch!(
        evidence.idempotency_identity,
        evidence.barrier_idempotency_identity,
        "idempotency identity"
    );
    mismatch!(
        evidence.canonical_request_digest,
        evidence.barrier_canonical_request_digest,
        "canonical request digest"
    );
    mismatch!(
        evidence.target_set_digest,
        evidence.barrier_target_set_digest,
        "target-set digest"
    );
    mismatch!(
        evidence.forget_fence_cursor,
        evidence.barrier_forget_fence_cursor,
        "ForgetFenceCursor"
    );
    mismatch!(
        evidence.forget_fence_generation,
        evidence.barrier_forget_fence_generation,
        "ForgetFenceGeneration"
    );
    mismatch!(
        evidence.installation_canonical_commit_cursor,
        evidence.barrier_installation_canonical_commit_cursor,
        "installation CanonicalCommitCursor"
    );
    mismatch!(
        evidence.immutable_snapshot_id,
        evidence.barrier_immutable_snapshot_id,
        "immutable snapshot identity"
    );
    mismatch!(
        evidence.forget_fence_linearization_reference,
        evidence.serialization_proof.as_ref().map(|proof| proof.fence_linearization_reference.clone()),
        "fence linearization reference"
    );

    if let (
        Some(captured),
        Some(barrier),
    ) = (
        evidence.captured_producer_registration_high_water,
        evidence.barrier_snapshot_producer_registration_high_water,
    ) {
        if captured != barrier {
            return SixConditionVerdict::Invalid(
                "captured and barrier-snapshot ProducerRegistrationHighWater differ".to_owned(),
            );
        }
    }

    if let (Some(high_water), Some(proof)) = (
        evidence.barrier_snapshot_producer_registration_high_water,
        evidence.serialization_proof.as_ref(),
    ) {
        for registration in &proof.pre_fence_registrations {
            if registration.accepted
                && (registration.sequence > high_water.get()
                    || !registration.included_in_barrier)
            {
                return SixConditionVerdict::Invalid(
                    "pre-fence accepted registration is above H or absent from barrier".to_owned(),
                );
            }
        }
        if proof
            .post_fence_registrations
            .iter()
            .any(|registration| registration.disposition == PostFenceDisposition::Published)
        {
            return SixConditionVerdict::Invalid(
                "ordinary post-fence registration was published".to_owned(),
            );
        }
    }

    // No missing field may be replaced by a default or sentinel.
    let required_strings = [
        evidence.brain_authority_id.as_ref(),
        evidence.producer_registration_ledger_identity.as_ref(),
        evidence.producer_registration_ledger_namespace.as_ref(),
        evidence.producer_registry_version.as_ref(),
        evidence.forget_operation_id.as_ref(),
        evidence.idempotency_identity.as_ref(),
        evidence.canonical_request_digest.as_ref(),
        evidence.target_set_digest.as_ref(),
        evidence.forget_fence_cursor.as_ref(),
        evidence.installation_canonical_commit_cursor.as_ref(),
        evidence.barrier_brain_authority_id.as_ref(),
        evidence.barrier_producer_registration_ledger_identity.as_ref(),
        evidence.barrier_producer_registration_ledger_namespace.as_ref(),
        evidence.barrier_producer_registry_version.as_ref(),
        evidence.barrier_forget_operation_id.as_ref(),
        evidence.barrier_idempotency_identity.as_ref(),
        evidence.barrier_canonical_request_digest.as_ref(),
        evidence.barrier_target_set_digest.as_ref(),
        evidence.barrier_forget_fence_cursor.as_ref(),
        evidence.barrier_installation_canonical_commit_cursor.as_ref(),
        evidence.immutable_snapshot_id.as_ref(),
        evidence.barrier_immutable_snapshot_id.as_ref(),
        evidence.ledger_consistency_proof_reference.as_ref(),
        evidence.forget_fence_linearization_reference.as_ref(),
    ];
    if required_strings
        .iter()
        .any(|field| field.is_none_or(|value| value.trim().is_empty()))
    {
        return SixConditionVerdict::Unknown(
            "one or more identity, digest, cursor, snapshot, or proof fields are missing or blank"
                .to_owned(),
        );
    }

    let Some(generation) = evidence.forget_fence_generation else {
        return SixConditionVerdict::Unknown("ForgetFenceGeneration is missing".to_owned());
    };
    if evidence.barrier_forget_fence_generation.is_none() {
        return SixConditionVerdict::Unknown(
            "barrier-bound ForgetFenceGeneration is missing".to_owned(),
        );
    }
    let _ = generation;

    if evidence.captured_producer_registration_high_water.is_none()
        || evidence
            .barrier_snapshot_producer_registration_high_water
            .is_none()
    {
        return SixConditionVerdict::Unknown(
            "one or both typed ProducerRegistrationHighWater cutoffs are missing".to_owned(),
        );
    }

    let Some(proof_reference) = evidence.ledger_consistency_proof_reference.as_ref() else {
        return SixConditionVerdict::Unknown("ledger consistency proof is missing".to_owned());
    };
    if proof_reference.trim().is_empty() {
        return SixConditionVerdict::Unknown("ledger consistency proof is unavailable".to_owned());
    }

    let Some(serialization_proof) = evidence.serialization_proof.as_ref() else {
        return SixConditionVerdict::Unknown("serialization-order proof is missing".to_owned());
    };
    if serialization_proof.fence_linearization_reference.trim().is_empty() {
        return SixConditionVerdict::Unknown(
            "serialization proof lacks a fence linearization reference".to_owned(),
        );
    }
    if !serialization_proof.ordering_is_exhaustive {
        return SixConditionVerdict::Unknown(
            "serialization proof does not establish exhaustive ordering coverage".to_owned(),
        );
    }

    // Empty registration lists are not proof that ordering was checked.
    if serialization_proof.pre_fence_registrations.is_empty()
        && serialization_proof.post_fence_registrations.is_empty()
    {
        return SixConditionVerdict::Unknown(
            "serialization proof contains no ordering observations".to_owned(),
        );
    }

    SixConditionVerdict::Valid
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

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_evidence() -> SixConditionEvidence {
        let mut evidence = SixConditionEvidence::default();
        evidence.brain_authority_id = Some("authority-A".into());
        evidence.producer_registration_ledger_identity = Some("ledger-A".into());
        evidence.producer_registration_ledger_namespace = Some("namespace-A".into());
        evidence.producer_registry_version = Some("registry-v7".into());
        evidence.forget_operation_id = Some("forget-1".into());
        evidence.idempotency_identity = Some("idem-1".into());
        evidence.canonical_request_digest = Some("request-digest".into());
        evidence.target_set_digest = Some("target-digest".into());
        evidence.forget_fence_cursor = Some("fence-cursor-8".into());
        evidence.forget_fence_generation = Some(ForgetFenceGeneration(8));
        evidence.installation_canonical_commit_cursor = Some("commit-cursor-9".into());
        evidence.immutable_snapshot_id = Some("snapshot-1".into());
        evidence.captured_producer_registration_high_water =
            ProducerRegistrationHighWater::new(5);
        evidence.barrier_snapshot_producer_registration_high_water =
            ProducerRegistrationHighWater::new(5);
        evidence.barrier_brain_authority_id = Some("authority-A".into());
        evidence.barrier_producer_registration_ledger_identity = Some("ledger-A".into());
        evidence.barrier_producer_registration_ledger_namespace = Some("namespace-A".into());
        evidence.barrier_producer_registry_version = Some("registry-v7".into());
        evidence.barrier_forget_operation_id = Some("forget-1".into());
        evidence.barrier_idempotency_identity = Some("idem-1".into());
        evidence.barrier_canonical_request_digest = Some("request-digest".into());
        evidence.barrier_target_set_digest = Some("target-digest".into());
        evidence.barrier_forget_fence_cursor = Some("fence-cursor-8".into());
        evidence.barrier_forget_fence_generation = Some(ForgetFenceGeneration(8));
        evidence.barrier_installation_canonical_commit_cursor =
            Some("commit-cursor-9".into());
        evidence.barrier_immutable_snapshot_id = Some("snapshot-1".into());
        evidence.ledger_consistency_proof_reference = Some("ledger-proof-1".into());
        evidence.forget_fence_linearization_reference = Some("linearization-8".into());
        evidence.serialization_proof = Some(SerializationProof {
            fence_linearization_reference: "linearization-8".into(),
            ordering_is_exhaustive: true,
            pre_fence_registrations: vec![PreFenceRegistration {
                sequence: 5,
                accepted: true,
                included_in_barrier: true,
            }],
            post_fence_registrations: vec![PostFenceRegistration {
                disposition: PostFenceDisposition::Rejected,
            }],
        });
        evidence
    }

    #[test]
    fn all_six_conditions_complete_return_valid() {
        assert_eq!(
            evaluate_six_conditions(&complete_evidence()),
            SixConditionVerdict::Valid
        );
    }

    #[test]
    fn captured_high_water_greater_than_barrier_is_invalid() {
        let mut evidence = complete_evidence();
        evidence.captured_producer_registration_high_water =
            ProducerRegistrationHighWater::new(6);
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Invalid(_)
        ));
    }

    #[test]
    fn captured_high_water_less_than_barrier_is_invalid() {
        let mut evidence = complete_evidence();
        evidence.captured_producer_registration_high_water =
            ProducerRegistrationHighWater::new(4);
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Invalid(_)
        ));
    }

    #[test]
    fn missing_captured_high_water_is_unknown() {
        let mut evidence = complete_evidence();
        evidence.captured_producer_registration_high_water = None;
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Unknown(_)
        ));
    }

    #[test]
    fn missing_barrier_high_water_is_unknown() {
        let mut evidence = complete_evidence();
        evidence.barrier_snapshot_producer_registration_high_water = None;
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Unknown(_)
        ));
    }

    #[test]
    fn target_set_digest_mismatch_is_invalid() {
        let mut evidence = complete_evidence();
        evidence.barrier_target_set_digest = Some("different-targets".into());
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Invalid(_)
        ));
    }

    #[test]
    fn fence_generation_mismatch_is_invalid() {
        let mut evidence = complete_evidence();
        evidence.barrier_forget_fence_generation = Some(ForgetFenceGeneration(9));
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Invalid(_)
        ));
    }

    #[test]
    fn missing_serialization_proof_is_unknown() {
        let mut evidence = complete_evidence();
        evidence.serialization_proof = None;
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Unknown(_)
        ));
    }

    #[test]
    fn partial_first_three_conditions_are_unknown() {
        let mut evidence = SixConditionEvidence::default();
        evidence.brain_authority_id = Some("authority-A".into());
        evidence.producer_registration_ledger_identity = Some("ledger-A".into());
        evidence.producer_registration_ledger_namespace = Some("namespace-A".into());
        evidence.producer_registry_version = Some("registry-v7".into());
        evidence.forget_operation_id = Some("forget-1".into());
        evidence.idempotency_identity = Some("idem-1".into());
        evidence.canonical_request_digest = Some("request-digest".into());
        evidence.target_set_digest = Some("target-digest".into());
        evidence.forget_fence_cursor = Some("fence-cursor-8".into());
        evidence.forget_fence_generation = Some(ForgetFenceGeneration(8));
        evidence.installation_canonical_commit_cursor = Some("commit-cursor-9".into());
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Unknown(_)
        ));
    }

    #[test]
    fn explicit_mismatch_precedes_other_missing_evidence() {
        let mut evidence = SixConditionEvidence::default();
        evidence.target_set_digest = Some("left".into());
        evidence.barrier_target_set_digest = Some("right".into());
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Invalid(_)
        ));
    }

    #[test]
    fn accepted_pre_fence_registration_above_h_is_invalid() {
        let mut evidence = complete_evidence();
        evidence.serialization_proof.as_mut().unwrap().pre_fence_registrations[0].sequence = 6;
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Invalid(_)
        ));
    }

    #[test]
    fn accepted_pre_fence_registration_missing_from_barrier_is_invalid() {
        let mut evidence = complete_evidence();
        evidence.serialization_proof.as_mut().unwrap().pre_fence_registrations[0]
            .included_in_barrier = false;
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Invalid(_)
        ));
    }

    #[test]
    fn published_post_fence_registration_is_invalid() {
        let mut evidence = complete_evidence();
        evidence.serialization_proof.as_mut().unwrap().post_fence_registrations[0].disposition =
            PostFenceDisposition::Published;
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Invalid(_)
        ));
    }

    #[test]
    fn empty_serialization_observations_are_unknown() {
        let mut evidence = complete_evidence();
        evidence.serialization_proof.as_mut().unwrap().pre_fence_registrations.clear();
        evidence.serialization_proof.as_mut().unwrap().post_fence_registrations.clear();
        assert!(matches!(
            evaluate_six_conditions(&evidence),
            SixConditionVerdict::Unknown(_)
        ));
    }

    #[test]
    fn plus_sign_is_rejected() {
        assert!(ProducerRegistrationHighWater::try_from("+5").is_err());
    }

    #[test]
    fn fractional_value_is_rejected() {
        assert!(ProducerRegistrationHighWater::try_from("5.0").is_err());
    }

    #[test]
    fn leading_whitespace_is_rejected() {
        assert!(ProducerRegistrationHighWater::try_from(" 5").is_err());
    }

    #[test]
    fn trailing_whitespace_is_rejected() {
        assert!(ProducerRegistrationHighWater::try_from("5 ").is_err());
    }

    #[test]
    fn exponent_notation_is_rejected() {
        assert!(ProducerRegistrationHighWater::try_from("1e3").is_err());
    }

    #[test]
    fn digit_separator_is_rejected() {
        assert!(ProducerRegistrationHighWater::try_from("5,000").is_err());
    }

    #[test]
    fn negative_value_is_rejected() {
        assert!(ProducerRegistrationHighWater::try_from("-5").is_err());
    }

    #[test]
    fn leading_zeroes_are_accepted_and_canonicalized_by_value() {
        let padded = ProducerRegistrationHighWater::try_from("0005").unwrap();
        let canonical = ProducerRegistrationHighWater::try_from("5").unwrap();
        assert_eq!(padded, canonical);
    }

    #[test]
    fn two_to_the_sixty_third_is_rejected() {
        assert!(ProducerRegistrationHighWater::try_from("9223372036854775808").is_err());
    }

    #[test]
    fn maximum_signed_sixty_four_bit_value_is_accepted() {
        assert_eq!(
            ProducerRegistrationHighWater::try_from("9223372036854775807")
                .unwrap()
                .get(),
            i64::MAX as u64
        );
    }

    #[test]
    fn zero_is_a_valid_high_water_value() {
        assert_eq!(
            ProducerRegistrationHighWater::try_from("0").unwrap().get(),
            0
        );
    }

    #[test]
    fn empty_high_water_is_rejected() {
        assert!(ProducerRegistrationHighWater::try_from("").is_err());
    }
}
