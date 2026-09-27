//! `ActionResult` validation entrypoints.

use wist_contracts::API_VERSION_V1;
use wist_contracts::action_result::{ACTION_RESULT_KIND, ActionResult, FinalStatus, StepStatus};

use crate::{ValidationError, parse_rfc3339, require_non_empty};

pub fn validate_action_result(contract: &ActionResult) -> Result<(), ValidationError> {
    if contract.api_version != API_VERSION_V1 {
        return Err(ValidationError::new("invalid_api_version"));
    }
    if contract.kind != ACTION_RESULT_KIND {
        return Err(ValidationError::new("invalid_kind"));
    }

    require_non_empty(&contract.action_id, "missing_action_id")?;
    require_non_empty(&contract.execution_id, "missing_execution_id")?;
    if let Some(request_id) = &contract.request_id {
        require_non_empty(request_id, "invalid_request_id")?;
    }
    if contract.step_records.is_empty() {
        return Err(ValidationError::new("missing_step_records"));
    }

    let started_at = contract
        .started_at
        .as_deref()
        .map(|value| parse_rfc3339(value, "invalid_started_at"))
        .transpose()?;
    let finished_at = contract
        .finished_at
        .as_deref()
        .map(|value| parse_rfc3339(value, "invalid_finished_at"))
        .transpose()?;
    if let (Some(started_at), Some(finished_at)) = (started_at, finished_at)
        && finished_at < started_at
    {
        return Err(ValidationError::new("finished_before_started"));
    }

    for step in &contract.step_records {
        require_non_empty(&step.step_id, "missing_step_id")?;
        if step.attempt == 0 {
            return Err(ValidationError::new("invalid_step_attempt"));
        }

        let started_at = parse_rfc3339(&step.started_at, "invalid_step_started_at")?;
        if let Some(finished_at) = &step.finished_at {
            let finished_at = parse_rfc3339(finished_at, "invalid_step_finished_at")?;
            if finished_at < started_at {
                return Err(ValidationError::new("step_finished_before_started"));
            }
        }
    }

    for item in &contract.outputs.items {
        require_non_empty(&item.name, "missing_output_name")?;
    }

    validate_final_status_consistency(contract)?;

    Ok(())
}

