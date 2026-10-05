//! Gateway envelope validation entrypoints.

use wist_api::gateway::{
    ACTION_PLAN_ACK_KIND, AckStatus, ActionPlanAck, DISPATCH_ACTION_PLAN_KIND, DispatchActionPlan,
    REPORT_ACTION_RESULT_KIND, ReportActionResult,
};
use wist_contracts::API_VERSION_V1;

use crate::action_plan::validate_action_plan;
use crate::action_result::validate_action_result;
use crate::{ValidationError, parse_rfc3339, require_non_empty};

pub fn validate_dispatch_action_plan(contract: &DispatchActionPlan) -> Result<(), ValidationError> {
    if contract.api_version != API_VERSION_V1 {
        return Err(ValidationError::new("invalid_api_version"));
    }
    if contract.kind != DISPATCH_ACTION_PLAN_KIND {
        return Err(ValidationError::new("invalid_kind"));
    }
    require_non_empty(&contract.dispatch_id, "missing_dispatch_id")?;
    validate_action_plan(&contract.plan)?;
    Ok(())
}

pub fn validate_action_plan_ack(contract: &ActionPlanAck) -> Result<(), ValidationError> {
    if contract.api_version != API_VERSION_V1 {
        return Err(ValidationError::new("invalid_api_version"));
    }
    if contract.kind != ACTION_PLAN_ACK_KIND {
        return Err(ValidationError::new("invalid_kind"));
    }

    require_non_empty(&contract.dispatch_id, "missing_dispatch_id")?;
    require_non_empty(&contract.action_id, "missing_action_id")?;
    require_non_empty(&contract.plan_digest, "missing_plan_digest")?;
    require_non_empty(&contract.agent_id, "missing_agent_id")?;
    require_non_empty(&contract.instance_id, "missing_instance_id")?;

    let received_at = parse_rfc3339(&contract.received_at, "invalid_received_at")?;
    let acknowledged_at = parse_rfc3339(&contract.acknowledged_at, "invalid_acknowledged_at")?;
    if acknowledged_at < received_at {
        return Err(ValidationError::new("acknowledged_before_received"));
    }

    if let Some(reason_code) = &contract.reason_code {
        require_non_empty(reason_code, "invalid_reason_code")?;
    }
    if let Some(reason_message) = &contract.reason_message {
        require_non_empty(reason_message, "invalid_reason_message")?;
    }

    match contract.ack_status {
        AckStatus::Accepted => {
            let execution_id = contract.execution_id.as_deref().unwrap_or_default();
            require_non_empty(execution_id, "missing_execution_id")?;
            if contract.queue_position.is_some() {
                return Err(ValidationError::new(
                    "queue_position_not_allowed_for_accepted",
                ));
            }
        }
        AckStatus::Queued => {
            let execution_id = contract.execution_id.as_deref().unwrap_or_default();
            require_non_empty(execution_id, "missing_execution_id")?;
            if contract.queue_position.is_none() {
                return Err(ValidationError::new("missing_queue_position"));
            }
        }
        AckStatus::Rejected | AckStatus::Duplicate | AckStatus::Stale | AckStatus::Busy => {
            if contract.queue_position.is_some() {
                return Err(ValidationError::new(
                    "queue_position_only_allowed_for_queued",
                ));
            }
        }
    }

    Ok(())
}

