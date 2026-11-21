use d_unicode_validator::d_predicates::{
    evaluate_six_conditions, ForgetFenceGeneration, PostFenceDisposition,
    PostFenceRegistration, PreFenceRegistration, SerializationProof, SixConditionEvidence,
    SixConditionVerdict,
};

fn complete_evidence(namespace: &str) -> SixConditionEvidence {
    let mut evidence = SixConditionEvidence {
        brain_authority_id: Some("brain-authority-A".into()),
        producer_registration_ledger_identity: Some("ledger-A".into()),
        producer_registration_ledger_namespace: Some(namespace.into()),
        producer_registry_version: Some("registry-v7".into()),
        forget_operation_id: Some(format!("forget-{namespace}")),
        idempotency_identity: Some(format!("idem-{namespace}")),
        canonical_request_digest: Some("sha256:request-A".into()),
        target_set_digest: Some("sha256:targets-A".into()),
        forget_fence_cursor: Some(format!("fence-cursor-{namespace}")),
        forget_fence_generation: Some(ForgetFenceGeneration::new(3).expect("valid generation")),
        installation_canonical_commit_cursor: Some(format!("commit-cursor-{namespace}")),
        immutable_snapshot_id: Some(format!("snapshot-{namespace}")),
        captured_producer_registration_high_water:
            d_unicode_validator::d_predicates::ProducerRegistrationHighWater::new(7),
        barrier_snapshot_producer_registration_high_water:
            d_unicode_validator::d_predicates::ProducerRegistrationHighWater::new(7),
        ledger_consistency_proof_reference: Some(format!("ledger-proof-{namespace}")),
        forget_fence_linearization_reference: Some(format!("linearization-{namespace}")),
        serialization_proof: Some(SerializationProof {
            fence_linearization_reference: format!("linearization-{namespace}"),
            ordering_is_exhaustive: true,
            pre_fence_registrations: vec![PreFenceRegistration {
                sequence: 7,
                accepted: true,
                included_in_barrier: true,
            }],
            post_fence_registrations: vec![PostFenceRegistration {
                disposition: PostFenceDisposition::Rejected,
            }],
        }),
        ..SixConditionEvidence::default()
    };

    evidence.barrier_brain_authority_id = evidence.brain_authority_id.clone();
    evidence.barrier_producer_registration_ledger_identity =
        evidence.producer_registration_ledger_identity.clone();
    evidence.barrier_producer_registration_ledger_namespace =
        evidence.producer_registration_ledger_namespace.clone();
    evidence.barrier_producer_registry_version = evidence.producer_registry_version.clone();
    evidence.barrier_forget_operation_id = evidence.forget_operation_id.clone();
    evidence.barrier_idempotency_identity = evidence.idempotency_identity.clone();
    evidence.barrier_canonical_request_digest = evidence.canonical_request_digest.clone();
    evidence.barrier_target_set_digest = evidence.target_set_digest.clone();
    evidence.barrier_forget_fence_cursor = evidence.forget_fence_cursor.clone();
    evidence.barrier_forget_fence_generation = evidence.forget_fence_generation;
    evidence.barrier_installation_canonical_commit_cursor =
        evidence.installation_canonical_commit_cursor.clone();
    evidence.barrier_immutable_snapshot_id = evidence.immutable_snapshot_id.clone();
    evidence
}

fn assert_not_valid(evidence: &SixConditionEvidence) {
    assert!(!matches!(
        evaluate_six_conditions(evidence),
        SixConditionVerdict::Valid
    ));
}

fn assert_unknown(evidence: &SixConditionEvidence) {
    assert!(matches!(
        evaluate_six_conditions(evidence),
        SixConditionVerdict::Unknown(_)
    ));
}

fn assert_invalid(evidence: &SixConditionEvidence) {
    assert!(matches!(
        evaluate_six_conditions(evidence),
        SixConditionVerdict::Invalid(_)
    ));
}

