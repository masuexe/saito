//! What a save or a link record states of a player as EXE3's rules' setup
//! facts: its NaviCust programs and its equipped Style.

use crate::save::{NAVICUST_SIZE, Part};
use crate::{Compat, Style};
use saito_battle::Content;
use saito_battle::rules::Fact;
use saito_content_api::{Registry, Value};

/// A NaviCust list (the save's, in its order) as the setup's
/// `navicust_programs`: each part's program and color (compat's
/// `navicust.toml`), its center on the engine's 7×7 grid (the save's column
/// and row `0..=4` on EXE3's 5×5 board, shifted by +1), its quarter-turns,
/// and whether it is compressed. Fails on any unknown or unimplemented part.
pub fn navicust<'c>(content: &'c Content, compat: &Compat, parts: &[Option<Part>]) -> Result<Vec<Fact<'c>>, String> {
    let mut out = Vec::new();
    for p in parts.iter().flatten() {
        let Some((key, color)) = compat.navicust_part(p.id)? else { continue };
        let program = content
            .defs
            .entry_in("navicust_programs", key)
            .ok_or_else(|| format!("the content has no NaviCust program {key}"))?;
        if p.column > 4 || p.row > 4 || p.rotation > 3 {
            return Err(format!(
                "NaviCust part {:#04x} at column {}, row {}, turned {}: off EXE3's 5x5 board",
                p.id, p.column, p.row, p.rotation
            ));
        }
        let colors = program_colors(content, program);
        let color = colors
            .get(color as usize)
            .copied()
            .ok_or_else(|| format!("NaviCust part {:#04x}: {key} has no color {color}", p.id))?;
        out.push(Fact::Record(vec![
            ("program", Fact::Value(Value::Def(Registry::Entry, program.0))),
            ("color", Fact::Name(color)),
            ("x", Fact::Value(Value::Int(p.column as i64 + 1))),
            ("y", Fact::Value(Value::Int(p.row as i64 + 1))),
            ("rotation", Fact::Value(Value::Int(p.rotation as i64))),
            ("compressed", Fact::Value(Value::Bool(p.compressed))),
        ]));
    }
    Ok(out)
}

/// Lenient NaviCust translation for save import: reads the part list (`0x1300`)
/// and optional 5×5 grid (`0x1D90`), skips unimplemented or invalid parts with
/// human-readable notes instead of failing the entire NaviCust, and honors both
/// per-entry compression (`p.compressed`) and learned compression event flags.
pub fn navicust_import<'c>(
    content: &'c Content,
    compat: &Compat,
    game: &str,
    parts: &[Option<Part>],
    grid: &[[Option<usize>; NAVICUST_SIZE]; NAVICUST_SIZE],
    is_compressed: impl Fn(u8) -> bool,
) -> (Vec<Fact<'c>>, Vec<String>) {
    let grid_has_entries = grid.iter().flatten().any(Option::is_some);
    let mut on_grid = [false; crate::save::NAVICUST_PARTS];
    if grid_has_entries {
        for idx in grid.iter().flatten().flatten() {
            if *idx < on_grid.len() {
                on_grid[*idx] = true;
            }
        }
    }

    let mut out = Vec::new();
    let mut notes = Vec::new();
    for (idx, p) in parts.iter().enumerate() {
        let Some(p) = p else { continue };
        if grid_has_entries && !on_grid[idx] {
            continue;
        }
        let (key, color_idx) = match compat.navicust_part(p.id) {
            Ok(Some(found)) => found,
            Ok(None) => continue,
            Err(e) => {
                notes.push(format!("the save's NaviCust holds unknown part {:#04x} ({e}): left out", p.id));
                continue;
            }
        };
        let Some(program) = content.defs.entry_in("navicust_programs", key) else {
            notes.push(format!("the save's NaviCust holds {key} (part {:#04x}), which {game} hasn't: left out", p.id));
            continue;
        };
        if p.column > 4 || p.row > 4 || p.rotation > 3 {
            notes.push(format!(
                "the save's NaviCust part {key} ({:#04x}) at column {}, row {}, turned {} is off EXE3's 5x5 board: left out",
                p.id, p.column, p.row, p.rotation
            ));
            continue;
        }
        let colors = program_colors(content, program);
        let Some(&color) = colors.get(color_idx as usize) else {
            notes.push(format!("the save's NaviCust part {key} ({:#04x}) has no color {color_idx} in {game}: left out", p.id));
            continue;
        };
        out.push(Fact::Record(vec![
            ("program", Fact::Value(Value::Def(Registry::Entry, program.0))),
            ("color", Fact::Name(color)),
            ("x", Fact::Value(Value::Int(p.column as i64 + 1))),
            ("y", Fact::Value(Value::Int(p.row as i64 + 1))),
            ("rotation", Fact::Value(Value::Int(p.rotation as i64))),
            ("compressed", Fact::Value(Value::Bool(p.compressed || is_compressed(p.id)))),
        ]));
    }
    (out, notes)
}

fn program_colors<'c>(content: &'c Content, program: saito_content_api::EntryHandle) -> Vec<&'c str> {
    match content
        .defs
        .definitions
        .get(Registry::Entry, &content.defs.entry(program).key)
        .map(|d| d.spec.field("colors"))
    {
        Some(saito_content_api::Data::List(colors)) => colors.iter().filter_map(|c| c.str()).collect(),
        _ => Vec::new(),
    }
}

/// Convert an EXE3 [`Style`] (`StyleType` × `Element`) into a setup fact record
/// (`{ style = <name>, element = <name> }`).
pub fn style_fact<'c>(style: Style) -> Fact<'c> {
    Fact::Record(vec![
        ("style", Fact::Name(style.kind.name())),
        ("element", Fact::Name(style.element.name())),
    ])
}
