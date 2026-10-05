# 更新日志

本文件记录 `wist-validate` 的所有重要变更。格式遵循 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)，
版本号遵循[语义化版本](https://semver.org/lang/zh-CN/)。

## [0.2.1] - 2026-10-05

### 变更

- 依赖 `wist-api` 由 `0.3` 升到 **`0.4`**（对齐 work / agent_uplink seam 报文迁入后的版本）。

## [0.2.0] - 2026-10-05

### 变更（不兼容）

- **agent 面 seam 校验的输入类型改由 `wist-api` 提供**：`validate_dispatch_action_plan` /
  `validate_action_plan_ack` / `validate_report_action_result` 现在接受
  `wist_api::gateway::{DispatchActionPlan, ActionPlanAck, ReportActionResult}`（类型只换了 crate，
  字段不变）。调用方相应改用 `use wist_api::gateway::…`。

## [0.1.5] - 2026-10-05

### 变更

- 依赖 `wist-contracts` 由 `0.2` 升到 **`0.3`**：对齐「agent 注册/续期报文迁到 `wist-api`」后的
  契约版本。校验的契约对象形态不变，使用者无需改动调用。
