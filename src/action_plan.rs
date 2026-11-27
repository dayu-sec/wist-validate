//! `ActionPlan` validation entrypoints.

use std::collections::BTreeSet;

use wist_contracts::API_VERSION_V1;
use wist_contracts::action_plan::{
    ACTION_PLAN_KIND, ATTESTATION_ALGORITHM_ED25519, ActionPlan, PLAN_DIGEST_VERSION,
    STEP_KIND_INVOKE, is_known_step_kind,
};

use crate::{ValidationError, parse_rfc3339, require_non_empty};

pub fn validate_action_plan(contract: &ActionPlan) -> Result<(), ValidationError> {
    if contract.api_version != API_VERSION_V1 {
        return Err(ValidationError::new("invalid_api_version"));
    }
    if contract.kind != ACTION_PLAN_KIND {
        return Err(ValidationError::new("invalid_kind"));
    }

    require_non_empty(&contract.meta.action_id, "missing_action_id")?;
    require_non_empty(&contract.meta.request_id, "missing_request_id")?;
    require_non_empty(&contract.meta.tenant_id, "missing_tenant_id")?;
    require_non_empty(&contract.meta.environment_id, "missing_environment_id")?;
    if contract.meta.plan_version == 0 {
        return Err(ValidationError::new("invalid_plan_version"));
    }

    let compiled_at = parse_rfc3339(&contract.meta.compiled_at, "invalid_compiled_at")?;
    let expires_at = parse_rfc3339(&contract.meta.expires_at, "invalid_expires_at")?;
    if expires_at <= compiled_at {
        return Err(ValidationError::new("expired_or_non_increasing_window"));
    }

    require_non_empty(&contract.target.agent_id, "missing_target_agent_id")?;
    require_non_empty(&contract.target.node_id, "missing_target_node_id")?;
    require_non_empty(&contract.target.platform, "missing_target_platform")?;
    require_non_empty(&contract.target.arch, "missing_target_arch")?;

    require_non_empty(
        &contract.constraints.requested_by,
        "missing_constraints_requested_by",
    )?;
    if contract.constraints.max_total_duration_ms == 0 {
        return Err(ValidationError::new("invalid_max_total_duration_ms"));
    }
    if contract.constraints.step_timeout_default_ms == 0 {
        return Err(ValidationError::new("invalid_step_timeout_default_ms"));
    }
    if contract.constraints.step_timeout_default_ms > contract.constraints.max_total_duration_ms {
        return Err(ValidationError::new("step_timeout_exceeds_total_duration"));
    }
    require_non_empty(
        &contract.constraints.execution_profile,
        "missing_execution_profile",
    )?;

    require_non_empty(&contract.program.entry, "missing_program_entry")?;
    if contract.program.steps.is_empty() {
        return Err(ValidationError::new("missing_program_steps"));
    }

    let mut step_ids = BTreeSet::new();
    let mut entry_found = false;
    for step in &contract.program.steps {
        require_non_empty(&step.id, "missing_step_id")?;
        require_non_empty(&step.kind, "missing_step_kind")?;
        if !is_known_step_kind(&step.kind) {
            return Err(ValidationError::new("invalid_step_kind"));
        }

        if step.kind == STEP_KIND_INVOKE {
            let op = step.op.as_deref().unwrap_or_default();
            require_non_empty(op, "missing_invoke_op")?;
        }

        // `args` 只对 `invoke` 有意义：必须是 JSON 对象或 `null`。
        // 非 `invoke` 的 step 带 `args` 视为非法 —— 它的语义没人定义，接受它只会让
        // 计划里出现“看着有参数、实际被忽略”的死字段。
        match step.args.as_ref() {
            None => {}
            Some(args) if step.kind == STEP_KIND_INVOKE => {
                if !(args.is_object() || args.is_null()) {
                    return Err(ValidationError::new("invalid_step_args"));
                }
            }
            Some(_) => return Err(ValidationError::new("invalid_step_args")),
        }

        if !step_ids.insert(step.id.as_str()) {
            return Err(ValidationError::new("duplicate_step_id"));
        }
        if step.id == contract.program.entry {
            entry_found = true;
        }
    }

    if !entry_found {
        return Err(ValidationError::new("program_entry_not_found"));
    }

    validate_attestation(contract)?;

    Ok(())
}

