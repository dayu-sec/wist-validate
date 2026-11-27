# 更新日志

本文件记录 `wist-validate` 的所有重要变更。格式遵循 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)，
版本号遵循[语义化版本](https://semver.org/lang/zh-CN/)。

## [0.14.0] - 2026-11-27

### 变更（依赖）

- 对齐 `wist-contracts 0.16.0`（新增主机流记录契约 `host_flow`）与 `wist-api 0.15.0`。
  本仓代码无改动；但报文校验接收的是 `wist_api` 的类型，而 api 直接引用 contracts 的领域类型 ——
  **两个依赖必须同步升**，否则树里会同时出现 contracts 0.15 与 0.16，
  同一个 `ActionPlan` 变成两个不兼容的类型。

## [0.13.0] - 2026-11-26

### 变更（依赖）

- 对齐 `wist-contracts 0.15.0`（新增采集来源 `HostFlow`）与 `wist-api 0.14.0`。本仓代码无改动；
  但报文校验接收的是 `wist_api` 的类型，而 api 直接引用 contracts 的领域类型 ——
  **两个依赖必须同步升**，否则树里会同时出现 contracts 0.14 与 0.15，
  同一个 `ActionPlan` 变成两个不兼容的类型。

## [0.12.0] - 2026-11-22

### 新增（`validate_attestation`：签发证明的**形状**校验）

- `validate_action_plan` 现在会检查 `ActionPlan.attestation` 的形状：`api_version` /
  digest 格式 / 算法 / 签名长度与 hex / `key_id` / `issued_by` / `approval_id` /
  `required_approvers` / 审批人非空且不重复 / 时间戳与窗口 / 证明不晚于计划 / 引用一致。
- 新错误码：`invalid_attestation_api_version`、`invalid_attestation_plan_digest`、
  `unsupported_attestation_algorithm`、`invalid_attestation_signature`、
  `missing_attestation_key_id`、`missing_attestation_issued_by`、
  `missing_attestation_approval_id`、`invalid_required_approvers`、
  `insufficient_approvers`、`duplicate_approver`、`missing_approver`、
  `invalid_attestation_signed_at`、`invalid_attestation_expires_at`、
  `invalid_attestation_window`、`attestation_outlives_plan`、`approval_ref_mismatch`。
- **刻意不验签**：验签要信任锚，是执行侧（agentd）的事，而且「要不要验」由本地策略决定。
  这一层只保证「拿到的东西长得像一份完整的证明」。
- 对齐 `wist-contracts 0.14.0`。

## [0.11.0] - 2026-11-22

### 新增（`invalid_step_args`）

- `validate_action_plan` 校验 `ActionPlanStep.args`：只允许 `invoke` 携带，
  且必须是 JSON 对象或 `null`；非 `invoke` 带 `args` 一律拒绝
  （语义未定义，不能留「看着有参数、实际被忽略」的死字段）。
- 仍然**不**在校验器里认识 opcode / target：业务校验归 agentd 的 opcode 注册表。
- 对齐 `wist-contracts 0.13.0`（`ActionPlanStep.args`）。

## [0.10.0] - 2026-11-21

### 变更（依赖）

- 对齐 `wist-contracts 0.12.0`（新增采集来源 `KeyProcessMetrics`）与 `wist-api 0.13.0`。
  本仓校验逻辑无改动；对外暴露的报文类型来自 api/contracts，所以随它们升 minor。
- 三个 crate（contracts / api / validate）必须**同批**发布，混用会出现两份 contracts。

## [0.9.0] - 2026-11-10

### 变更（依赖）

- **对齐 `wist-contracts 0.11` + `wist-api 0.11`**。采集状态改多字段正交模型，
  `AgentLocalCollection` 新增 `prerequisites` / `result` / `reasons`。
- 本 crate 的校验入口以 `wist_contracts` 类型为参数，**消费方需同样升到 contracts 0.11**，
  否则 cargo 会同时保留两份 contracts、跨边界类型对不上（mismatched types）。

## [0.8.0] - 2026-11-03

### 变更（不兼容）

- **对齐 `wist-contracts` 0.10**（`wist-api` 仍是 `0.10`）。本 crate 的校验入口以
  `wist_contracts` 的类型为参数，**消费方需同样升到 contracts 0.10**，
  否则 cargo 会同时保留两份 contracts、跨边界类型对不上（mismatched types）。

## [0.7.0] - 2026-11-03

### 变更（不兼容）

- **对齐 `wist-api` 0.10**：`wist-api` 依赖升到 `0.10`（`wist-contracts` 仍是 `0.9`）。
  本 crate 的校验入口以 `wist_api` 的 seam 类型为参数，**消费方需同样升到 `wist-api` 0.10** ——
  否则 cargo 会同时保留两份 `wist-api`，跨边界类型对不上（mismatched types）。

## [0.6.0] - 2026-10-18

### 变更（不兼容）

- **对齐 `wist-contracts` 0.9 / `wist-api` 0.9**：依赖升到 `0.9` / `0.9`。本 crate 的校验入口以
  `wist_contracts` 的类型为参数，**消费方需同样升到 0.9**，否则跨边界类型对不上
  （cargo 会同时保留 0.8 与 0.9，报 mismatched types）。

## [0.5.0] - 2026-10-11

### 变更（不兼容）

- **对齐 `wist-contracts` 0.8 / `wist-api` 0.8**：依赖升到 `0.8` / `0.8`。本 crate 的校验入口以
  `wist_contracts` 的类型为参数（`ActionResult` / `ActionPlan` / `AgentRuntimeState` …），
  **消费方需同样升到 `wist-contracts` 0.8 + `wist-api` 0.8**，否则跨边界类型对不上。
- **`config` 模块删除**：agent 配置校验 `validate_config` 已随 `AgentConfig` 一起移到
  `wist-agentd::config::validate`（`wist-contracts` 删了 `agent_config` 模块）。本 crate 不再提供
  `config` 校验；其 fixture 测试也一并迁入 `wist-agentd`。通用助手 `require_non_empty` /
  `parse_rfc3339` 仍在这里（改为 `pub`，供上层复用）。

## [0.4.0] - 2026-10-09

### 变更（不兼容）

- **对齐 `wist-contracts` 0.7 / `wist-api` 0.7**：依赖由 `wist-contracts 0.3` / `wist-api 0.6` 升到
  `0.7` / `0.7`（0.4–0.7 期间 seam 报文陆续迁入 `wist-api`）。本 crate 的校验入口大量以
  `wist_contracts` 的 `ActionResult` / `ActionPlan` / `AgentConfig` / `AgentRuntimeState` 为参数，
  **消费方需同样升到 `wist-contracts` 0.7 + `wist-api` 0.7**，否则跨边界类型对不上。

## [0.3.0] - 2026-10-06

### 变更（不兼容）

- 依赖 `wist-api` 由 `0.5` 升到 **`0.6`**：后者把杂物袋模块 `gateway` 拆成
  `action_plan` / `action_result` / `facts` / `discovery_policies`（**线上 JSON 不变**）。
  本 crate 的校验入口（`validate_dispatch_action_plan` / `validate_action_plan_ack` /
  `validate_report_action_result`）接受这些报文类型，故**类型的 crate 归属未变、但版本变了**——
  消费方拿 `wist-api 0.5` 构造的报文现在与这里的入参不是同一个类型。

### 说明（升级序）

- 本 crate 必须与 `wist-api 0.6` **同批**升级：`wist-api` → `wist-validate` → `wist-agentd`。
  否则依赖图里会同时出现 `wist-api` 0.5 与 0.6，同名报文变成两个不同类型（编译期
  `expected ReportActionResult, found a different ReportActionResult`）。

## [0.2.2] - 2026-10-06

### 变更

- 依赖 `wist-api` 由 `0.4` 升到 **`0.5`**（该版把 `gateway` / `work` / `agent_uplink` 规范成 `v1` 子模块，
  报文路径经 `pub use v1::*` 不变——**非破坏**）。公共 API 无变化。

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
