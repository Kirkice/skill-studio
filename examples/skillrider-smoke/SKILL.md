---
name: skillrider-smoke
description: 当用户明确要求运行 SkillRider 冒烟测试时使用，验证图谱、Pi Trace 和静态检查。
---

# SkillRider 冒烟测试

**触发条件：** 用户明确要求运行本测试。只读取本目录，不修改文件。

**调试协议：** 如果有 `skillrider_stage` 工具，每进入一个 Step 时必须先单独调用它，传入 `path: "SKILL.md"`、不含 `## ` 的完整标题（例如 `Step 1：验证直接引用`）及 `state: "running"`；等待返回后再执行。第 1、2、5、6 步结束时分别上报 `done`，第 3 步越界时上报 `failed` 并继续，第 4 步确认保留备用分支后上报 `skipped`。第 6 步先准备汇报，再上报 `done`，最后输出汇报。不要批量补报。工具不存在时仍可执行只读检查，但不会有完整阶段状态。

## Step 1：验证直接引用

用 Pi 的 `read` 工具读取 [检查清单](references/checklist.md)，记录返回的标记。不要改用 Shell。

## Step 2：验证间接引用

按照检查清单中的链接，用 `read` 工具读取 [间接引用样本](references/nested.md)。这条链接也标明本阶段对应的调用证据。不要跳过这一步。

## Step 3：验证失败事件

用 `read` 工具读取 [错误测试样本](references/error-target.md)，指定 `offset: 999999`。预期超出文件行数；记录实际结果，不重试。

## Step 4：保留未调用分支

[备用分支](references/unused.md)只用于对照，**不要读取**。

## Step 5：检查失效链接

[故意缺失的文件](references/intentionally-missing.md)用于 Problems 静态检查。不要读取或创建它。

## Step 6：汇报

列出每次调用的文件、实际结果和找到的标记。不要替用户宣称 UI 验收通过，界面状态由用户观察。
