//! Persistent Auto Battle Data (ABD) high-class/dark chip usage recorder and exporter (Scheme A).
//!
//! Records per-round Mega / Giga / Dark / Program Advance chip counts used by ABD in both
//! Online and Practice EXE4/EXE5 battles into a single JSONL file (`abd-stats.jsonl`), without
//! altering simulation state, rollback digests, or match outcomes. Exports cleanly to readable
//! `.log`, Excel-friendly `.csv` (UTF-8 BOM), or `.jsonl`, always leading with the AI version
//! (`GBA` / `合集` / `NDS`) and explicitly noting when Scheme B's hard cap was active.

use chrono::{Local, TimeZone, Utc};
use saito_battle::content::{ChipClass, ChipFlags, Content};
use saito_content_api::ChipHandle;
use saito_frontend::driver::RoundAbdUsage;
use saito_match::AbdMode;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Export format selectable in Settings (`LOG` | `CSV` | `JSONL`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AbdExportFormat {
    #[default]
    Log = 0,
    Csv = 1,
    Jsonl = 2,
}

impl AbdExportFormat {
    pub fn from_index(idx: i32) -> Self {
        match idx {
            1 => Self::Csv,
            2 => Self::Jsonl,
            _ => Self::Log,
        }
    }

    pub fn as_index(self) -> i32 {
        self as i32
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Log => "log",
            Self::Csv => "csv",
            Self::Jsonl => "jsonl",
        }
    }

    pub fn default_filename(self) -> &'static str {
        match self {
            Self::Log => "abd-usage.log",
            Self::Csv => "abd-usage.csv",
            Self::Jsonl => "abd-usage.jsonl",
        }
    }
}

/// Metadata attached to a live Online or Practice battle for ABD usage logging.
#[derive(Clone, Debug)]
pub struct AbdLogContext {
    pub game: String,
    pub mode: &'static str,
    pub abd_mode: AbdMode,
    pub abd_chip_cap: Option<u8>,
    pub players: [String; 2],
}

/// One Mega / Giga / Dark / PA chip's usage count in a single round for one ABD owner.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbdChipEntry {
    pub key: String,
    pub name: String,
    /// `"D"` (Dark), `"M"` (Mega), `"G"` (Giga), or `"PA"` (Program Advance).
    pub class: String,
    pub count: u16,
}

/// One recorded round of ABD Mega / Giga / Dark / PA chip usage.
///
/// `ai_version` is placed first so every serialized JSONL line starts with the AI version tag.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbdRoundRecord {
    /// AI version tag placed first: `"GBA"`, `"合集"`, or `"NDS"`.
    pub ai_version: String,
    /// Whether Scheme B's hard cap was enabled during this round.
    pub under_cap_limit: bool,
    /// Explicit Scheme B notice when active (e.g. `"在方案B的限制下(≤5)"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_note: Option<String>,
    /// Active per-round hard cap value (`1..=15`) when `under_cap_limit` is true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap_limit: Option<u8>,
    /// Unix timestamp in seconds (UTC).
    pub timestamp: u64,
    /// Game identifier (`"exe4"` or `"exe5"`).
    pub game: String,
    /// Battle source (`"online"` or `"practice"`).
    pub mode: String,
    /// 1-based round number within the set.
    pub round: u8,
    /// Side 0 (P1) and Side 1 (P2) player labels.
    pub players: [String; 2],
    /// Tracked chip usage per ABD owner side, sorted by count descending then key ascending.
    pub sides: [Vec<AbdChipEntry>; 2],
}

/// Canonical AI version label placed at the start of every record.
pub fn ai_version_label(mode: AbdMode) -> &'static str {
    match mode {
        AbdMode::Original => "GBA",
        AbdMode::LegacyCollection => "合集",
        AbdMode::DoubleTeamDs => "NDS",
    }
}

