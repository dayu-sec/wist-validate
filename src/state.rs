//! Local state validation entrypoints.

use wist_contracts::SCHEMA_VERSION_V1;
use wist_contracts::agent_state::AgentRuntimeState;

use crate::{ValidationError, parse_rfc3339, require_non_empty};

pub fn validate_execution_state(contract: &AgentRuntimeState) -> Result<(), ValidationError> {
    if contract.schema_version != SCHEMA_VERSION_V1 {
        return Err(ValidationError::new("invalid_schema_version"));
    }
    require_non_empty(&contract.agent_id, "missing_runtime_agent_id")?;
    require_non_empty(&contract.instance_id, "missing_runtime_instance_id")?;
    require_non_empty(&contract.version, "missing_runtime_version")?;
    parse_rfc3339(&contract.updated_at, "invalid_runtime_updated_at")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use wist_contracts::agent_state::RuntimeMode;

    use super::*;

    /// 校验用例：期望错误码 + 对合法样本的破坏性修改。
    type Case<T> = (&'static str, fn(&mut T));

    fn base_state() -> AgentRuntimeState {
        AgentRuntimeState::new(
            "agent-001".to_string(),
            "instance-001".to_string(),
            "0.1.0".to_string(),
            RuntimeMode::Normal,
            "2026-04-12T10:00:00Z".to_string(),
        )
    }

    fn assert_code(result: Result<(), ValidationError>, expected: &'static str) {
        let err = result.expect_err("expected a validation error");
        assert_eq!(err.code, expected);
    }

    #[test]
    fn a_well_formed_state_passes() {
        validate_execution_state(&base_state()).expect("valid runtime state");
    }

    #[test]
    fn state_errors_have_stable_codes() {
        let cases: &[Case<AgentRuntimeState>] = &[
            ("invalid_schema_version", |state| {
                state.schema_version = "v2".to_string()
            }),
            ("missing_runtime_agent_id", |state| {
                state.agent_id = " ".to_string()
            }),
            ("missing_runtime_instance_id", |state| {
                state.instance_id = String::new()
            }),
            ("missing_runtime_version", |state| {
                state.version = "\t".to_string()
            }),
            ("invalid_runtime_updated_at", |state| {
                state.updated_at = "not-a-time".to_string()
            }),
        ];
        for &(expected, mutate) in cases {
            let mut state = base_state();
            mutate(&mut state);
            assert_code(validate_execution_state(&state), expected);
        }
    }
}
