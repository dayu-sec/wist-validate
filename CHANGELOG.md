# 更新日志

本文件记录 `wist-validate` 的所有重要变更。格式遵循 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)，
版本号遵循[语义化版本](https://semver.org/lang/zh-CN/)。

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
