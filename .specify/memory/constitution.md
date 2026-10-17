<!--
Sync Impact Report
- Version change: (template, 未版本化) → 1.0.0
- Modified principles: 無（首次制定）
- Added sections: 核心原則 I–II、治理
- Removed sections: 範本中多餘的原則與章節（依使用者指示僅保留兩項）
- Deferred / TODO: 無
- 此報告為審閱用暫存內容，提交前應移除。
-->

# embroider Constitution

## Core Principles

### I. 測試驅動開發（TDD，NON-NEGOTIABLE）

所有行為變更 MUST 遵循紅—綠—重構：先寫失敗的測試，確認它因正確的原因失敗，再寫最少的
實作使其通過，最後重構。

- 不得在沒有對應失敗測試的情況下新增或修改業務邏輯。
- 修正 bug 時 MUST 先寫出能重現該 bug 的測試。
- 測試 MUST 在合併前全數通過；MUST NOT 以刪除或跳過測試來使其通過。
- 任務清單中，測試任務 MUST 排在對應的實作任務之前。

### II. 繁體中文

規格文件、計畫、任務清單，以及對使用者的所有回答 MUST 使用繁體中文撰寫。程式識別字、
指令與技術專有名詞維持原文。

## Governance

本憲章優先於其他開發慣例。修訂 MUST 以文件記錄變更內容與理由，並採語意化版本：
移除或重新定義原則為 MAJOR；新增原則或實質擴充為 MINOR；文字澄清為 PATCH。
計畫與審查 MUST 檢查是否符合本憲章。

**Version**: 1.0.0 | **Ratified**: 2026-10-17 | **Last Amended**: 2026-10-17

<!-- Knowie: Project Knowledge -->
## Project Knowledge

This project maintains structured knowledge in `knowledge/`:

- **Principles** (`knowledge/principles.md`): Core axioms and derived development principles — the project's non-negotiable rules.
- **Vision** (`knowledge/vision.md`): Goals, current state, architecture decisions, and roadmap.
- **Experience** (`knowledge/experience.md`): Distilled lessons from past development — patterns, pitfalls, and takeaways.

Read these files at the start of any task to understand the project's *why* and constraints.
Additional context may be found in `knowledge/concepts/`, `knowledge/history/`, and `knowledge/draft/`.

Learned procedures live in `knowledge/skills/` (agentskills.io SKILL.md format). If your tool auto-loads skills, they may be projected into your skill directory; otherwise read the relevant `SKILL.md` there and follow it.
<!-- /Knowie -->