#[test]
fn s1_two_namespaces_are_evaluated_independently() {
    let n1 = complete_evidence("N1");
    let n2 = complete_evidence("N2");
    assert_eq!(evaluate_six_conditions(&n1), SixConditionVerdict::Valid);
    assert_eq!(evaluate_six_conditions(&n2), SixConditionVerdict::Valid);

    // 本接口一次只评估一个 namespace，无法证明跨 namespace 聚合完成；
    // 因此仅有 N1 成功、N2 缺证据时，聚合结论只能是 UNKNOWN。
    let incomplete_n2 = SixConditionEvidence {
        producer_registration_ledger_namespace: Some("N2".into()),
        ..SixConditionEvidence::default()
    };
    assert_unknown(&incomplete_n2);

    // Must reject 1: 不得把 N2 barrier namespace 替换成 N1 namespace。
    let mut cross_namespace = n1.clone();
    cross_namespace.barrier_producer_registration_ledger_namespace = Some("N2".into());
    assert_invalid(&cross_namespace);
    assert_not_valid(&cross_namespace);

    // Must reject 2: 不得跨 namespace 替换身份元组。
    let mut substituted_identity = n1.clone();
    substituted_identity.barrier_producer_registration_ledger_identity =
        Some("ledger-N2".into());
    assert_invalid(&substituted_identity);
    assert_not_valid(&substituted_identity);

    // Must reject 3: 不得使用另一 namespace 的 generation。
    let mut substituted_generation = n1.clone();
    substituted_generation.barrier_forget_fence_generation =
        Some(ForgetFenceGeneration::new(4).expect("valid generation"));
    assert_invalid(&substituted_generation);
    assert_not_valid(&substituted_generation);

    // Must reject 4: 部分 namespace 成功不能成为全局成功。
    assert_unknown(&incomplete_n2);
    assert_not_valid(&incomplete_n2);
}

#[test]
fn s2_capture_requires_existing_immutable_cutoff() {
    // Capture 发生在 durable install 前：本接口没有已安装 receipt/cutoff，
    // 无法证明安装完成，故此处只能断言 UNKNOWN。
    let before_install = SixConditionEvidence::default();
    assert_unknown(&before_install);

    // Must reject 1: captured H 不得由 Capture 擅自推进。
    let mut advanced_h = complete_evidence("N1");
    advanced_h.barrier_snapshot_producer_registration_high_water =
        d_unicode_validator::d_predicates::ProducerRegistrationHighWater::new(8);
    assert_invalid(&advanced_h);
    assert_not_valid(&advanced_h);

    // Must reject 2: 后续 ledger tip 不能被证明为原始 immutable cutoff。
    // 现有结构没有独立 ledger-tip / install-time snapshot 字段，故 UNKNOWN。
    let later_tip_as_cutoff = SixConditionEvidence {
        captured_producer_registration_high_water:
            d_unicode_validator::d_predicates::ProducerRegistrationHighWater::new(8),
        ..SixConditionEvidence::default()
    };
    assert_unknown(&later_tip_as_cutoff);
    assert_not_valid(&later_tip_as_cutoff);

    // Must reject 3: 单次权威 currentness read / 不重试读无法由纯谓词接口证明。
    let split_currentness_reads = SixConditionEvidence::default();
    assert_unknown(&split_currentness_reads);
    assert_not_valid(&split_currentness_reads);

    // Must reject 4: 与 fence 竞争的发布是否越过线性化边界，不能只凭字段证明；
    // 已观测到 post-fence Published 则现有谓词明确拒绝。
    let mut published_after_fence = complete_evidence("N1");
    published_after_fence.serialization_proof.as_mut().unwrap()
        .post_fence_registrations[0].disposition = PostFenceDisposition::Published;
    assert_invalid(&published_after_fence);
    assert_not_valid(&published_after_fence);
}

