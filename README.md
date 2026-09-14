# wist-validate

Static validators for contracts, configuration, and runtime state.

[![crates.io](https://img.shields.io/crates/v/wist-validate.svg)](https://crates.io/crates/wist-validate)
[![docs.rs](https://img.shields.io/docsrs/wist-validate/latest.svg)](https://docs.rs/wist-validate)
[![Downloads](https://img.shields.io/crates/d/wist-validate.svg)](https://crates.io/crates/wist-validate)
[![CI](https://github.com/dayu-sec/wist-validate/actions/workflows/ci.yml/badge.svg)](https://github.com/dayu-sec/wist-validate/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/dayu-sec/wist-validate/branch/main/graph/badge.svg)](https://codecov.io/gh/dayu-sec/wist-validate)
[![dependency status](https://deps.rs/repo/github/dayu-sec/wist-validate/status.svg)](https://deps.rs/repo/github/dayu-sec/wist-validate)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

`wist-validate` provides small, side-effect-free validators that return a `ValidationError` on
failure. It validates [`wist-contracts`](../wist-contracts) objects, `wist-agentd` configuration,
and runtime state before they are executed or persisted.

## Modules

| Module          | Purpose                              |
| --------------- | ------------------------------------ |
| `action_plan`   | `ActionPlan` validation entrypoints. |
| `action_result` | `ActionResult` validation entrypoints. |
| `config`        | Config validation entrypoints.       |
| `gateway`       | Gateway envelope validation entrypoints. |
| `state`         | Local state validation entrypoints.  |

## Example

```rust
use wist_validate::action_plan::validate_action_plan;

validate_action_plan(&plan)?;
```

## Related crates

- [`wist-contracts`](../wist-contracts) — the objects being validated.
- [`wist-agentd`](../wist-agentd) — validates config and plans before execution.

## License

[Apache-2.0](LICENSE)
