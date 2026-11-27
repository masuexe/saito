//! EXE3's records in the engine's terms: the 0x40-byte NaviStats block, the
//! field's 11 panel types, the 0x24-byte chip hand block, and the 0x100-byte
//! link record.
//!
//! All offsets and structures are verified against the four Legacy Collection
//! Vol. 1 EXE3 ROMs (`A6BE`, `A3XE`, `A6BJ`, `A3XJ`) and documented in
//! `docs/design/exe3-map.md` and `docs/design/exe3-extract-map.md`.

use crate::{Compat, Style};

/// A side's NaviStats block: 0x40 bytes in EXE3 (`0x02037920 + 0x40 * side`,
/// initialized at `0x0804735E` in A6BE).
pub const NAVI_STATS: usize = 0x40;

/// A side's chip hand block: 0x24 bytes (`0x02034060 + 0x24 * side`,
/// populated at `0x08014E10` in A6BE): 6 chip IDs, 6 base damages, 6 bonus
/// damages (each `u16`).
pub const HAND_BLOCK: usize = 0x24;

/// A side's link record payload: 0x100 bytes (`0x02036830 + 0x110 * side + 0x10`,
/// packed by `0x080071F0` and unpacked by `0x08007288` in A6BE).
pub const LINK_RECORD: usize = 0x100;

/// Number of chip slots in EXE3's hand block (5 selectable + 1 sentinel/extra).
pub const HAND_SLOTS: usize = 6;

/// A side's stats from EXE3's 0x40-byte NaviStats block (`0x02037920 + 0x40 * side`,
/// initialized by `0x0804735E` and copied from the save at `0x5770` / link record `+0x10`).
/// Unmapped bytes remain accessible via `raw`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NaviStats {
    /// +0x01: SuperArmor (`SprArmor`).
    pub super_armor: bool,
    /// +0x02: FloatShoes (`FlotShoe`).
    pub float_shoes: bool,
    /// +0x03: AirShoes (`AirShoes`).
    pub air_shoes: bool,
    /// +0x04: UnderShirt (`UnderSht`).
    pub undershirt: bool,
    /// +0x08: Buster attack level (1..=5; doubled by `buster_mult` in Guts Style).
    pub attack: u8,
    /// +0x09: Buster rapid level (1..=5).
    pub rapid: u8,
    /// +0x0A: Buster charge level (1..=5).
    pub charge: u8,
    /// +0x0D: Shadow Style / shadow-step flag (`0` normal, non-zero shadow).
    pub shadow_flag: u8,
    /// +0x13: Custom screen hand size (default 5; 6 in Custom Style, +Custom1/+Custom2 up to 10).
    pub custom_level: u8,
    /// +0x14: Mega chip folder limit (default 5; 6 in Team Style, +MegFldr1/+MegFldr2).
    pub mega_level: u8,
    /// +0x15: Giga chip folder limit (default 1; +GigFldr1).
    pub giga_level: u8,
    /// +0x1B: Elemental encounter modifier (`0xFF` none; OilBody/Fish/Battery/Jungle).
    pub encounter_element: Option<u8>,
    /// +0x26: Buster attack multiplier (`1` normal, `2` in Guts Style).
    pub buster_mult: u8,
    /// +0x2A: FirstBarrier active at round start (100 HP barrier, id 8).
    pub first_barrier: bool,
    /// +0x2C: Base HP in 5-HP units (`20 + 4 * hp_memory`, i.e. `base_max_hp / 5`).
    pub base_hp_units: u16,
    /// Full raw 0x40-byte block.
    pub raw: [u8; NAVI_STATS],
}

/// Offsets explicitly decoded by [`NaviStats`].
pub const NAMED_OFFSETS: &[usize] = &[
    0x01, 0x02, 0x03, 0x04, 0x08, 0x09, 0x0A, 0x0D, 0x13, 0x14, 0x15, 0x1B, 0x26, 0x2A, 0x2C, 0x2D,
];

/// Decode a side's 0x40-byte EXE3 NaviStats block.
pub fn navi_stats(b: &[u8; NAVI_STATS]) -> NaviStats {
    let u16at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
    let unless_ff = |i: usize| (b[i] != 0xFF).then_some(b[i]);
    NaviStats {
        super_armor: b[0x01] != 0,
        float_shoes: b[0x02] != 0,
        air_shoes: b[0x03] != 0,
        undershirt: b[0x04] != 0,
        attack: b[0x08],
        rapid: b[0x09],
        charge: b[0x0A],
        shadow_flag: b[0x0D],
        custom_level: b[0x13],
        mega_level: b[0x14],
        giga_level: b[0x15],
        encounter_element: unless_ff(0x1B),
        buster_mult: b[0x26],
        first_barrier: b[0x2A] != 0,
        base_hp_units: u16at(0x2C),
        raw: *b,
    }
}