/// 签发证明的**形状**校验。
///
/// 这里**不验签**：验签需要信任锚，那是执行侧（agentd）的事，而且必须由本地策略
/// 决定“要不要验”。契约层只保证“拿到的东西长得像一份完整的证明”。
fn validate_attestation(contract: &ActionPlan) -> Result<(), ValidationError> {
    let Some(attestation) = contract.attestation.as_ref() else {
        return Ok(());
    };
    if attestation.api_version != API_VERSION_V1 {
        return Err(ValidationError::new("invalid_attestation_api_version"));
    }
    // `plan-v1:sha256:<64 hex>`：前缀、长度、hex 三件都要成立。
    //
    // 只查前缀与长度会让 `plan-v1:sha256:` + `"z"×64` 也过，与下面签名的口径
    // （`is_ascii_hexdigit`）不一致。用 `strip_prefix` 而不是按字节切片：
    // 未经校验的 UTF-8 在非字符边界上切会 panic（这是对外部报文的校验，不能有这种面）。
    let digest_hex = attestation
        .plan_digest
        .strip_prefix(&format!("{PLAN_DIGEST_VERSION}:sha256:"));
    let digest_is_well_formed = digest_hex
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()));
    if !digest_is_well_formed {
        return Err(ValidationError::new("invalid_attestation_plan_digest"));
    }
    if attestation.algorithm != ATTESTATION_ALGORITHM_ED25519 {
        return Err(ValidationError::new("unsupported_attestation_algorithm"));
    }
    // Ed25519 签名 = 64 字节 = 128 个小写 hex 字符。
    if attestation.signature.len() != 128
        || !attestation
            .signature
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(ValidationError::new("invalid_attestation_signature"));
    }
    require_non_empty(&attestation.key_id, "missing_attestation_key_id")?;
    require_non_empty(&attestation.issued_by, "missing_attestation_issued_by")?;
    require_non_empty(
        &attestation.approval.approval_id,
        "missing_attestation_approval_id",
    )?;
    if attestation.approval.required_approvers == 0 {
        return Err(ValidationError::new("invalid_required_approvers"));
    }
    if attestation.approval.approvers.is_empty()
        || attestation.approval.approvers.len()
            < usize::from(attestation.approval.required_approvers)
    {
        return Err(ValidationError::new("insufficient_approvers"));
    }
    let mut approvers = BTreeSet::new();
    for approver in &attestation.approval.approvers {
        require_non_empty(approver, "missing_approver")?;
        if !approvers.insert(approver.as_str()) {
            return Err(ValidationError::new("duplicate_approver"));
        }
    }

    let signed_at = parse_rfc3339(&attestation.signed_at, "invalid_attestation_signed_at")?;
    // 审批时间与其它时间戳同口径：形状校验就得能读，否则下游按时间排序/判窗时会默默把它当 0。
    parse_rfc3339(
        &attestation.approval.approved_at,
        "invalid_attestation_approved_at",
    )?;
    let expires_at = parse_rfc3339(&attestation.expires_at, "invalid_attestation_expires_at")?;
    if expires_at <= signed_at {
        return Err(ValidationError::new("invalid_attestation_window"));
    }
    // 证明不能比计划本身活得更久：两者取更短的那个有效期才有意义。
    let plan_expires = parse_rfc3339(&contract.meta.expires_at, "invalid_expires_at")?;
    if expires_at > plan_expires {
        return Err(ValidationError::new("attestation_outlives_plan"));
    }

    // 计划声明的审批引用必须就是被签的那张审批单。
    if let Some(reference) = contract.constraints.approval_ref.as_deref()
        && reference != attestation.approval.approval_id
    {
        return Err(ValidationError::new("approval_ref_mismatch"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use wist_contracts::action_plan::{
        ActionPlanConstraints, ActionPlanMeta, ActionPlanProgram, ActionPlanStep, ActionPlanTarget,
        ApprovalMode, RiskLevel,
    };

    use super::*;

    /// 校验用例：期望错误码 + 对合法样本的破坏性修改。
    type Case<T> = (&'static str, fn(&mut T));

    fn base_plan() -> ActionPlan {
        ActionPlan::new(
            ActionPlanMeta {
                action_id: "act_001".to_string(),
                request_id: "req_001".to_string(),
                template_id: None,
                tenant_id: "tenant_a".to_string(),
                environment_id: "prod".to_string(),
                plan_version: 1,
                compiled_at: "2026-04-12T10:00:00Z".to_string(),
                expires_at: "2026-04-12T10:05:00Z".to_string(),
            },
            ActionPlanTarget {
                agent_id: "agent-001".to_string(),
                instance_id: None,
                node_id: "node-001".to_string(),
                host_name: None,
                platform: "linux".to_string(),
                arch: "amd64".to_string(),
                selectors: BTreeMap::new(),
            },
            ActionPlanConstraints {
                risk_level: RiskLevel::R1,
                approval_ref: None,
                approval_mode: ApprovalMode::NotRequired,
                requested_by: "alice@example.com".to_string(),
                reason: None,
                max_total_duration_ms: 30_000,
                step_timeout_default_ms: 10_000,
                execution_profile: "agent_exec_v1".to_string(),
                required_capabilities: Vec::new(),
                allow: None,
            },
            ActionPlanProgram {
                entry: "step_run".to_string(),
                steps: vec![
                    ActionPlanStep {
                        id: "step_run".to_string(),
                        kind: STEP_KIND_INVOKE.to_string(),
                        op: Some("process.list".to_string()),
                        args: None,
                    },
                    ActionPlanStep {
                        id: "step_output".to_string(),
                        kind: "output".to_string(),
                        op: None,
                        args: None,
                    },
                ],
            },
        )
    }

    fn assert_code(result: Result<(), ValidationError>, expected: &'static str) {
        let err = result.expect_err("expected a validation error");
        assert_eq!(err.code, expected);
    }

    /// 「把一份合法计划改成某种违纪」的用例形状：原因码 + 变换函数。
    ///
    /// 写成别名是因为这个元组类型在用例表里重复出现，直接铺开会触发
    /// `clippy::type_complexity`（CI 是 `-D warnings`）。
    type MutationCase = (&'static str, fn(&mut ActionPlan));

    #[test]
    fn a_well_formed_plan_passes() {
        validate_action_plan(&base_plan()).expect("valid plan");
    }

    #[test]
    fn header_and_meta_errors_have_stable_codes() {
        let cases: &[Case<ActionPlan>] = &[
            ("invalid_api_version", |plan| {
                plan.api_version = "v2".to_string()
            }),
            ("invalid_kind", |plan| plan.kind = "other".to_string()),
            ("missing_action_id", |plan| {
                plan.meta.action_id = "   ".to_string()
            }),
            ("missing_request_id", |plan| {
                plan.meta.request_id = String::new()
            }),
            ("missing_tenant_id", |plan| {
                plan.meta.tenant_id = "\t".to_string()
            }),
            ("missing_environment_id", |plan| {
                plan.meta.environment_id = String::new()
            }),
            ("invalid_plan_version", |plan| plan.meta.plan_version = 0),
        ];
        for &(expected, mutate) in cases {
            let mut plan = base_plan();
            mutate(&mut plan);
            assert_code(validate_action_plan(&plan), expected);
        }
    }

    #[test]
    fn timestamp_errors_have_stable_codes() {
        let mut plan = base_plan();
        plan.meta.compiled_at = "not-a-time".to_string();
        assert_code(validate_action_plan(&plan), "invalid_compiled_at");

        let mut plan = base_plan();
        plan.meta.expires_at = String::new();
        assert_code(validate_action_plan(&plan), "invalid_expires_at");
    }

    #[test]
    fn an_expired_or_non_increasing_window_is_rejected() {
        let mut plan = base_plan();
        plan.meta.expires_at = plan.meta.compiled_at.clone();
        assert_code(
            validate_action_plan(&plan),
            "expired_or_non_increasing_window",
        );

        let mut plan = base_plan();
        plan.meta.expires_at = "2026-04-12T09:59:59Z".to_string();
        assert_code(
            validate_action_plan(&plan),
            "expired_or_non_increasing_window",
        );
    }

    #[test]
    fn target_errors_have_stable_codes() {
        let cases: &[Case<ActionPlan>] = &[
            ("missing_target_agent_id", |plan| {
                plan.target.agent_id = " ".to_string()
            }),
            ("missing_target_node_id", |plan| {
                plan.target.node_id = String::new()
            }),
            ("missing_target_platform", |plan| {
                plan.target.platform = " ".to_string()
            }),
            ("missing_target_arch", |plan| {
                plan.target.arch = " ".to_string()
            }),
        ];
        for &(expected, mutate) in cases {
            let mut plan = base_plan();
            mutate(&mut plan);
            assert_code(validate_action_plan(&plan), expected);
        }
    }

    #[test]
    fn constraint_errors_have_stable_codes() {
        let cases: &[Case<ActionPlan>] = &[
            ("missing_constraints_requested_by", |plan| {
                plan.constraints.requested_by = " ".to_string()
            }),
            ("invalid_max_total_duration_ms", |plan| {
                plan.constraints.max_total_duration_ms = 0
            }),
            ("invalid_step_timeout_default_ms", |plan| {
                plan.constraints.step_timeout_default_ms = 0
            }),
            ("step_timeout_exceeds_total_duration", |plan| {
                plan.constraints.step_timeout_default_ms =
                    plan.constraints.max_total_duration_ms + 1
            }),
            ("missing_execution_profile", |plan| {
                plan.constraints.execution_profile = "\n".to_string()
            }),
        ];
        for &(expected, mutate) in cases {
            let mut plan = base_plan();
            mutate(&mut plan);
            assert_code(validate_action_plan(&plan), expected);
        }
    }

    #[test]
    fn step_timeout_equal_to_total_duration_is_allowed() {
        let mut plan = base_plan();
        plan.constraints.step_timeout_default_ms = plan.constraints.max_total_duration_ms;
        validate_action_plan(&plan).expect("boundary is inclusive");
    }

    #[test]
    fn program_errors_have_stable_codes() {
        let cases: &[Case<ActionPlan>] = &[
            ("missing_program_entry", |plan| {
                plan.program.entry = "   ".to_string()
            }),
            ("missing_program_steps", |plan| plan.program.steps.clear()),
            ("missing_step_id", |plan| {
                plan.program.steps[0].id = " ".to_string()
            }),
            ("missing_step_kind", |plan| {
                plan.program.steps[1].kind = String::new()
            }),
            ("invalid_step_kind", |plan| {
                plan.program.steps[0].kind = "shell".to_string()
            }),
            ("missing_invoke_op", |plan| plan.program.steps[0].op = None),
            ("missing_invoke_op", |plan| {
                plan.program.steps[0].op = Some("  ".to_string())
            }),
            ("program_entry_not_found", |plan| {
                plan.program.entry = "step_absent".to_string()
            }),
        ];
        for &(expected, mutate) in cases {
            let mut plan = base_plan();
            mutate(&mut plan);
            assert_code(validate_action_plan(&plan), expected);
        }
    }

    #[test]
    fn duplicate_step_id_is_rejected() {
        let mut plan = base_plan();
        plan.program.steps.push(ActionPlanStep {
            id: "step_run".to_string(),
            kind: "abort".to_string(),
            op: None,
            args: None,
        });
        assert_code(validate_action_plan(&plan), "duplicate_step_id");
    }

    #[test]
    fn a_non_invoke_step_may_omit_op() {
        // base_plan 的第二个 step 是 output 且 op 为 None，整单仍应通过。
        validate_action_plan(&base_plan()).expect("non-invoke steps do not need op");
    }

    #[test]
    fn invoke_args_must_be_object_or_null() {
        let mut plan = base_plan();
        plan.program.steps[0].args = Some(serde_json::json!({}));
        validate_action_plan(&plan).expect("empty object args are valid");

        plan.program.steps[0].args = Some(serde_json::Value::Null);
        validate_action_plan(&plan).expect("null args are valid");

        for bad in [
            serde_json::json!(3),
            serde_json::json!("x"),
            serde_json::json!([1]),
        ] {
            plan.program.steps[0].args = Some(bad);
            assert_code(validate_action_plan(&plan), "invalid_step_args");
        }
    }

    #[test]
    fn non_invoke_steps_may_not_carry_args() {
        let mut plan = base_plan();
        plan.program.steps[1].args = Some(serde_json::json!({ "a": 1 }));
        assert_code(validate_action_plan(&plan), "invalid_step_args");
    }

    #[test]
    fn entry_may_point_at_any_step() {
        let mut plan = base_plan();
        plan.program.entry = "step_output".to_string();
        validate_action_plan(&plan).expect("entry may reference any step");
    }

    #[test]
    fn entry_matching_is_exact_not_trimmed() {
        // id 不做规范化：entry 带首尾空格视为找不到对应 step。
        let mut plan = base_plan();
        plan.program.entry = " step_run ".to_string();
        assert_code(validate_action_plan(&plan), "program_entry_not_found");
    }

    #[test]
    fn extreme_unsigned_values_are_accepted() {
        let mut plan = base_plan();
        // `plan_version` 与中心模型一致为有符号 `Int`（i64），时长上限仍为无符号毫秒。
        plan.meta.plan_version = i64::MAX;
        plan.constraints.max_total_duration_ms = u64::MAX;
        plan.constraints.step_timeout_default_ms = u64::MAX;
        validate_action_plan(&plan).expect("extreme boundary values are valid");
    }

    fn attested_plan() -> ActionPlan {
        use wist_contracts::action_plan::{ActionPlanApproval, ActionPlanAttestation};
        let mut plan = base_plan();
        plan.meta.expires_at = "2031-01-01T00:00:00Z".to_string();
        plan.constraints.approval_ref = Some("apv-1".to_string());
        plan.constraints.approval_mode = ApprovalMode::Required;
        plan.attestation = Some(ActionPlanAttestation {
            api_version: API_VERSION_V1.to_string(),
            plan_digest: format!("{PLAN_DIGEST_VERSION}:sha256:{}", "a".repeat(64)),
            approval: ActionPlanApproval {
                approval_id: "apv-1".to_string(),
                approvers: vec!["user:alice".to_string()],
                required_approvers: 1,
                approved_at: "2026-11-22T00:00:00Z".to_string(),
            },
            risk_level: RiskLevel::R1,
            issued_by: "gateway".to_string(),
            key_id: "k1".to_string(),
            algorithm: ATTESTATION_ALGORITHM_ED25519.to_string(),
            signed_at: "2026-11-22T00:00:00Z".to_string(),
            expires_at: "2026-11-22T00:15:00Z".to_string(),
            signature: "b".repeat(128),
        });
        plan
    }

    #[test]
    fn a_well_formed_attestation_passes_shape_checks() {
        validate_action_plan(&attested_plan()).expect("shape only, no signature check");
    }

    #[test]
    fn attestation_shape_failures_are_distinct_codes() {
        let cases: Vec<MutationCase> = vec![
            ("invalid_attestation_signature", |plan| {
                plan.attestation.as_mut().unwrap().signature = "zz".to_string();
            }),
            ("unsupported_attestation_algorithm", |plan| {
                plan.attestation.as_mut().unwrap().algorithm = "rsa".to_string();
            }),
            ("invalid_attestation_plan_digest", |plan| {
                plan.attestation.as_mut().unwrap().plan_digest = "abc".to_string();
            }),
            ("invalid_attestation_plan_digest", |plan| {
                // 前缀与长度都对、但尾巴 64 位不是 hex：与签名同口径，必须拒。
                plan.attestation.as_mut().unwrap().plan_digest =
                    format!("{PLAN_DIGEST_VERSION}:sha256:{}", "z".repeat(64));
            }),
            ("insufficient_approvers", |plan| {
                let attestation = plan.attestation.as_mut().unwrap();
                attestation.approval.required_approvers = 2;
            }),
            ("duplicate_approver", |plan| {
                let attestation = plan.attestation.as_mut().unwrap();
                attestation.approval.approvers =
                    vec!["user:alice".to_string(), "user:alice".to_string()];
                attestation.approval.required_approvers = 2;
            }),
            ("attestation_outlives_plan", |plan| {
                plan.meta.expires_at = "2026-11-22T00:10:00Z".to_string();
            }),
            ("approval_ref_mismatch", |plan| {
                plan.constraints.approval_ref = Some("apv-other".to_string());
            }),
            ("missing_attestation_key_id", |plan| {
                plan.attestation.as_mut().unwrap().key_id = String::new();
            }),
        ];
        for (code, mutate) in cases {
            let mut plan = attested_plan();
            mutate(&mut plan);
            assert_code(validate_action_plan(&plan), code);
        }
    }

    #[test]
    fn more_attestation_shape_failures_have_distinct_codes() {
        let cases: Vec<MutationCase> = vec![
            ("invalid_attestation_api_version", |plan| {
                plan.attestation.as_mut().unwrap().api_version = "v2".to_string();
            }),
            ("invalid_required_approvers", |plan| {
                plan.attestation
                    .as_mut()
                    .unwrap()
                    .approval
                    .required_approvers = 0;
            }),
            ("missing_approver", |plan| {
                let attestation = plan.attestation.as_mut().unwrap();
                attestation.approval.approvers = vec![String::new()];
                attestation.approval.required_approvers = 1;
            }),
            ("missing_attestation_issued_by", |plan| {
                plan.attestation.as_mut().unwrap().issued_by = String::new();
            }),
            ("missing_attestation_approval_id", |plan| {
                plan.attestation.as_mut().unwrap().approval.approval_id = String::new();
            }),
            ("invalid_attestation_signed_at", |plan| {
                plan.attestation.as_mut().unwrap().signed_at = "not-a-time".to_string();
            }),
            ("invalid_attestation_expires_at", |plan| {
                plan.attestation.as_mut().unwrap().expires_at = "not-a-time".to_string();
            }),
            ("invalid_attestation_approved_at", |plan| {
                // 审批时间是唯一一个以前没校验的时间戳：其它三个都能读，它也得能读。
                plan.attestation.as_mut().unwrap().approval.approved_at = "not-a-time".to_string();
            }),
            ("invalid_attestation_window", |plan| {
                // signed_at 晚于 expires_at：窗口本身就是错的。
                let attestation = plan.attestation.as_mut().unwrap();
                attestation.signed_at = "2026-11-22T01:00:00Z".to_string();
                attestation.expires_at = "2026-11-22T00:00:00Z".to_string();
            }),
        ];
        for (code, mutate) in cases {
            let mut plan = attested_plan();
            mutate(&mut plan);
            assert_code(validate_action_plan(&plan), code);
        }
    }

    #[test]
    fn a_plan_without_attestation_is_still_valid_shape_wise() {
        // 要不要强制带 attestation 由执行侧策略决定，不是契约层的事。
        validate_action_plan(&base_plan()).expect("unattested plan is structurally fine");
    }

    /// 窗口的两个边界各自不同：证明可以和计划**同时**到期（只有“更晚”才是越界），
    /// 但自己的窗口必须**严格**递增 —— `signed_at == expires_at` 是一个空窗口。
    #[test]
    fn attestation_window_boundaries_are_strict_and_inclusive_respectively() {
        let mut plan = attested_plan();
        let plan_expires = plan.meta.expires_at.clone();
        plan.attestation.as_mut().expect("attestation").expires_at = plan_expires;
        validate_action_plan(&plan).expect("证明与计划同时到期是允许的（取更短的那个）");

        let mut plan = attested_plan();
        {
            let attestation = plan.attestation.as_mut().expect("attestation");
            attestation.expires_at = attestation.signed_at.clone();
        }
        assert_code(validate_action_plan(&plan), "invalid_attestation_window");
    }

    /// `approval_ref` 是**引用**，按字面量匹配：带首尾空白就是另一张审批单。
    /// 与 `entry_matching_is_exact_not_trimmed` 同一条规矩（这里不做规范化）。
    #[test]
    fn approval_ref_matching_is_exact_not_trimmed() {
        let mut plan = attested_plan();
        plan.constraints.approval_ref = Some(" apv-1 ".to_string());
        assert_code(validate_action_plan(&plan), "approval_ref_mismatch");
    }

    /// 这层只判**窗口形状**（`expires_at > compiled_at`），不判“现在是不是已经过期”：
    /// 计划还能不能执行由执行侧按自己的时钟决定。
    ///
    /// 错误码叫 `expired_or_non_increasing_window`，但**没有**与当前时间比较 ——
    /// 这条用例把这个语义钉住（改成立即拒过去窗口，审计/回放的样例会随日期变红）。
    #[test]
    fn expiry_is_a_window_check_not_a_wall_clock_check() {
        let mut plan = base_plan();
        plan.meta.compiled_at = "2000-01-01T00:00:00Z".to_string();
        plan.meta.expires_at = "2000-01-01T00:05:00Z".to_string();
        validate_action_plan(&plan).expect("已经过去的时间窗口仍是一个合法窗口");
    }
}
