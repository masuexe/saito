//! A side from an EXE3 save file ([`crate::save`]), whole: MegaMan (the one
//! navi an EXE3 save operates), its equipped Style (`style_type` and
//! `style_element` from `0x1881`), its equipped 30-chip folder (`0x1410 + 0x78 *
//! equipped_folder`) with its Regular chip (`0x189C` / `0x189D..0x189F`), its
//! placed NaviCust programs (`0x1300` list + `0x1D90` 5×5 grid), and what the
//! save brings to the stats (`hp` from HP Memories at `0x1A20`, `reg_up` from
//! `0x1898`).
//!
//! Unimplemented chips or NaviCust programs are gracefully skipped with notes
//! rather than failing the import.

use crate::save::{FOLDERS_COUNT, Save};
use crate::{Compat, Version};
use saito_battle::content::{ChipCode, Content};
use saito_battle::custom::FolderChip;
use saito_battle::rules::Fact;
use saito_content_api::Value;
use saito_match::{Folder, Side, ids};

/// Read an EXE3 save from `file`: accepts full `.sav` SRAM dumps (32KB/64KB
/// with the `0x57B0`-byte image at offset 0), exact signed `0x57B0`-byte save
/// images (auto-detecting White `+0x16` vs Blue/Black `+0x22`), or unsigned raw
/// `0x57B0`-byte images bearing the `"ROCKMANEXE3 20021002"` signature at
/// `0x1E00` (defaulting to White when checksum is absent).
pub fn read(file: &[u8]) -> Result<Save, String> {
    Save::read(file).or_else(|e| Save::from_image(file, Version::White).map_err(|_| e))
}