pub fn validate_report_action_result(contract: &ReportActionResult) -> Result<(), ValidationError> {
    if contract.api_version != API_VERSION_V1 {
        return Err(ValidationError::new("invalid_api_version"));
    }
    if contract.kind != REPORT_ACTION_RESULT_KIND {
        return Err(ValidationError::new("invalid_kind"));
    }

    require_non_empty(&contract.report_id, "missing_report_id")?;
    if let Some(dispatch_id) = &contract.dispatch_id {
        require_non_empty(dispatch_id, "invalid_dispatch_id")?;
    }
    require_non_empty(&contract.action_id, "missing_action_id")?;
    require_non_empty(&contract.execution_id, "missing_execution_id")?;
    require_non_empty(&contract.plan_digest, "missing_plan_digest")?;
    require_non_empty(&contract.agent_id, "missing_agent_id")?;
    require_non_empty(&contract.instance_id, "missing_instance_id")?;
    if contract.report_attempt == 0 {
        return Err(ValidationError::new("invalid_report_attempt"));
    }
    let reported_at = parse_rfc3339(&contract.reported_at, "invalid_reported_at")?;

    require_non_empty(
        &contract.result_attestation.result_digest,
        "missing_result_digest",
    )?;
    require_non_empty(&contract.result_attestation.signature, "missing_signature")?;
    require_non_empty(&contract.result_attestation.issued_by, "missing_issued_by")?;
    let attested_at = parse_rfc3339(
        &contract.result_attestation.attested_at,
        "invalid_attested_at",
    )?;
    if attested_at > reported_at {
        return Err(ValidationError::new("attested_after_reported"));
    }

    validate_action_result(&contract.result)?;
    if contract.final_status != contract.result.final_status {
        return Err(ValidationError::new("mismatched_final_status"));
    }
    if contract.action_id != contract.result.action_id {
        return Err(ValidationError::new("mismatched_action_id"));
    }
    if contract.execution_id != contract.result.execution_id {
        return Err(ValidationError::new("mismatched_execution_id"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use wist_api::gateway::ResultAttestation;
    use wist_contracts::action_plan::ActionPlan;
    use wist_contracts::action_result::{ActionResult, FinalStatus, StepRecord, StepStatus};

    use super::*;

    /// 校验用例：期望错误码 + 对合法样本的破坏性修改。
    type Case<T> = (&'static str, fn(&mut T));

    fn sample_plan() -> ActionPlan {
        serde_json::from_str(include_str!(
            "../fixtures/contracts/action-plan/valid/basic.json"
        ))
        .expect("decode action plan fixture")
    }

    fn sample_result() -> ActionResult {
        let mut result = ActionResult::new(
            "act_001".to_string(),
            "exec_001".to_string(),
            FinalStatus::Succeeded,
        );
        result.step_records = vec![StepRecord {
            step_id: "step_run".to_string(),
            attempt: 1,
            op: Some("process.list".to_string()),
            status: StepStatus::Succeeded,
            started_at: "2026-04-12T10:00:00Z".to_string(),
            finished_at: Some("2026-04-12T10:00:01Z".to_string()),
            duration_ms: Some(1000),
            error_code: None,
            stdout_summary: None,
            stderr_summary: None,
            resource_usage: None,
        }];
        result
    }

    fn base_ack(ack_status: AckStatus) -> ActionPlanAck {
        ActionPlanAck {
            api_version: API_VERSION_V1.to_string(),
            kind: ACTION_PLAN_ACK_KIND.to_string(),
            dispatch_id: "dsp_001".to_string(),
            action_id: "act_001".to_string(),
            plan_digest: "sha256:plan".to_string(),
            agent_id: "agent-001".to_string(),
            instance_id: "instance-001".to_string(),
            execution_id: Some("exec_001".to_string()),
            ack_status,
            reason_code: None,
            reason_message: None,
            queue_position: None,
            received_at: "2026-04-12T10:00:00Z".to_string(),
            acknowledged_at: "2026-04-12T10:00:01Z".to_string(),
        }
    }

    fn sample_report() -> ReportActionResult {
        ReportActionResult::new(
            "rep_001".to_string(),
            "act_001".to_string(),
            1,
            FinalStatus::Succeeded,
            "exec_001".to_string(),
            "sha256:plan".to_string(),
            "agent-001".to_string(),
            "instance-001".to_string(),
            ResultAttestation {
                result_digest: "sha256:res".to_string(),
                signature: "sig".to_string(),
                issued_by: "dev-placeholder:agent-001".to_string(),
                attested_at: "2026-04-12T10:00:02Z".to_string(),
            },
            "2026-04-12T10:00:03Z".to_string(),
            sample_result(),
        )
    }

    fn assert_code(result: Result<(), ValidationError>, expected: &'static str) {
        let err = result.expect_err("expected a validation error");
        assert_eq!(err.code, expected);
    }

    #[test]
    fn a_well_formed_dispatch_passes() {
        let dispatch = DispatchActionPlan::new("dsp_001".to_string(), sample_plan());
        validate_dispatch_action_plan(&dispatch).expect("valid dispatch");
    }

    #[test]
    fn dispatch_header_and_identity_errors_have_stable_codes() {
        let cases: &[Case<DispatchActionPlan>] = &[
            ("invalid_api_version", |dispatch| {
                dispatch.api_version = "v2".to_string()
            }),
            ("invalid_kind", |dispatch| {
                dispatch.kind = "action_plan".to_string()
            }),
            ("missing_dispatch_id", |dispatch| {
                dispatch.dispatch_id = " ".to_string()
            }),
            ("invalid_kind", |dispatch| {
                dispatch.plan.kind = "other".to_string()
            }),
        ];
        for &(expected, mutate) in cases {
            let mut dispatch = DispatchActionPlan::new("dsp_001".to_string(), sample_plan());
            mutate(&mut dispatch);
            assert_code(validate_dispatch_action_plan(&dispatch), expected);
        }
    }

    #[test]
    fn accepted_and_queued_acks_pass_with_their_required_fields() {
        validate_action_plan_ack(&base_ack(AckStatus::Accepted)).expect("accepted ack");

        let mut queued = base_ack(AckStatus::Queued);
        queued.queue_position = Some(1);
        validate_action_plan_ack(&queued).expect("queued ack");
    }

    #[test]
    fn terminal_ack_statuses_do_not_require_queue_position() {
        for status in [
            AckStatus::Rejected,
            AckStatus::Duplicate,
            AckStatus::Stale,
            AckStatus::Busy,
        ] {
            validate_action_plan_ack(&base_ack(status))
                .unwrap_or_else(|err| panic!("{status:?} should pass, got {}", err.code));
        }
    }

    #[test]
    fn ack_header_and_identity_errors_have_stable_codes() {
        let cases: &[Case<ActionPlanAck>] = &[
            ("invalid_api_version", |ack| {
                ack.api_version = "v2".to_string()
            }),
            ("invalid_kind", |ack| ack.kind = "other".to_string()),
            ("missing_dispatch_id", |ack| {
                ack.dispatch_id = " ".to_string()
            }),
            ("missing_action_id", |ack| ack.action_id = String::new()),
            ("missing_plan_digest", |ack| {
                ack.plan_digest = " ".to_string()
            }),
            ("missing_agent_id", |ack| ack.agent_id = String::new()),
            ("missing_instance_id", |ack| {
                ack.instance_id = " ".to_string()
            }),
        ];
        for &(expected, mutate) in cases {
            let mut ack = base_ack(AckStatus::Accepted);
            mutate(&mut ack);
            assert_code(validate_action_plan_ack(&ack), expected);
        }
    }

    #[test]
    fn ack_timestamp_errors_have_stable_codes() {
        let mut ack = base_ack(AckStatus::Accepted);
        ack.received_at = "nope".to_string();
        assert_code(validate_action_plan_ack(&ack), "invalid_received_at");

        let mut ack = base_ack(AckStatus::Accepted);
        ack.acknowledged_at = String::new();
        assert_code(validate_action_plan_ack(&ack), "invalid_acknowledged_at");

        let mut ack = base_ack(AckStatus::Accepted);
        ack.acknowledged_at = "2026-04-12T09:59:59Z".to_string();
        assert_code(
            validate_action_plan_ack(&ack),
            "acknowledged_before_received",
        );

        let mut ack = base_ack(AckStatus::Accepted);
        ack.acknowledged_at = ack.received_at.clone();
        validate_action_plan_ack(&ack).expect("equal timestamps are allowed");
    }

    #[test]
    fn blank_reason_fields_are_rejected_when_present() {
        let mut ack = base_ack(AckStatus::Rejected);
        ack.reason_code = Some(" ".to_string());
        assert_code(validate_action_plan_ack(&ack), "invalid_reason_code");

        let mut ack = base_ack(AckStatus::Rejected);
        ack.reason_message = Some(String::new());
        assert_code(validate_action_plan_ack(&ack), "invalid_reason_message");

        let mut ack = base_ack(AckStatus::Rejected);
        ack.reason_code = Some("target_mismatch".to_string());
        ack.reason_message = Some("agent id didn't match".to_string());
        validate_action_plan_ack(&ack).expect("populated reason fields are valid");
    }

    #[test]
    fn accepted_ack_requires_execution_id_and_forbids_queue_position() {
        let mut ack = base_ack(AckStatus::Accepted);
        ack.execution_id = None;
        assert_code(validate_action_plan_ack(&ack), "missing_execution_id");

        let mut ack = base_ack(AckStatus::Accepted);
        ack.execution_id = Some("  ".to_string());
        assert_code(validate_action_plan_ack(&ack), "missing_execution_id");

        let mut ack = base_ack(AckStatus::Accepted);
        ack.queue_position = Some(1);
        assert_code(
            validate_action_plan_ack(&ack),
            "queue_position_not_allowed_for_accepted",
        );
    }

    #[test]
    fn queued_ack_requires_execution_id_and_queue_position() {
        let mut ack = base_ack(AckStatus::Queued);
        ack.execution_id = None;
        assert_code(validate_action_plan_ack(&ack), "missing_execution_id");

        let mut ack = base_ack(AckStatus::Queued);
        ack.queue_position = None;
        assert_code(validate_action_plan_ack(&ack), "missing_queue_position");
    }

    #[test]
    fn queue_position_is_only_allowed_for_queued() {
        for status in [
            AckStatus::Rejected,
            AckStatus::Duplicate,
            AckStatus::Stale,
            AckStatus::Busy,
        ] {
            let mut ack = base_ack(status);
            ack.queue_position = Some(0);
            assert_code(
                validate_action_plan_ack(&ack),
                "queue_position_only_allowed_for_queued",
            );
        }
    }

    #[test]
    fn a_well_formed_report_passes() {
        validate_report_action_result(&sample_report()).expect("valid report");
    }

    #[test]
    fn report_header_envelope_errors_have_stable_codes() {
        let cases: &[Case<ReportActionResult>] = &[
            ("invalid_api_version", |report| {
                report.api_version = "v2".to_string()
            }),
            ("invalid_kind", |report| {
                report.kind = "action_result".to_string()
            }),
            ("missing_report_id", |report| {
                report.report_id = " ".to_string()
            }),
            ("invalid_dispatch_id", |report| {
                report.dispatch_id = Some(String::new())
            }),
            ("missing_action_id", |report| {
                report.action_id = " ".to_string()
            }),
            ("missing_execution_id", |report| {
                report.execution_id = String::new()
            }),
            ("missing_plan_digest", |report| {
                report.plan_digest = " ".to_string()
            }),
            ("missing_agent_id", |report| report.agent_id = String::new()),
            ("missing_instance_id", |report| {
                report.instance_id = " ".to_string()
            }),
            ("invalid_report_attempt", |report| report.report_attempt = 0),
            ("invalid_reported_at", |report| {
                report.reported_at = "nope".to_string()
            }),
        ];
        for &(expected, mutate) in cases {
            let mut report = sample_report();
            mutate(&mut report);
            assert_code(validate_report_action_result(&report), expected);
        }
    }

    #[test]
    fn a_present_dispatch_id_must_not_be_blank() {
        let mut report = sample_report();
        report.dispatch_id = Some("dsp_001".to_string());
        validate_report_action_result(&report).expect("non-blank dispatch_id is fine");
    }

    #[test]
    fn report_attestation_errors_have_stable_codes() {
        let cases: &[Case<ReportActionResult>] = &[
            ("missing_result_digest", |report| {
                report.result_attestation.result_digest = " ".to_string()
            }),
            ("missing_signature", |report| {
                report.result_attestation.signature = String::new()
            }),
            ("missing_issued_by", |report| {
                report.result_attestation.issued_by = " ".to_string()
            }),
            ("invalid_attested_at", |report| {
                report.result_attestation.attested_at = "nope".to_string()
            }),
            ("attested_after_reported", |report| {
                report.result_attestation.attested_at = "2026-04-12T10:00:04Z".to_string()
            }),
        ];
        for &(expected, mutate) in cases {
            let mut report = sample_report();
            mutate(&mut report);
            assert_code(validate_report_action_result(&report), expected);
        }
    }

    #[test]
    fn attestation_equal_to_reported_at_is_allowed() {
        let mut report = sample_report();
        report.result_attestation.attested_at = report.reported_at.clone();
        validate_report_action_result(&report).expect("equal timestamps are allowed");
    }

    #[test]
    fn report_propagates_inner_result_validation() {
        let mut report = sample_report();
        report.result.api_version = "v2".to_string();
        assert_code(
            validate_report_action_result(&report),
            "invalid_api_version",
        );
    }

    #[test]
    fn report_requires_consistency_with_inner_result() {
        let mut report = sample_report();
        report.final_status = FinalStatus::Failed;
        assert_code(
            validate_report_action_result(&report),
            "mismatched_final_status",
        );

        let mut report = sample_report();
        report.action_id = "act_other".to_string();
        assert_code(
            validate_report_action_result(&report),
            "mismatched_action_id",
        );

        let mut report = sample_report();
        report.execution_id = "exec_other".to_string();
        assert_code(
            validate_report_action_result(&report),
            "mismatched_execution_id",
        );
    }
}
