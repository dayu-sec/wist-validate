# wist-validate

Static validators for contracts, configuration, and runtime state.

[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/rustc-1.85+-orange.svg)](#)

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