fn chip_class_tag(content: &Content, chip: ChipHandle) -> &'static str {
    let data = content.chip(chip);
    if data.flags.has(ChipFlags::DARK) {
        "D"
    } else {
        match data.class {
            ChipClass::Mega => "M",
            ChipClass::Giga => "G",
            ChipClass::ProgramAdvance => "PA",
            ChipClass::Standard | ChipClass::Special => "S",
        }
    }
}

/// Build an [`AbdRoundRecord`] from settled round ABD usage data.
pub fn build_record(content: &Content, ctx: &AbdLogContext, usage: &RoundAbdUsage, timestamp: u64) -> Option<AbdRoundRecord> {
    if !saito_match::abd_mode::supported(&ctx.game) || !usage.has_any() {
        return None;
    }
    let sides = [0, 1].map(|side| {
        let mut entries: Vec<AbdChipEntry> = usage.uses[side]
            .iter()
            .filter(|&&(_, count)| count > 0)
            .map(|&(chip, count)| AbdChipEntry {
                key: saito_match::ids::local(&content.defs.chip(chip).key).to_string(),
                name: saito_match::names::chip(content, chip).to_string(),
                class: chip_class_tag(content, chip).to_string(),
                count,
            })
            .collect();
        entries.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
        entries
    });
    if sides[0].is_empty() && sides[1].is_empty() {
        return None;
    }
    let cap_limit = ctx.abd_chip_cap.filter(|&c| saito_match::abd_mode::chip_cap_allowed(&ctx.game, Some(c)));
    let under_cap_limit = cap_limit.is_some();
    let limit_note = cap_limit.map(|c| format!("在方案B的限制下(≤{c})"));
    Some(AbdRoundRecord {
        ai_version: ai_version_label(ctx.abd_mode).to_string(),
        under_cap_limit,
        limit_note,
        cap_limit,
        timestamp,
        game: ctx.game.clone(),
        mode: ctx.mode.to_string(),
        round: usage.round.max(1),
        players: ctx.players.clone(),
        sides,
    })
}

/// Append one round record (if non-empty) to the default `abd-stats.jsonl` path.
pub fn record_round(content: &Content, ctx: &AbdLogContext, usage: &RoundAbdUsage) {
    let now = Utc::now().timestamp().max(0) as u64;
    let Some(record) = build_record(content, ctx, usage, now) else { return };
    let path = crate::paths::abd_stats();
    if let Err(why) = append_to(&path, &record) {
        crate::errors::log("recording ABD chip usage", &why);
    }
}

/// Append `record` as one JSONL line to `path`.
pub fn append_to(path: &Path, record: &AbdRoundRecord) -> Result<(), String> {
    let mut line = serde_json::to_string(record).map_err(|e| e.to_string())?;
    line.push('\n');
    #[cfg(target_arch = "wasm32")]
    {
        let mut existing = match saito_files::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e.to_string()),
        };
        existing.push_str(&line);
        saito_files::write(path, existing.as_bytes()).map_err(|e| e.to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::io::Write;
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        f.write_all(line.as_bytes()).map_err(|e| e.to_string())
    }
}

/// Load all valid ABD records from the default `abd-stats.jsonl`.
pub fn load_all() -> Vec<AbdRoundRecord> {
    load_from(&crate::paths::abd_stats())
}

/// Load all valid ABD records from `path` (missing file returns empty).
pub fn load_from(path: &Path) -> Vec<AbdRoundRecord> {
    let text = match saito_files::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<AbdRoundRecord>(l).ok())
        .collect()
}

/// Summary counts: `(total, gba, lc, nds)`.
pub fn counts_summary(records: &[AbdRoundRecord]) -> (usize, usize, usize, usize) {
    let mut gba = 0;
    let mut lc = 0;
    let mut nds = 0;
    for r in records {
        match r.ai_version.as_str() {
            "GBA" | "Original" => gba += 1,
            "合集" | "LC" | "LegacyCollection" => lc += 1,
            "NDS" | "5DS" | "DoubleTeamDs" => nds += 1,
            _ => {}
        }
    }
    (records.len(), gba, lc, nds)
}

fn format_time(ts: u64) -> String {
    Local
        .timestamp_opt(ts as i64, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| ts.to_string())
}