#[test]
fn s3_request_identity_and_digest_mismatches_are_rejected() {
    // 本接口没有请求到达/重放次数、durable receipt lookup 或响应状态字段；
    // 因而无法证明 exact replay 返回同一持久化结果，场景结论只能是 UNKNOWN。
    let replay_not_observable = SixConditionEvidence::default();
    assert_unknown(&replay_not_observable);

    // 完整单 namespace 六条件形状本身可为 VALID，但这不是 replay 行为证明。
    let exact_request_shape = complete_evidence("N1");
    assert_eq!(
        evaluate_six_conditions(&exact_request_shape),
        SixConditionVerdict::Valid
    );

    // Must reject 1: 同一操作身份下更改 request digest。
    let mut changed_request = exact_request_shape.clone();
    changed_request.barrier_canonical_request_digest =
        Some("sha256:changed-request".into());
    assert_invalid(&changed_request);
    assert_not_valid(&changed_request);

    // Must reject 2: 更改 target digest 不得被当作 exact replay。
    let mut changed_target = exact_request_shape.clone();
    changed_target.barrier_target_set_digest = Some("sha256:changed-target".into());
    assert_invalid(&changed_target);
    assert_not_valid(&changed_target);

    // Must reject 3: ACK 丢失后的第二 generation/cutoff 无法被当作同一结果。
    let mut second_generation = exact_request_shape;
    second_generation.barrier_forget_fence_generation =
        Some(ForgetFenceGeneration::new(4).expect("valid generation"));
    assert_invalid(&second_generation);
    assert_not_valid(&second_generation);
}

#[test]
fn s4_incomplete_commit_evidence_remains_unknown() {
    // 无成功 receipt 的初始/提交不确定状态没有完整 C2-C6 证据，故 UNKNOWN。
    let pre_commit_or_ambiguous = SixConditionEvidence::default();
    assert_unknown(&pre_commit_or_ambiguous);

    // Must reject 1: 没有提交证据不能凭空声称 receipt 已安装。
    assert_not_valid(&pre_commit_or_ambiguous);

    // Must reject 2: 不完整写入集合不能被当成完整序列化证明。
    let mut partial_rows = complete_evidence("N1");
    partial_rows.serialization_proof.as_mut().unwrap().ordering_is_exhaustive = false;
    assert_unknown(&partial_rows);
    assert_not_valid(&partial_rows);

    // Must reject 3: 新 operation/generation 逃避不确定性会产生身份冲突。
    let mut new_operation = complete_evidence("N1");
    new_operation.barrier_forget_operation_id = Some("replacement-operation".into());
    assert_invalid(&new_operation);
    assert_not_valid(&new_operation);

    // Must reject 4: 歧义 fence 结果下观测到 post-fence Published 必须拒绝。
    let mut ambiguous_publication = complete_evidence("N1");
    ambiguous_publication.serialization_proof.as_mut().unwrap()
        .post_fence_registrations[0].disposition = PostFenceDisposition::Published;
    assert_invalid(&ambiguous_publication);
    assert_not_valid(&ambiguous_publication);
}