fn validate_final_status_consistency(contract: &ActionResult) -> Result<(), ValidationError> {
    match contract.final_status {
        FinalStatus::Succeeded => {
            if contract.exit_reason.is_some() {
                return Err(ValidationError::new("succeeded_result_has_exit_reason"));
            }
            if contract
                .step_records
                .iter()
                .any(|step| !matches!(step.status, StepStatus::Succeeded | StepStatus::Skipped))
            {
                return Err(ValidationError::new(
                    "succeeded_result_has_non_success_step",
                ));
            }
        }
        FinalStatus::Rejected => {
            if contract
                .step_records
                .iter()
                .any(|step| matches!(step.status, StepStatus::Succeeded))
            {
                return Err(ValidationError::new("rejected_result_has_success_step"));
            }
        }
        FinalStatus::Failed => {
            if contract
                .step_records
                .iter()
                .all(|step| !matches!(step.status, StepStatus::Failed))
            {
                return Err(ValidationError::new("failed_result_has_no_failed_step"));
            }
        }
        FinalStatus::TimedOut => {
            if contract
                .step_records
                .iter()
                .all(|step| !matches!(step.status, StepStatus::TimedOut))
            {
                return Err(ValidationError::new(
                    "timed_out_result_has_no_timed_out_step",
                ));
            }
        }
        FinalStatus::Cancelled => {
            if contract
                .step_records
                .iter()
                .all(|step| !matches!(step.status, StepStatus::Cancelled))
            {
                return Err(ValidationError::new(
                    "cancelled_result_has_no_cancelled_step",
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use wist_contracts::action_result::{ActionOutputItem, StepRecord};

    use super::*;

    /// 校验用例：期望错误码 + 对合法样本的破坏性修改。
    type Case<T> = (&'static str, fn(&mut T));

    fn step_record(status: StepStatus) -> StepRecord {
        StepRecord {
            step_id: "step_run".to_string(),
            attempt: 1,
            op: Some("process.list".to_string()),
            status,
            started_at: "2026-04-12T10:00:00Z".to_string(),
            finished_at: Some("2026-04-12T10:00:01Z".to_string()),
            duration_ms: Some(1000),
            error_code: None,
            stdout_summary: None,
            stderr_summary: None,
            resource_usage: None,
        }
    }

    /// 构造一份与该 final_status 自洽的最小结果：
    /// 成功不带 exit_reason，其余状态带 exit_reason 并配一个对应步状态。
    fn base_result(final_status: FinalStatus) -> ActionResult {
        let mut result =
            ActionResult::new("act_001".to_string(), "exec_001".to_string(), final_status);
        result.request_id = Some("req_001".to_string());
        result.step_records = vec![step_record(match final_status {
            FinalStatus::Succeeded => StepStatus::Succeeded,
            FinalStatus::Failed => StepStatus::Failed,
            FinalStatus::TimedOut => StepStatus::TimedOut,
            FinalStatus::Cancelled => StepStatus::Cancelled,
            FinalStatus::Rejected => StepStatus::Skipped,
        })];
        result.started_at = Some("2026-04-12T10:00:00Z".to_string());
        result.finished_at = Some("2026-04-12T10:00:01Z".to_string());
        if final_status != FinalStatus::Succeeded {
            result.exit_reason = Some("reason".to_string());
        }
        result
    }

    fn assert_code(result: Result<(), ValidationError>, expected: &'static str) {
        let err = result.expect_err("expected a validation error");
        assert_eq!(err.code, expected);
    }

    #[test]
    fn each_self_consistent_final_status_passes() {
        for status in [
            FinalStatus::Succeeded,
            FinalStatus::Failed,
            FinalStatus::TimedOut,
            FinalStatus::Cancelled,
            FinalStatus::Rejected,
        ] {
            validate_action_result(&base_result(status))
                .unwrap_or_else(|err| panic!("{status:?} should pass, got {}", err.code));
        }
    }

    #[test]
    fn header_errors_have_stable_codes() {
        let cases: &[Case<ActionResult>] = &[
            ("invalid_api_version", |result| {
                result.api_version = "v2".to_string()
            }),
            ("invalid_kind", |result| {
                result.kind = "action_plan".to_string()
            }),
        ];
        for &(expected, mutate) in cases {
            let mut result = base_result(FinalStatus::Succeeded);
            mutate(&mut result);
            assert_code(validate_action_result(&result), expected);
        }
    }

    #[test]
    fn identity_errors_have_stable_codes() {
        let cases: &[Case<ActionResult>] = &[
            ("missing_action_id", |result| {
                result.action_id = " ".to_string()
            }),
            ("missing_execution_id", |result| {
                result.execution_id = String::new()
            }),
            ("invalid_request_id", |result| {
                result.request_id = Some("  ".to_string())
            }),
            ("missing_step_records", |result| result.step_records.clear()),
        ];
        for &(expected, mutate) in cases {
            let mut result = base_result(FinalStatus::Succeeded);
            mutate(&mut result);
            assert_code(validate_action_result(&result), expected);
        }
    }

    #[test]
    fn absent_request_id_is_allowed() {
        let mut result = base_result(FinalStatus::Succeeded);
        result.request_id = None;
        validate_action_result(&result).expect("request_id is optional");
    }

    #[test]
    fn top_level_timestamp_errors_have_stable_codes() {
        let mut result = base_result(FinalStatus::Succeeded);
        result.started_at = Some("nope".to_string());
        assert_code(validate_action_result(&result), "invalid_started_at");

        let mut result = base_result(FinalStatus::Succeeded);
        result.finished_at = Some(String::new());
        assert_code(validate_action_result(&result), "invalid_finished_at");
    }

    #[test]
    fn finished_before_started_is_rejected() {
        let mut result = base_result(FinalStatus::Succeeded);
        result.finished_at = Some("2026-04-12T09:59:59Z".to_string());
        assert_code(validate_action_result(&result), "finished_before_started");
    }

    #[test]
    fn equal_or_single_sided_timestamps_are_allowed() {
        let mut result = base_result(FinalStatus::Succeeded);
        result.finished_at = result.started_at.clone();
        validate_action_result(&result).expect("zero-length window is allowed");

        let mut result = base_result(FinalStatus::Succeeded);
        result.finished_at = None;
        validate_action_result(&result).expect("finished_at is optional");

        let mut result = base_result(FinalStatus::Succeeded);
        result.started_at = None;
        validate_action_result(&result).expect("started_at is optional");
    }

    #[test]
    fn step_record_errors_have_stable_codes() {
        let cases: &[Case<ActionResult>] = &[
            ("missing_step_id", |result| {
                result.step_records[0].step_id = " ".to_string()
            }),
            ("invalid_step_attempt", |result| {
                result.step_records[0].attempt = 0
            }),
            ("invalid_step_started_at", |result| {
                result.step_records[0].started_at = "nope".to_string()
            }),
            ("invalid_step_finished_at", |result| {
                result.step_records[0].finished_at = Some("nope".to_string())
            }),
            ("step_finished_before_started", |result| {
                result.step_records[0].finished_at = Some("2026-04-12T09:59:59Z".to_string())
            }),
        ];
        for &(expected, mutate) in cases {
            let mut result = base_result(FinalStatus::Succeeded);
            mutate(&mut result);
            assert_code(validate_action_result(&result), expected);
        }
    }

    #[test]
    fn output_item_name_must_not_be_blank() {
        let mut result = base_result(FinalStatus::Succeeded);
        result.outputs.items.push(ActionOutputItem {
            name: "  ".to_string(),
            value: serde_json::Value::Null,
            redacted: None,
        });
        assert_code(validate_action_result(&result), "missing_output_name");
    }

    #[test]
    fn named_outputs_are_allowed() {
        let mut result = base_result(FinalStatus::Succeeded);
        result.outputs.items.push(ActionOutputItem {
            name: "process_count".to_string(),
            value: serde_json::Value::from(2),
            redacted: Some(false),
        });
        validate_action_result(&result).expect("named output is valid");
    }

    #[test]
    fn succeeded_result_rejects_exit_reason_and_non_success_steps() {
        let mut result = base_result(FinalStatus::Succeeded);
        result.exit_reason = Some("exec_exit_1".to_string());
        assert_code(
            validate_action_result(&result),
            "succeeded_result_has_exit_reason",
        );

        for status in [
            StepStatus::Started,
            StepStatus::Failed,
            StepStatus::Cancelled,
            StepStatus::TimedOut,
        ] {
            let mut result = base_result(FinalStatus::Succeeded);
            result.step_records = vec![step_record(status)];
            assert_code(
                validate_action_result(&result),
                "succeeded_result_has_non_success_step",
            );
        }
    }

    #[test]
    fn succeeded_result_allows_skipped_steps() {
        let mut result = base_result(FinalStatus::Succeeded);
        result.step_records = vec![
            step_record(StepStatus::Succeeded),
            step_record(StepStatus::Skipped),
        ];
        validate_action_result(&result).expect("skipped steps are compatible with success");
    }

    #[test]
    fn rejected_result_forbids_succeeded_steps() {
        let mut result = base_result(FinalStatus::Rejected);
        result.step_records = vec![step_record(StepStatus::Succeeded)];
        assert_code(
            validate_action_result(&result),
            "rejected_result_has_success_step",
        );
    }

    #[test]
    fn rejected_result_allows_skipped_and_failed_steps() {
        for status in [StepStatus::Skipped, StepStatus::Failed, StepStatus::Started] {
            let mut result = base_result(FinalStatus::Rejected);
            result.step_records = vec![step_record(status)];
            validate_action_result(&result).expect("no succeeded step, so rejected is fine");
        }
    }

    #[test]
    fn failed_result_requires_a_failed_step() {
        let mut result = base_result(FinalStatus::Failed);
        result.step_records = vec![step_record(StepStatus::Cancelled)];
        assert_code(
            validate_action_result(&result),
            "failed_result_has_no_failed_step",
        );

        let mut result = base_result(FinalStatus::Failed);
        result.step_records = vec![
            step_record(StepStatus::Succeeded),
            step_record(StepStatus::Failed),
        ];
        validate_action_result(&result).expect("one failed step is enough");
    }

    #[test]
    fn timed_out_result_requires_a_timed_out_step() {
        let mut result = base_result(FinalStatus::TimedOut);
        result.step_records = vec![step_record(StepStatus::Cancelled)];
        assert_code(
            validate_action_result(&result),
            "timed_out_result_has_no_timed_out_step",
        );

        let mut result = base_result(FinalStatus::TimedOut);
        result.step_records = vec![step_record(StepStatus::TimedOut)];
        validate_action_result(&result).expect("timed out step present");
    }

    #[test]
    fn cancelled_result_requires_a_cancelled_step() {
        let mut result = base_result(FinalStatus::Cancelled);
        result.step_records = vec![step_record(StepStatus::TimedOut)];
        assert_code(
            validate_action_result(&result),
            "cancelled_result_has_no_cancelled_step",
        );

        let mut result = base_result(FinalStatus::Cancelled);
        result.step_records = vec![step_record(StepStatus::Cancelled)];
        validate_action_result(&result).expect("cancelled step present");
    }

    #[test]
    fn extreme_attempt_value_is_accepted() {
        let mut result = base_result(FinalStatus::Succeeded);
        result.step_records[0].attempt = u32::MAX;
        validate_action_result(&result).expect("unsigned maximum attempt is valid");
    }
}