/// Populate `side` (of a match of `game`, an EXE3 pack) from an EXE3 `save`,
/// returning human-readable notes for anything skipped or left out.
pub fn import(compat: &Compat, content: &Content, game: &str, side: &mut Side, save: &Save) -> Vec<String> {
    let mut notes = Vec::new();
    match ids::navi(content, game, "megaman") {
        Some(navi) => {
            if let Err(e) = side.set_navi(content, navi) {
                notes.push(format!("the save's navi is left out: {e}"));
            }
        }
        None => notes.push(format!("{game} has no MegaMan: the side keeps its navi")),
    }

    let mut left_out: Vec<String> = Vec::new();
    let mut state = |side: &mut Side, field: &str, values: &[Fact]| {
        if let Err(e) = side.set_fact(content, field, values) {
            left_out.push(format!("the save's {field} is left out: {e}"));
        }
    };

    // Version fact if the game's rules declare one.
    if saito_match::facts::takes(content, "version") {
        state(side, "version", &[Fact::Name(save.version.name())]);
    }

    // Equipped Style (save 0x1881: (style_type << 3) | element).
    match save.style() {
        Some(style) => {
            state(side, "style_type", &[Fact::Name(style.kind.name())]);
            state(side, "style_element", &[Fact::Name(style.element.name())]);
        }
        None => {
            notes.push(format!("the save's style byte {:#04x} is invalid: left at normal/null", save.style_byte()));
            state(side, "style_type", &[Fact::Name("normal")]);
            state(side, "style_element", &[Fact::Name("null")]);
        }
    }

    // Equipped 30-chip folder (0x1410 + 0x78 * equipped_folder) and Regular chip.
    let equipped = save.equipped_folder();
    if equipped >= FOLDERS_COUNT {
        notes.push(format!("the save's equipped folder index {equipped} is out of range (0..{FOLDERS_COUNT}): folder left empty"));
        if let Err(e) = side.set_folder(content, &Folder::EMPTY) {
            notes.push(format!("the save's folder is left out: {e}"));
        }
    } else {
        let raw_folder = save.folder(equipped);
        let mut chips = [None; 30];
        let mut nameless: Vec<u16> = Vec::new();
        for (slot, c) in chips.iter_mut().zip(raw_folder) {
            let Some(c) = c else { continue };
            match compat.chip_key(c.id).and_then(|k| ids::chip(content, game, k)) {
                Some(chip) => *slot = Some(FolderChip::new(chip, ChipCode(c.code))),
                None => nameless.push(c.id),
            }
        }
        if !nameless.is_empty() {
            let numbers: Vec<String> = nameless.iter().map(|n| format!("{n:#05x}")).collect();
            notes.push(format!(
                "the save's folder holds chip numbers {game} has no chip for ({}): their entries are left empty",
                numbers.join(", ")
            ));
        }
        let mut regular = save.regular_chip().or_else(|| save.folder_regular_chip(equipped)).map(|i| i as u8);
        if let Some(r) = regular {
            if chips[r as usize].is_none() {
                if let Some(orig) = raw_folder[r as usize] {
                    notes.push(format!(
                        "the save's Regular chip (slot {r}, chip {:#05x}) is not in {game}: Regular chip cleared",
                        orig.id
                    ));
                }
                regular = None;
            }
        }
        if let Err(e) = side.set_folder(content, &Folder { chips, regular, tags: None }) {
            notes.push(format!("the save's folder is left out: {e}"));
        }
    }

    // MegaMan's NaviCust (0x1300 part list + 0x1D90 5x5 grid + 0x1700 compression flags).
    let (programs, nc_notes) = crate::setup::navicust_import(
        content,
        compat,
        game,
        &save.navicust(),
        &save.navicust_grid(),
        |id| save.part_compressed_flag(id),
    );
    state(side, "navicust_programs", &programs);
    notes.extend(nc_notes);

    // Base HP (HP Memories: 100 + 20 * count) and Regular memory (MB).
    state(side, "hp", &[Fact::Value(Value::Int(save.base_max_hp() as i64))]);
    state(side, "reg_up", &[Fact::Value(Value::Int(save.regular_memory() as i64))]);

    notes.extend(left_out);
    notes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::{IMAGE_SIZE, compute_checksum};
    use crate::{Element, Style, StyleType};
    use saito_match::facts::Stated;
    use std::sync::{Arc, OnceLock};

    fn exe3_content() -> Arc<Content> {
        static EXE3: OnceLock<Arc<Content>> = OnceLock::new();
        EXE3.get_or_init(|| Arc::new(saito_match::testing::defined(&["exe3"]).unwrap_or_else(|e| panic!("content/exe3: {e}")))).clone()
    }

    fn build_save(version: Version, style: Style, equipped: u8, regular: u8, sram_size: usize) -> Vec<u8> {
        let mut buf = vec![0u8; sram_size.max(IMAGE_SIZE)];
        buf[0x1E00..0x1E00 + 20].copy_from_slice(crate::save::NAME);
        buf[0x1881] = style.pack();
        buf[0x1882] = equipped;
        buf[0x1898] = 32; // Regular memory
        buf[0x189C] = regular;
        buf[0x189D..0x189D + 3].fill(0xFF);
        if (equipped as usize) < 3 {
            buf[0x189D + equipped as usize] = regular;
        }
        buf[0x18A0..0x18A2].copy_from_slice(&1000u16.to_le_bytes());
        buf[0x18A2..0x18A4].copy_from_slice(&1000u16.to_le_bytes());
        buf[0x1A20] = 45; // 100 + 20 * 45 = 1000 base HP

        // Populate equipped folder with 30 real EXE3 chips:
        // Slot 0: Cannon A (id 1, code 0), Slot 1: Sword L (id 30, code 11),
        // Slot 2: AreaGrab * (id 131, code 26), rest AirShot1 * (id 4, code 26).
        let compat = Compat::exe3();
        let cannon = compat.chip_entry("cannon").unwrap().id;
        let sword = compat.chip_entry("sword").unwrap().id;
        let areagrab = compat.chip_entry("areagrab").unwrap().id;
        let airshot1 = compat.chip_entry("airshot1").unwrap().id;
        let fbase = 0x1410 + 0x78 * (equipped as usize);
        let write_chip = |buf: &mut [u8], slot: usize, id: u16, code: u8| {
            let at = fbase + 4 * slot;
            buf[at..at + 2].copy_from_slice(&id.to_le_bytes());
            buf[at + 2] = code;
        };
        write_chip(&mut buf, 0, cannon, 0);
        write_chip(&mut buf, 1, sword, 11);
        write_chip(&mut buf, 2, areagrab, 26);
        for slot in 3..30 {
            write_chip(&mut buf, slot, airshot1, 26);
        }

        // NaviCust part 0: UnderSht (program 30, variant 0 => id 120, white) at col 2, row 2 (center on command line).
        let undersht_id = 30 * 4;
        buf[0x1300..0x1300 + 8].copy_from_slice(&[undersht_id, 0, 2, 2, 0, 0, 0, 0]);
        buf[0x1D90 + 5 * 2 + 2] = 1; // 1-based index into 0x1300

        // NaviCust part 1: Atk+1 (program 41, variant 0 => id 164, white) at col 1, row 1, compressed via event flag.
        let atk1_id = 41 * 4;
        buf[0x1308..0x1308 + 8].copy_from_slice(&[atk1_id, 0, 1, 1, 0, 0, 0, 0]);
        buf[0x1D90 + 5 + 1] = 2;
        let flag = 0x1700u16 + atk1_id as u16;
        buf[0x0030 + flag as usize / 8] |= 0x80 >> (flag % 8);

        let csum = compute_checksum(&buf, version);
        buf[0x1DD8..0x1DDC].copy_from_slice(&csum.to_le_bytes());
        buf
    }

    #[test]
    fn imports_white_and_blue_saves_including_sram_dumps() {
        let content = exe3_content();
        let compat = Compat::exe3();

        // White save as 64KB .sav SRAM dump with Ground/Heat Style and Regular chip at slot 2.
        let white_bytes = build_save(
            Version::White,
            Style { kind: StyleType::Ground, element: Element::Heat },
            1,
            2,
            0x10000,
        );
        let white_save = read(&white_bytes).unwrap();
        assert_eq!(white_save.version, Version::White);
        let mut white_side = Side::fresh(&content, "exe3").unwrap();
        let notes = import(compat, &content, "exe3", &mut white_side, &white_save);
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(white_side.facts.get(&content, "style_type"), Some(Stated::Variant(Some("ground".into()))));
        assert_eq!(white_side.facts.get(&content, "style_element"), Some(Stated::Variant(Some("heat".into()))));
        assert_eq!(white_side.facts.get(&content, "hp"), Some(Stated::Number(1000)));
        assert_eq!(white_side.facts.get(&content, "reg_up"), Some(Stated::Number(32)));

        let folder = white_side.folder(&content);
        assert_eq!(folder.regular, Some(2));
        let named = |c: Option<FolderChip>| c.map(|c| (ids::local(&content.defs.chip(c.id).key).to_string(), c.code.letter()));
        assert_eq!(
            [named(folder.chips[0]), named(folder.chips[1]), named(folder.chips[2])],
            [
                Some(("cannon".into(), 'A')),
                Some(("sword".into(), 'L')),
                Some(("areagrab".into(), '*')),
            ]
        );

        let Some(Stated::List(nc)) = white_side.facts.get(&content, "navicust_programs") else {
            panic!("{:?}", white_side.facts.get(&content, "navicust_programs"));
        };
        assert_eq!(nc.len(), 2);
        let field = |rec: &Stated, name: &str| {
            let Stated::Record(fields) = rec else { panic!("{rec:?}") };
            fields.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone())
        };
        assert_eq!(
            (field(&nc[0], "x"), field(&nc[0], "y"), field(&nc[0], "color"), field(&nc[0], "compressed")),
            (
                Some(Stated::Number(3)),
                Some(Stated::Number(3)),
                Some(Stated::Variant(Some("white".into()))),
                Some(Stated::Flag(false)),
            )
        );
        assert_eq!(
            (field(&nc[1], "x"), field(&nc[1], "y"), field(&nc[1], "compressed")),
            (Some(Stated::Number(2)), Some(Stated::Number(2)), Some(Stated::Flag(true)))
        );

        // Blue save as raw 0x57B0 image with Shadow/Aqua Style and no Regular chip.
        let blue_bytes = build_save(
            Version::Blue,
            Style { kind: StyleType::Shadow, element: Element::Aqua },
            0,
            0xFF,
            IMAGE_SIZE,
        );
        let blue_save = read(&blue_bytes).unwrap();
        assert_eq!(blue_save.version, Version::Blue);
        let mut blue_side = Side::fresh(&content, "exe3").unwrap();
        let notes = import(compat, &content, "exe3", &mut blue_side, &blue_save);
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(blue_side.facts.get(&content, "style_type"), Some(Stated::Variant(Some("shadow".into()))));
        assert_eq!(blue_side.facts.get(&content, "style_element"), Some(Stated::Variant(Some("aqua".into()))));
        assert_eq!(blue_side.folder(&content).regular, None);
    }

    #[test]
    fn gracefully_notes_unimplemented_chips_and_navicust_parts() {
        let content = exe3_content();
        let compat = Compat::exe3();
        let mut buf = build_save(
            Version::White,
            Style { kind: StyleType::Custom, element: Element::Elec },
            0,
            1,
            IMAGE_SIZE,
        );
        // Replace slot 1 (also the Regular chip) with an unknown chip ID 0x1FF.
        buf[0x1410 + 4..0x1410 + 6].copy_from_slice(&0x1FFu16.to_le_bytes());
        // Add an out-of-table NaviCust part ID (0xF0, beyond the 204 ROM entries) as part 2.
        buf[0x1310..0x1310 + 8].copy_from_slice(&[0xF0, 0, 0, 0, 0, 0, 0, 0]);
        buf[0x1D90] = 3;
        // Re-sign checksum.
        let csum = compute_checksum(&buf, Version::White);
        buf[0x1DD8..0x1DDC].copy_from_slice(&csum.to_le_bytes());

        let save = read(&buf).unwrap();
        let mut side = Side::fresh(&content, "exe3").unwrap();
        let notes = import(compat, &content, "exe3", &mut side, &save);
        assert!(notes.iter().any(|n| n.contains("0x1ff")), "{notes:?}");
        assert!(notes.iter().any(|n| n.contains("Regular chip")), "{notes:?}");
        assert!(notes.iter().any(|n| n.contains("0xf0")), "{notes:?}");
        // The two implemented NaviCust programs (UnderSht + Atk+1) are still preserved.
        let Some(Stated::List(nc)) = side.facts.get(&content, "navicust_programs") else {
            panic!("missing navicust_programs");
        };
        assert_eq!(nc.len(), 2);
        assert_eq!(side.folder(&content).regular, None);
    }
}