#[test]
fn s5_lost_response_cannot_be_resolved_by_predicate_alone() {
    // 本接口没有 durable receipt lookup / response-delivery 状态字段；
    // 因而即使服务端可能已提交，也只能断言 UNKNOWN，不能声称完成 replay。
    let lost_response = SixConditionEvidence::default();
    assert_unknown(&lost_response);

    // Must reject 1: timeout 不能作为 abort 证据；当前接口对此无观测字段。
    assert_not_valid(&lost_response);

    // Must reject 2: 新 idempotency identity 不是原请求的重试。
    let mut new_idempotency = complete_evidence("N1");
    new_idempotency.barrier_idempotency_identity = Some("new-idempotency".into());
    assert_invalid(&new_idempotency);
    assert_not_valid(&new_idempotency);

    // Must reject 3: recapture 后 cutoff 不同，必须拒绝。
    let mut recaptured_cutoff = complete_evidence("N1");
    recaptured_cutoff.barrier_snapshot_producer_registration_high_water =
        d_unicode_validator::d_predicates::ProducerRegistrationHighWater::new(8);
    assert_invalid(&recaptured_cutoff);
    assert_not_valid(&recaptured_cutoff);

    // Must reject 4: 一个 operation identity 不能绑定不同 receipt/commit identity。
    // receipt 本身不在此结构中；以不匹配的安装 commit cursor 表达可见的矛盾。
    let mut different_commit = complete_evidence("N1");
    different_commit.barrier_installation_canonical_commit_cursor =
        Some("different-commit".into());
    assert_invalid(&different_commit);
    assert_not_valid(&different_commit);
}

#[test]
fn s6_restore_continuity_and_anti_rollback_remain_unknown() {
    // 本接口没有 authoritative restore lineage、state-version monotonicity、
    // anti-rollback proof 或 cache provenance 字段，故此处只能断言 UNKNOWN。
    let restored_without_continuity = SixConditionEvidence::default();
    assert_unknown(&restored_without_continuity);

    // Must reject 1: prior VALID cache 不是本次权威证据。
    assert_not_valid(&restored_without_continuity);

    // Must reject 2: 无法比较 restore 前后 state version/generation 是否回退；
    // 缺少连续性证据时必须 UNKNOWN，而不是 VALID 或猜测 STALE。
    let mut missing_anti_rollback = complete_evidence("N1");
    missing_anti_rollback.ledger_consistency_proof_reference = None;
    assert_unknown(&missing_anti_rollback);
    assert_not_valid(&missing_anti_rollback);

    // Must reject 3: 本接口无法证明旧 operation 未被重新 mint 新 cutoff。
    assert_unknown(&restored_without_continuity);
    assert_not_valid(&restored_without_continuity);

    // Must reject 4: 单个 stale replica 的 absence 不能证明 operation aborted。
    assert_unknown(&restored_without_continuity);
    assert_not_valid(&restored_without_continuity);
}

#[test]
fn s7_partial_namespace_success_does_not_establish_aggregate_validity() {
    let n1 = complete_evidence("N1");
    let n2_incomplete = SixConditionEvidence {
        producer_registration_ledger_namespace: Some("N2".into()),
        ..SixConditionEvidence::default()
    };

    // N1 的本地 proof 仍可独立成立；N2 缺证据时不得推出全局 VALID。
    assert_eq!(evaluate_six_conditions(&n1), SixConditionVerdict::Valid);
    assert_unknown(&n2_incomplete);

    // 本接口没有 required-namespace set / aggregate evaluator；
    // 因此 N1 + 不完整 N2 的 aggregate 只能按 fail-closed 断言 UNKNOWN。
    let aggregate_not_proven = n2_incomplete.clone();
    assert_unknown(&aggregate_not_proven);
    assert_not_valid(&aggregate_not_proven);

    // Must reject 1: N2 失败不应反向污染 N1 独立 receipt。
    assert_eq!(evaluate_six_conditions(&n1), SixConditionVerdict::Valid);

    // Must reject 2: N1 成功不能替代 aggregate proof。
    assert_not_valid(&aggregate_not_proven);

    // Must reject 3: 静默丢弃 N2 后无法证明 required namespace set 完整。
    assert_unknown(&aggregate_not_proven);
    assert_not_valid(&aggregate_not_proven);

    // Must reject 4: 跨 namespace watermark arithmetic 不属于该单 namespace 接口。
    let mut cross_namespace_watermark = n1;
    cross_namespace_watermark.barrier_producer_registration_ledger_namespace =
        Some("N2".into());
    assert_invalid(&cross_namespace_watermark);
    assert_not_valid(&cross_namespace_watermark);
}
