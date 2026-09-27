//! `ActionPlan` validation entrypoints.

use std::collections::BTreeSet;

use wist_contracts::API_VERSION_V1;
use wist_contracts::action_plan::{
    ACTION_PLAN_KIND, ActionPlan, STEP_KIND_INVOKE, is_known_step_kind,
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
            },
            ActionPlanProgram {
                entry: "step_run".to_string(),
                steps: vec![
                    ActionPlanStep {
                        id: "step_run".to_string(),
                        kind: STEP_KIND_INVOKE.to_string(),
                        op: Some("process.list".to_string()),
                    },
                    ActionPlanStep {
                        id: "step_output".to_string(),
                        kind: "output".to_string(),
                        op: None,
                    },
                ],
            },
        )
    }

    fn assert_code(result: Result<(), ValidationError>, expected: &'static str) {
        let err = result.expect_err("expected a validation error");
        assert_eq!(err.code, expected);
    }

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
        });
        assert_code(validate_action_plan(&plan), "duplicate_step_id");
    }

    #[test]
    fn a_non_invoke_step_may_omit_op() {
        // base_plan 的第二个 step 是 output 且 op 为 None，整单仍应通过。
        validate_action_plan(&base_plan()).expect("non-invoke steps do not need op");
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
}