/// A panel as EXE3 stores it: panel type number (`0..=10`) and owner (`0` or `1`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panel {
    pub number: u8,
    pub alliance: u8,
}

/// Validate and construct an EXE3 panel (11 panel types `0..=10`, owner `0` or `1`).
pub fn panel(number: u8, alliance: u8) -> Result<Panel, String> {
    if alliance > 1 {
        return Err(format!("panel owner {alliance}"));
    }
    if number > 10 {
        return Err(format!("panel type {number}: EXE3's are 0 to 10"));
    }
    Ok(Panel { number, alliance })
}

/// A chip in a hand or selection: its table id and, where compat names it, its key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chip {
    pub id: u16,
    pub key: Option<String>,
}

/// A side's 0x24-byte EXE3 chip hand block (`0x02034060 + 0x24 * side`, `0x08014E10`):
/// - `+0x00..+0x0C`: 6 × `u16` chip IDs (`0xFFFF` empty)
/// - `+0x0C..+0x18`: 6 × `u16` base damage values
/// - `+0x18..+0x24`: 6 × `u16` bonus damage values (Atk+10, Navi+20, etc.)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandBlock {
    pub ids: [Option<Chip>; HAND_SLOTS],
    pub damage: [u16; HAND_SLOTS],
    pub bonus: [u16; HAND_SLOTS],
}

/// Decode a side's 0x24-byte EXE3 chip hand block.
pub fn hand_block(compat: &Compat, b: &[u8]) -> Result<HandBlock, String> {
    if b.len() != HAND_BLOCK {
        return Err(format!("an EXE3 hand block of {:#x} bytes, not {HAND_BLOCK:#x}", b.len()));
    }
    let u16s = |off: usize| -> [u16; HAND_SLOTS] {
        std::array::from_fn(|i| u16::from_le_bytes([b[off + 2 * i], b[off + 2 * i + 1]]))
    };
    let chip = |id: u16| Chip { id, key: compat.chip(id) };
    Ok(HandBlock {
        ids: u16s(0x00).map(|v| (v != 0xFFFF).then(|| chip(v))),
        damage: u16s(0x0C),
        bonus: u16s(0x18),
    })
}

/// EXE3's 0x100-byte link record payload (`0x02036830 + 0x110 * side + 0x10`),
/// built by `0x080071F0` and unpacked by `0x08007288`:
/// - `+0x04`: style byte `(style_type << 3) | element`
/// - `+0x08`: current HP (`u16`)
/// - `+0x0A`: max HP (`u16`)
/// - `+0x0C`: sender's RNG2 seed (`u32`)
/// - `+0x10..+0x50`: 0x40-byte [`NaviStats`] block
/// - `+0x77..+0x9F`: 0x28-byte extra data block (chip/style/NaviCust metadata)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkRecord {
    pub style_byte: u8,
    pub style: Option<Style>,
    pub hp: u16,
    pub max_hp: u16,
    pub rng2: u32,
    pub navi_stats: NaviStats,
    pub extra: [u8; 0x28],
    pub raw: [u8; LINK_RECORD],
}

