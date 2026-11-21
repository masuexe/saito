# Saito Agent Collaboration Guide

## Issue & Feature Board (Tencent Docs SmartSheet)

The project uses a Tencent Docs SmartSheet to collect and track feature requests and bug reports from QQ community users. Agents can read and update the board directly via the `tencent-smartsheet` MCP server.

- **URL**: https://docs.qq.com/smartsheet/DY2Nzd210TEhibHdr
- **`file_id`**: `DY2Nzd210TEhibHdr`
- **`sheet_id`**: `t00i2h` (`智能表1`)
- **Views**:
  - `需求总表` (`v2JKhc`, grid view)
  - `需求看板` (`vS21Im`, kanban view, grouped by `状态`)
  - `需求收集表` (`vrUXZz`, form view for community submissions)

### Key Fields

> Note: Keep exact Chinese field names and option texts when calling `tencent-smartsheet` tools.

| Field Name | field_id | Type | Description / Options |
| :--- | :--- | :--- | :--- |
| `需求标题` | `fkfKit` | text | Primary column (issue title) |
| `编号` | `fsl17x` | autoNumber | Auto-incrementing ID starting from `S-001` |
| `类型` | `f0JNov` | singleSelect | `新功能` / `Bug反馈` / `体验优化` / `其他` |
| `状态` | `fzVaGp` | singleSelect | `待评估` / `待开发` / `开发中` / `已完成` / `暂不考虑` |
| `优先级` | `fsC14X` | singleSelect | `P0 紧急` / `P1 高` / `P2 中` / `P3 低` |
| `详细描述 / 复现步骤` | `fLw0GD` | text | Context, expected behavior, or reproduction steps |
| `截图` | `fbqdLX` | image | Screenshots (readable by agents via `image_value.items[].image_url`) |
| `附件` | `fvfeZn` | attachment | Legacy attachments (unreadable via MCP; kept for historical records) |
| `创建人` | `fxO0vR` | createdUser | Auto-recorded submitter account (web UI & row-level self-edit permissions; omitted in MCP records) |
| `维护者备注` | `fikdq2` | text | Technical evaluation, fix summary, or rejection reason |
| `提交时间` | `fFlBj3` | createdTime | Automatically recorded timestamp (web UI; omitted in MCP records) |

---

## Board-Driven Development SOP

1. **Triage**
   - Call `smartsheet_list_records` on `DY2Nzd210TEhibHdr` / `t00i2h`.
   - If a newly submitted record has an empty `状态`, set it to `待评估`.
   - **Reading screenshots**: For records with `截图` (`image_value.items[].image_url`), download the image to a scratch path with `curl -sSL -H "Referer: https://docs.qq.com/" -o <path> "<image_url>"` (the `Referer` header is required; direct requests return HTTP 403) and view it with `read_file`.
   - Note that SmartSheet record comments and legacy `附件` (`fvfeZn`) files are not readable via `tencent-smartsheet` MCP.
   - Evaluate feasibility and impact against the codebase; after user confirmation, update `状态` (`待开发` or `暂不考虑`), `优先级`, and `维护者备注`.
2. **Develop**
   - Before implementing a specific item (e.g., `S-002`), call `smartsheet_update_records` to set its `状态` to `开发中`.
3. **Resolve**
   - Always include the board item ID (e.g., `(S-002)`) in the git commit message.
   - After implementation, passing build/test verification, and committing, update `状态` to `已完成` and record the commit hash (e.g., `commit abc603f`) plus a concise summary in `维护者备注`.
4. **Safety Constraints**
   - Never delete community-submitted records.
   - Only update maintainer fields (`状态`, `优先级`, `维护者备注`); preserve original submission content and submitter information.