fn format_mode_cn(mode: &str) -> &str {
    match mode {
        "online" => "联机",
        "practice" => "练习",
        other => other,
    }
}

fn format_chips(entries: &[AbdChipEntry]) -> String {
    if entries.is_empty() {
        return "-".to_string();
    }
    entries
        .iter()
        .map(|e| format!("{}({})×{}", e.name, e.class, e.count))
        .collect::<Vec<_>>()
        .join(", ")
}

fn max_count(entries: &[AbdChipEntry]) -> u16 {
    entries.iter().map(|e| e.count).max().unwrap_or(0)
}

fn csv_escape(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// Format records as readable `.log` lines, always starting with `[GBA]` / `[合集]` / `[NDS]`.
pub fn format_log(records: &[AbdRoundRecord]) -> String {
    let mut out = String::new();
    for r in records {
        let cap_tag = if r.under_cap_limit {
            match r.cap_limit {
                Some(c) => format!(" 【在方案B的限制下(≤{c})】"),
                None => " 【在方案B的限制下】".to_string(),
            }
        } else {
            String::new()
        };
        let p1 = if r.players[0].is_empty() { "P1".to_string() } else { format!("P1({})", r.players[0]) };
        let p2 = if r.players[1].is_empty() { "P2".to_string() } else { format!("P2({})", r.players[1]) };
        out.push_str(&format!(
            "[{}]{} {} | {} {} R{} | {}: {} | {}: {}\n",
            r.ai_version,
            cap_tag,
            format_time(r.timestamp),
            r.game.to_uppercase(),
            format_mode_cn(&r.mode),
            r.round,
            p1,
            format_chips(&r.sides[0]),
            p2,
            format_chips(&r.sides[1]),
        ));
    }
    out
}

/// Format records as Excel-compatible UTF-8 BOM `.csv`, with `ai_version` as column 1.
pub fn format_csv(records: &[AbdRoundRecord]) -> String {
    let mut out = String::from("\u{FEFF}ai_version,limit_note,cap_limit,time,game,mode,round,p1_name,p1_max,p1_chips,p2_name,p2_max,p2_chips\n");
    for r in records {
        let limit_note = if r.under_cap_limit {
            match r.cap_limit {
                Some(c) => format!("在方案B的限制下(≤{c})"),
                None => "在方案B的限制下".to_string(),
            }
        } else {
            "无限制".to_string()
        };
        let cap_col = r.cap_limit.map(|c| c.to_string()).unwrap_or_default();
        let fields = [
            csv_escape(&r.ai_version),
            csv_escape(&limit_note),
            csv_escape(&cap_col),
            csv_escape(&format_time(r.timestamp)),
            csv_escape(&r.game.to_uppercase()),
            csv_escape(format_mode_cn(&r.mode)),
            r.round.to_string(),
            csv_escape(&r.players[0]),
            max_count(&r.sides[0]).to_string(),
            csv_escape(&format_chips(&r.sides[0])),
            csv_escape(&r.players[1]),
            max_count(&r.sides[1]).to_string(),
            csv_escape(&format_chips(&r.sides[1])),
        ];
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    out
}

/// Format records as `.jsonl`, with `ai_version` as the first JSON key of every line.
pub fn format_jsonl(records: &[AbdRoundRecord]) -> String {
    let mut out = String::new();
    for r in records {
        if let Ok(line) = serde_json::to_string(r) {
            out.push_str(&line);
            out.push('\n');
        }
    }
    out
}

/// Render `records` in the requested export format.
pub fn render(records: &[AbdRoundRecord], format: AbdExportFormat) -> String {
    match format {
        AbdExportFormat::Log => format_log(records),
        AbdExportFormat::Csv => format_csv(records),
        AbdExportFormat::Jsonl => format_jsonl(records),
    }
}

/// Export all recorded ABD stats to `path` in `format`. Returns record count written.
pub fn export_to(path: &Path, format: AbdExportFormat) -> Result<usize, String> {
    let records = load_all();
    let text = render(&records, format);
    saito_files::write(path, text.as_bytes()).map_err(|e| e.to_string())?;
    Ok(records.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_records() -> Vec<AbdRoundRecord> {
        vec![
            AbdRoundRecord {
                ai_version: "合集".into(),
                under_cap_limit: false,
                limit_note: None,
                cap_limit: None,
                timestamp: 1_764_092_400,
                game: "exe5".into(),
                mode: "online".into(),
                round: 1,
                players: ["Alice".into(), "Bob".into()],
                sides: [
                    vec![
                        AbdChipEntry { key: "darkthnd".into(), name: "ダークサンダー".into(), class: "D".into(), count: 6 },
                        AbdChipEntry { key: "serchmn-sp".into(), name: "サーチマンSP".into(), class: "M".into(), count: 2 },
                    ],
                    vec![AbdChipEntry { key: "bignoise".into(), name: "ビッグノイズ".into(), class: "PA".into(), count: 3 }],
                ],
            },
            AbdRoundRecord {
                ai_version: "NDS".into(),
                under_cap_limit: true,
                limit_note: Some("在方案B的限制下(≤5)".into()),
                cap_limit: Some(5),
                timestamp: 1_764_092_700,
                game: "exe5".into(),
                mode: "practice".into(),
                round: 1,
                players: ["Alice".into(), "Dummy".into()],
                sides: [
                    vec![AbdChipEntry { key: "darkthnd".into(), name: "ダークサンダー".into(), class: "D".into(), count: 5 }],
                    vec![],
                ],
            },
            AbdRoundRecord {
                ai_version: "GBA".into(),
                under_cap_limit: false,
                limit_note: None,
                cap_limit: None,
                timestamp: 1_764_093_000,
                game: "exe4".into(),
                mode: "online".into(),
                round: 2,
                players: ["P1".into(), "P2".into()],
                sides: [
                    vec![AbdChipEntry { key: "drksword".into(), name: "ダークソード".into(), class: "D".into(), count: 4 }],
                    vec![],
                ],
            },
        ]
    }

    #[test]
    fn exports_start_with_ai_version_and_mark_scheme_b_limit() {
        let recs = sample_records();
        assert_eq!(counts_summary(&recs), (3, 1, 1, 1));

        let log = format_log(&recs);
        let lines: Vec<&str> = log.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("[合集] "), "log line 0: {}", lines[0]);
        assert!(!lines[0].contains("方案B"));
        assert!(lines[1].starts_with("[NDS] 【在方案B的限制下(≤5)】 "), "log line 1: {}", lines[1]);
        assert!(lines[1].contains("练习"));
        assert!(lines[2].starts_with("[GBA] "), "log line 2: {}", lines[2]);

        let csv = format_csv(&recs);
        assert!(csv.starts_with("\u{FEFF}ai_version,limit_note,cap_limit,"));
        let csv_lines: Vec<&str> = csv.trim_start_matches('\u{FEFF}').lines().collect();
        assert!(csv_lines[1].starts_with("合集,无限制,,"));
        assert!(csv_lines[2].starts_with("NDS,在方案B的限制下(≤5),5,"));
        assert!(csv_lines[3].starts_with("GBA,无限制,,"));

        let jsonl = format_jsonl(&recs);
        let json_lines: Vec<&str> = jsonl.lines().collect();
        assert!(json_lines[0].starts_with("{\"ai_version\":\"合集\",\"under_cap_limit\":false"));
        assert!(json_lines[1].starts_with("{\"ai_version\":\"NDS\",\"under_cap_limit\":true,\"limit_note\":\"在方案B的限制下(≤5)\",\"cap_limit\":5"));
        assert!(json_lines[2].starts_with("{\"ai_version\":\"GBA\",\"under_cap_limit\":false"));
    }

    #[test]
    fn jsonl_append_and_reload_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("abd-stats.jsonl");
        let recs = sample_records();
        for r in &recs {
            append_to(&path, r).unwrap();
        }
        assert_eq!(load_from(&path), recs);
    }
}