impl LinkRecord {
    /// Decode a 0x100-byte EXE3 link record payload.
    pub fn from_bytes(b: &[u8]) -> Result<LinkRecord, String> {
        let raw: [u8; LINK_RECORD] = b
            .try_into()
            .map_err(|_| format!("an EXE3 link record of {:#x} bytes, not {LINK_RECORD:#x}", b.len()))?;
        let u16at = |i: usize| u16::from_le_bytes([raw[i], raw[i + 1]]);
        let u32at = |i: usize| u32::from_le_bytes(raw[i..i + 4].try_into().unwrap());
        let style_byte = raw[0x04];
        let stats: [u8; NAVI_STATS] = raw[0x10..0x10 + NAVI_STATS].try_into().unwrap();
        let extra: [u8; 0x28] = raw[0x77..0x77 + 0x28].try_into().unwrap();
        Ok(LinkRecord {
            style_byte,
            style: Style::unpack(style_byte),
            hp: u16at(0x08),
            max_hp: u16at(0x0A),
            rng2: u32at(0x0C),
            navi_stats: navi_stats(&stats),
            extra,
            raw,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Element, StyleType};

    #[test]
    fn navi_stats_block_decodes_exe3_layout() {
        let mut raw = [0u8; NAVI_STATS];
        raw[0x01] = 1; // super_armor
        raw[0x03] = 1; // air_shoes
        raw[0x04] = 1; // undershirt
        raw[0x08] = 5; // attack
        raw[0x09] = 4; // rapid
        raw[0x0A] = 3; // charge
        raw[0x0D] = 1; // shadow
        raw[0x13] = 6; // custom_level (Custom Style)
        raw[0x14] = 5; // mega_level
        raw[0x15] = 1; // giga_level
        raw[0x1B] = 0xFF; // no encounter element
        raw[0x26] = 2; // buster_mult (Guts)
        raw[0x2A] = 1; // first_barrier
        raw[0x2C..0x2E].copy_from_slice(&200u16.to_le_bytes()); // 1000 HP = 200 * 5

        let s = navi_stats(&raw);
        assert!(s.super_armor);
        assert!(!s.float_shoes);
        assert!(s.air_shoes);
        assert!(s.undershirt);
        assert_eq!((s.attack, s.rapid, s.charge, s.buster_mult), (5, 4, 3, 2));
        assert_eq!((s.custom_level, s.mega_level, s.giga_level), (6, 5, 1));
        assert_eq!((s.shadow_flag, s.encounter_element, s.first_barrier), (1, None, true));
        assert_eq!(s.base_hp_units, 200);
    }

    #[test]
    fn panel_validates_exe3_eleven_types() {
        for n in 0..=10 {
            assert_eq!(panel(n, 0), Ok(Panel { number: n, alliance: 0 }));
            assert_eq!(panel(n, 1), Ok(Panel { number: n, alliance: 1 }));
        }
        assert!(panel(11, 0).is_err());
        assert!(panel(2, 2).is_err());
    }

    #[test]
    fn hand_block_decodes_0x24_bytes() {
        let mut b = [0u8; HAND_BLOCK];
        // Slot 0: Cannon (id 1), base 40, bonus 20
        b[0x00..0x02].copy_from_slice(&1u16.to_le_bytes());
        // Slot 1: Sword (id 0x1E = 30), base 80, bonus 10
        b[0x02..0x04].copy_from_slice(&0x1Eu16.to_le_bytes());
        // Slots 2..6: empty (0xFFFF)
        for i in 2..HAND_SLOTS {
            b[2 * i..2 * i + 2].copy_from_slice(&0xFFFFu16.to_le_bytes());
        }
        b[0x0C..0x0E].copy_from_slice(&40u16.to_le_bytes());
        b[0x0E..0x10].copy_from_slice(&80u16.to_le_bytes());
        b[0x18..0x1A].copy_from_slice(&20u16.to_le_bytes());
        b[0x1A..0x1C].copy_from_slice(&10u16.to_le_bytes());

        let h = hand_block(Compat::exe3(), &b).unwrap();
        assert_eq!(h.ids[0].as_ref().and_then(|c| c.key.as_deref()), Some("cannon"));
        assert_eq!(h.ids[1].as_ref().and_then(|c| c.key.as_deref()), Some("sword"));
        assert!(h.ids[2..].iter().all(Option::is_none));
        assert_eq!((h.damage[0], h.bonus[0]), (40, 20));
        assert_eq!((h.damage[1], h.bonus[1]), (80, 10));
        assert!(hand_block(Compat::exe3(), &b[..0x20]).is_err());
    }

    #[test]
    fn link_record_decodes_0x100_bytes() {
        let mut raw = [0u8; LINK_RECORD];
        let style = Style { kind: StyleType::Custom, element: Element::Aqua };
        raw[0x04] = style.pack();
        raw[0x08..0x0A].copy_from_slice(&920u16.to_le_bytes());
        raw[0x0A..0x0C].copy_from_slice(&1000u16.to_le_bytes());
        raw[0x0C..0x10].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        raw[0x10 + 0x13] = 6; // custom_level
        raw[0x10 + 0x26] = 1; // buster_mult
        raw[0x77] = 0xAA;
        raw[0x9E] = 0x55;

        let rec = LinkRecord::from_bytes(&raw).unwrap();
        assert_eq!(rec.style, Some(style));
        assert_eq!((rec.hp, rec.max_hp, rec.rng2), (920, 1000, 0xDEAD_BEEF));
        assert_eq!((rec.navi_stats.custom_level, rec.navi_stats.buster_mult), (6, 1));
        assert_eq!((rec.extra[0], rec.extra[0x27]), (0xAA, 0x55));
        assert!(LinkRecord::from_bytes(&raw[..0x80]).is_err());
    }
}
