//! Optional battle background catalogs and ROM extractors for non-playable
//! Legacy Collection Vol. 1 titles (`exe1` and `exe2`).
//!
//! Both EXE1 (`AREE`/`AREJ`, 17 battle backgrounds) and EXE2 (`AE2E`/`AE2J`,
//! 33 battle backgrounds) share the same Area `0xF0` 12-byte background table
//! `(gfx_ptr, pal_ptr, map_ptr)` and per-background Thumb handler jump table
//! architecture used by EXE3:
//! - Layer-0 LZ77 4bpp tiles (`size_words`, `lz_off`, `vram_off`).
//! - LZ77 16-bit tilemaps at `map_ptr + 0x10`; `46x36` maps (all EXE1 entries
//!   and EXE2 `#0`) are cropped to their top-left `32x32` GBA BG1 screenblock
//!   so hardware scrolling and widescreen rendering wrap seamlessly.
//! - `+4`-header BGR555 palettes (masked with `& 0x7FFF`). Entries whose map
//!   uses a non-zero palette slot (EXE1 `#1` School Comp uses slot 6) load that
//!   slot into palette 0 and normalize map entries to slot 0 so they never
//!   collide with the battle field's panel palettes `1..=8`.
//! - Per-background Thumb handlers installing word-counted `GfxAnim` structs
//!   (`gfx::gfx_anim(..., word_units = true)`) and BG1 scroll callbacks.
//!
//! When a title later gains a full playable `content/<game>` pack, callers
//! prefer the playable pack's locales and recipe automatically, allowing its
//! entry here to be removed without changing any `<game>:<key>` background IDs.

use crate::bgm::EXTRA_BGM_GAMES;
use crate::gfx::gfx_anim;
use crate::rom::Rom;
use crate::source::RomSource;
use saito_assets::{AnimTarget, Background, GfxAnim, MapEntry, Palette, Tiles};
use std::collections::BTreeMap;

/// One battle background entry for an optional non-playable title.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtraBackgroundSpec {
    /// Area `0xF0` background ID in the original ROM.
    pub id: u8,
    /// Stable canonical background key (used as `<game>:<key>` in `BackgroundPool`).
    pub key: &'static str,
    /// Paired battle BGM key in `crate::bgm::gba_bgm_tracks(game)`.
    pub music: &'static str,
}

/// Optional non-playable titles whose battle backgrounds can be extracted
/// directly from `RomSource` (e.g. Legacy Collection Vol. 1 `exe1.dat` / `exe2j.dat`).
pub const EXTRA_BG_GAMES: &[(&str, &[ExtraBackgroundSpec])] = &[
    ("exe1", EXE1_BACKGROUNDS),
    ("exe2", EXE2_BACKGROUNDS),
];

/// Complete EXE1 (BN1) battle background catalog (17 entries, Area `0xF0` IDs `0..=16`).
pub const EXE1_BACKGROUNDS: &[ExtraBackgroundSpec] = &[
    ExtraBackgroundSpec { id: 0, key: "comp", music: "operation" },
    ExtraBackgroundSpec { id: 1, key: "school-comp", music: "operation" },
    ExtraBackgroundSpec { id: 2, key: "waterworks-comp", music: "operation" },
    ExtraBackgroundSpec { id: 3, key: "traffic-comp", music: "operation" },
    ExtraBackgroundSpec { id: 4, key: "power-plant-comp", music: "operation" },
    ExtraBackgroundSpec { id: 5, key: "www-comp", music: "net-battle" },
    ExtraBackgroundSpec { id: 6, key: "internet", music: "operation" },
    ExtraBackgroundSpec { id: 7, key: "lans-pc", music: "operation" },
    ExtraBackgroundSpec { id: 8, key: "dexs-pc", music: "operation" },
    ExtraBackgroundSpec { id: 9, key: "mayls-pc", music: "operation" },
    ExtraBackgroundSpec { id: 10, key: "yais-pc", music: "operation" },
    ExtraBackgroundSpec { id: 11, key: "sals-comp", music: "net-battle" },
    ExtraBackgroundSpec { id: 12, key: "gov-comp", music: "operation" },
    ExtraBackgroundSpec { id: 13, key: "restaurant-comp", music: "operation" },
    ExtraBackgroundSpec { id: 14, key: "antique-comp", music: "net-battle" },
    ExtraBackgroundSpec { id: 15, key: "fish-stand-comp", music: "net-battle" },
    ExtraBackgroundSpec { id: 16, key: "rocket-comp", music: "final-battle" },
];

/// Complete EXE2 (BN2) battle background catalog (33 entries, Area `0xF0` IDs `0..=32`).
pub const EXE2_BACKGROUNDS: &[ExtraBackgroundSpec] = &[
    ExtraBackgroundSpec { id: 0, key: "comp", music: "virus-battle" },
    ExtraBackgroundSpec { id: 1, key: "gas-comp", music: "virus-battle" },
    ExtraBackgroundSpec { id: 2, key: "bomb-comp", music: "virus-battle" },
    ExtraBackgroundSpec { id: 3, key: "mother-comp", music: "virus-battle" },
    ExtraBackgroundSpec { id: 4, key: "mother-comp-dark", music: "virus-battle" },
    ExtraBackgroundSpec { id: 5, key: "castle-comp", music: "virus-battle" },
    ExtraBackgroundSpec { id: 6, key: "castle-comp-2", music: "virus-battle" },
    ExtraBackgroundSpec { id: 7, key: "castle-comp-3", music: "virus-battle" },
    ExtraBackgroundSpec { id: 8, key: "airplane-comp", music: "virus-battle" },
    ExtraBackgroundSpec { id: 9, key: "lans-hp", music: "virus-battle" },
    ExtraBackgroundSpec { id: 10, key: "mayls-hp", music: "virus-battle" },
    ExtraBackgroundSpec { id: 11, key: "yais-hp", music: "virus-battle" },
    ExtraBackgroundSpec { id: 12, key: "dexs-hp", music: "virus-battle" },
    ExtraBackgroundSpec { id: 13, key: "ribittas-comp", music: "battle-spirit" },
    ExtraBackgroundSpec { id: 14, key: "stereo-comp", music: "virus-battle" },
    ExtraBackgroundSpec { id: 15, key: "vase-comp", music: "battle-spirit" },
    ExtraBackgroundSpec { id: 16, key: "official-square", music: "virus-battle" },
    ExtraBackgroundSpec { id: 17, key: "official-comp", music: "virus-battle" },
    ExtraBackgroundSpec { id: 18, key: "den-area", music: "virus-battle" },
    ExtraBackgroundSpec { id: 19, key: "koto-area", music: "virus-battle" },
    ExtraBackgroundSpec { id: 20, key: "koto-undernet", music: "virus-battle" },
    ExtraBackgroundSpec { id: 21, key: "yumland-area", music: "virus-battle" },
    ExtraBackgroundSpec { id: 22, key: "netopia-area", music: "virus-battle" },
    ExtraBackgroundSpec { id: 23, key: "undernet", music: "virus-battle" },
    ExtraBackgroundSpec { id: 24, key: "netopia-square", music: "virus-battle" },
    ExtraBackgroundSpec { id: 25, key: "den-square", music: "virus-battle" },
    ExtraBackgroundSpec { id: 26, key: "apart-comp", music: "virus-battle" },
    ExtraBackgroundSpec { id: 27, key: "under-square", music: "battle-spirit" },
    ExtraBackgroundSpec { id: 28, key: "gospel-server", music: "battle-spirit" },
    ExtraBackgroundSpec { id: 29, key: "gospel-server-dark", music: "battle-spirit" },
    ExtraBackgroundSpec { id: 30, key: "bug-comp", music: "battle-spirit" },
    ExtraBackgroundSpec { id: 31, key: "www-area", music: "battle-spirit" },
    ExtraBackgroundSpec { id: 32, key: "gospel-core", music: "final-battle" },
];

/// Built-in optional background catalog for `game` (`"exe1"` / `"exe2"`).
pub fn extra_backgrounds(game: &str) -> &'static [ExtraBackgroundSpec] {
    EXTRA_BG_GAMES
        .iter()
        .find(|&&(g, _)| g == game)
        .map_or(&[], |&(_, specs)| specs)
}

/// Paired battle BGM track key for an optional title's background, if known.
pub fn extra_background_music(game: &str, key: &str) -> Option<&'static str> {
    extra_backgrounds(game)
        .iter()
        .find(|s| s.key == key)
        .map(|s| s.music)
}

/// Whether `source` contains at least one supported ROM for optional title `game`.
pub fn has_extra_bg_rom(source: &RomSource, game: &str) -> bool {
    let Some(&(_, codes)) = EXTRA_BGM_GAMES.iter().find(|&&(g, _)| g == game) else {
        return false;
    };
    let wanted: Vec<String> = codes.iter().map(|c| (*c).to_string()).collect();
    let found = source.find(&wanted).unwrap_or_else(|(_, found)| found);
    found.iter().any(|r| rom_layout(game, &r.code).is_some())
}

/// Extract all battle backgrounds for optional title `game` (`"exe1"` or `"exe2"`)
/// from `source`. Returns an error if no matching ROM is found or extraction fails.
pub fn extract_extra_backgrounds(source: &RomSource, game: &str) -> Result<BTreeMap<String, Background>, String> {
    let specs = extra_backgrounds(game);
    if specs.is_empty() {
        return Err(format!("no extra background catalog for {game}"));
    }
    let Some(&(_, codes)) = EXTRA_BGM_GAMES.iter().find(|&&(g, _)| g == game) else {
        return Err(format!("no ROM codes declared for {game}"));
    };
    let wanted: Vec<String> = codes.iter().map(|c| (*c).to_string()).collect();
    let roms = source.find(&wanted).unwrap_or_else(|(_, found)| found);
    if roms.is_empty() {
        return Err(format!("no {game} ROM found in {source}"));
    }
    let mut last_err = format!("no supported {game} ROM found in {source}");
    for found in roms {
        let Some(layout) = rom_layout(game, &found.code) else {
            last_err = format!("unsupported {game} ROM code {}", found.code);
            continue;
        };
        let bytes = match found.read() {
            Ok(b) => b,
            Err(e) => {
                last_err = e.to_string();
                continue;
            }
        };
        let rom = Rom(bytes);
        return extract_from_rom(&rom, layout, specs);
    }
    Err(last_err)
}

#[derive(Clone, Copy)]
struct RomLayout {
    area_f0_table: u32,
    anim_jump_table: u32,
    anim_fn: u32,
    scroll_fn: u32,
    scrollers_base: u32,
    palette_buffer: u32,
    scroller_steps: &'static [(u32, (i32, i32))],
}

const EXE1_SCROLLER_STEPS: &[(u32, (i32, i32))] = &[
    (0x00, (-8, -4)),
    (0x20, (0, -4)),
    (0x34, (-8, 0)),
    (0x48, (8, 0)),
    (0x5C, (-32, 0)),
    (0x6C, (0, 0)),
];

const EXE2_SCROLLER_STEPS: &[(u32, (i32, i32))] = &[
    (0x00, (-8, -4)),
    (0x20, (0, -4)),
    (0x34, (0, 32)),
    (0x48, (0, 0)),
];

fn rom_layout(game: &str, code: &str) -> Option<RomLayout> {
    match (game, code) {
        ("exe1", "AREE") => Some(RomLayout {
            area_f0_table: 0x0801_323C,
            anim_jump_table: 0x0800_5320,
            anim_fn: 0x0800_1D20,
            scroll_fn: 0x0800_1BB8,
            scrollers_base: 0x0800_1C4C,
            palette_buffer: 0x0200_6CD0,
            scroller_steps: EXE1_SCROLLER_STEPS,
        }),
        ("exe1", "AREJ") => Some(RomLayout {
            area_f0_table: 0x0801_31C0,
            anim_jump_table: 0x0800_5310,
            anim_fn: 0x0800_1D10,
            scroll_fn: 0x0800_1BA8,
            scrollers_base: 0x0800_1C3C,
            palette_buffer: 0x0200_6CD0,
            scroller_steps: EXE1_SCROLLER_STEPS,
        }),
        ("exe2", "AE2E") => Some(RomLayout {
            area_f0_table: 0x0801_FA74,
            anim_jump_table: 0x0800_53F0,
            anim_fn: 0x0800_1A70,
            scroll_fn: 0x0800_1890,
            scrollers_base: 0x0800_191C,
            palette_buffer: 0x0200_9090,
            scroller_steps: EXE2_SCROLLER_STEPS,
        }),
        ("exe2", "AE2J") => Some(RomLayout {
            area_f0_table: 0x0801_F900,
            anim_jump_table: 0x0800_53EC,
            anim_fn: 0x0800_1A70,
            scroll_fn: 0x0800_1890,
            scrollers_base: 0x0800_191C,
            palette_buffer: 0x0200_9090,
            scroller_steps: EXE2_SCROLLER_STEPS,
        }),
        _ => None,
    }
}

fn mask_palette(p: Palette) -> Palette {
    p.map(|c| c & 0x7FFF)
}

fn read_palette(rom: &Rom, addr: u32) -> Option<Palette> {
    if !rom.contains(addr) || !rom.has(addr, 32) {
        return None;
    }
    let bytes = rom.bytes(addr, 32);
    Some(std::array::from_fn(|i| {
        u16::from_le_bytes([bytes[2 * i], bytes[2 * i + 1]]) & 0x7FFF
    }))
}

fn parse_handler(rom: &Rom, handler: u32, layout: RomLayout) -> (Vec<GfxAnim>, (i32, i32)) {
    let mut pc = handler & !1;
    let mut r0: Option<u32> = None;
    let mut r1: Option<u32> = None;
    let mut anims = Vec::new();
    let mut scroll = (0, 0);

    for _ in 0..64 {
        if !rom.contains(pc) || !rom.has(pc, 2) {
            break;
        }
        let h = rom.u16(pc);
        if (h & 0xFF00) == 0x4800 {
            let pool = ((pc + 4) & !3) + u32::from(h & 0xFF) * 4;
            if rom.has(pool, 4) {
                r0 = Some(rom.u32(pool));
            }
            pc += 2;
        } else if (h & 0xFF00) == 0x4900 {
            let pool = ((pc + 4) & !3) + u32::from(h & 0xFF) * 4;
            if rom.has(pool, 4) {
                r1 = Some(rom.u32(pool));
            }
            pc += 2;
        } else if (h & 0xF800) == 0xF000 && rom.has(pc + 2, 2) {
            let h2 = rom.u16(pc + 2);
            if (h2 & 0xF800) == 0xF800 {
                let mut imm = (u32::from(h & 0x7FF) << 12) | (u32::from(h2 & 0x7FF) << 1);
                if imm >= 0x0040_0000 {
                    imm = imm.wrapping_sub(0x0080_0000);
                }
                let target = (pc + 4).wrapping_add(imm);
                if target == layout.anim_fn
                    && let Some(anim_ptr) = r0
                    && let Some(mut anim) = gfx_anim(rom, anim_ptr, layout.palette_buffer, true)
                {
                    for frame in &mut anim.frames {
                        for pal in &mut frame.palettes {
                            *pal = mask_palette(*pal);
                        }
                    }
                    anims.push(anim);
                } else if target == layout.scroll_fn
                    && let Some(cb_ptr) = r1
                {
                    let cb = cb_ptr & !1;
                    if let Some(off) = cb.checked_sub(layout.scrollers_base)
                        && let Some(&(_, step)) = layout.scroller_steps.iter().find(|&&(o, _)| o == off)
                    {
                        scroll = step;
                    }
                }
                pc += 4;
            } else {
                pc += 2;
            }
        } else if (h & 0xFF00) == 0xBD00 || h == 0x4770 || h == 0x46F7 {
            break;
        } else {
            pc += 2;
        }
    }

    (anims, scroll)
}

fn decode_background(rom: &Rom, layout: RomLayout, id: u8) -> Option<Background> {
    let entry = layout.area_f0_table + u32::from(id) * 12;
    let handler_slot = layout.anim_jump_table + u32::from(id) * 4;
    if !rom.has(entry, 12) || !rom.has(handler_slot, 4) {
        return None;
    }

    let gfx_ptr = rom.u32(entry);
    let pal_src = rom.u32(entry + 4);
    let map_src = rom.u32(entry + 8);
    if !rom.contains(gfx_ptr) || !rom.has(gfx_ptr, 12) || !rom.contains(map_src) || !rom.has(map_src, 0x14) {
        return None;
    }

    let size_bytes = (rom.u32(gfx_ptr) as usize).saturating_mul(4);
    let lz_off = rom.u32(gfx_ptr + 4);
    let vram_off = rom.u32(gfx_ptr + 8);
    let raw_tiles = rom.lz77(gfx_ptr + lz_off)?;
    let tiles = Tiles::from_4bpp(&raw_tiles[..size_bytes.min(raw_tiles.len())]);
    let first_tile = ((vram_off & 0xFFFF) / 32) as u16;

    let raw_w = rom.u8(map_src) as usize;
    let raw_h = rom.u8(map_src + 1) as usize;
    let raw_map = rom.lz77(map_src + 0x10)?;
    if raw_w == 0 || raw_h == 0 || raw_map.len() < raw_w * raw_h * 2 {
        return None;
    }

    // Crop 46x36 battle maps (all EXE1 entries and EXE2 #0) to their top-left
    // 32x32 GBA BG1 hardware screenblock so scrolling and widescreen wrap cleanly.
    let (map_w, map_h) = if raw_w == 46 && raw_h == 36 { (32, 32) } else { (raw_w, raw_h) };
    let mut map = Vec::with_capacity(map_w * map_h);
    let mut non_zero_slot: Option<u8> = None;
    for y in 0..map_h {
        for x in 0..map_w {
            let off = (y * raw_w + x) * 2;
            let entry = MapEntry::from_gba(u16::from_le_bytes([raw_map[off], raw_map[off + 1]]));
            if entry.palette != 0 && non_zero_slot.is_none() {
                non_zero_slot = Some(entry.palette);
            }
            map.push(entry);
        }
    }

    // If the map uses a single non-zero palette slot (e.g. EXE1 #1 School Comp uses
    // slot 6), load that slot into palette 0 and normalize entries to palette 0 so it
    // never collides with the battle field's panel palettes 1..=8.
    let pal_slot = non_zero_slot.unwrap_or(0);
    if pal_slot != 0 {
        for entry in &mut map {
            if entry.palette == pal_slot {
                entry.palette = 0;
            }
        }
    }

    let palette = if rom.contains(pal_src) && rom.has(pal_src, 4) {
        let pal_bytes = rom.u32(pal_src) as usize;
        let slot_off = usize::from(pal_slot) * 32;
        let off = if slot_off + 32 <= pal_bytes { slot_off } else { 0 };
        read_palette(rom, pal_src + 4 + off as u32)
    } else {
        None
    };

    let handler = rom.u32(handler_slot);
    let (mut anims, scroll) = parse_handler(rom, handler, layout);
    if pal_slot != 0 {
        for anim in &mut anims {
            if let AnimTarget::Palettes { first, .. } = &mut anim.target
                && *first == pal_slot
            {
                *first = 0;
            }
        }
    }

    Some(Background {
        tiles,
        first_tile,
        map,
        map_width: map_w as u16,
        map_height: map_h as u16,
        palette,
        scroll,
        speeding: None,
        anims,
    })
}

fn extract_from_rom(rom: &Rom, layout: RomLayout, specs: &[ExtraBackgroundSpec]) -> Result<BTreeMap<String, Background>, String> {
    let mut out = BTreeMap::new();
    for spec in specs {
        let bg = decode_background(rom, layout, spec.id)
            .ok_or_else(|| format!("failed to decode background #{} ({})", spec.id, spec.key))?;
        out.insert(spec.key.to_string(), bg);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bgm::{gba_bgm_is_battle_theme, gba_bgm_track};
    use std::collections::HashSet;

    #[test]
    fn catalogs_have_unique_ids_keys_and_valid_battle_music() {
        for &(game, specs) in EXTRA_BG_GAMES {
            assert!(!specs.is_empty(), "{game} catalog empty");
            let mut ids = HashSet::new();
            let mut keys = HashSet::new();
            for spec in specs {
                assert!(ids.insert(spec.id), "{game}: duplicate id {}", spec.id);
                assert!(keys.insert(spec.key), "{game}: duplicate key {}", spec.key);
                assert!(
                    gba_bgm_track(game, spec.music).is_some(),
                    "{game}/{}: unknown music {}",
                    spec.key,
                    spec.music
                );
                assert!(
                    gba_bgm_is_battle_theme(game, spec.music),
                    "{game}/{}: music {} is not a battle theme",
                    spec.key,
                    spec.music
                );
            }
        }
        assert_eq!(EXE1_BACKGROUNDS.len(), 17);
        assert_eq!(EXE2_BACKGROUNDS.len(), 33);
    }

    #[test]
    fn absent_roms_return_false_and_error() {
        let dir = tempfile::tempdir().unwrap();
        let src = RomSource::Copies(dir.path().to_path_buf());
        assert!(!has_extra_bg_rom(&src, "exe1"));
        assert!(!has_extra_bg_rom(&src, "exe2"));
        assert!(!has_extra_bg_rom(&src, "exe9"));
        assert!(extract_extra_backgrounds(&src, "exe1").is_err());
        assert!(extract_extra_backgrounds(&src, "exe2").is_err());
    }

    #[test]
    fn extracts_real_exe1_and_exe2_backgrounds_when_available() {
        let Ok(src) = RomSource::from_env() else { return };
        for &(game, specs) in EXTRA_BG_GAMES {
            if !has_extra_bg_rom(&src, game) {
                continue;
            }
            let bgs = extract_extra_backgrounds(&src, game).unwrap_or_else(|e| panic!("{game}: {e}"));
            assert_eq!(bgs.len(), specs.len(), "{game}: background count mismatch");
            for spec in specs {
                let bg = bgs.get(spec.key).unwrap_or_else(|| panic!("{game}: missing {}", spec.key));
                assert!(!bg.tiles.is_empty(), "{game}/{}: empty tiles", spec.key);
                assert_eq!((bg.map_width, bg.map_height), (32, 32), "{game}/{}: expected 32x32 map", spec.key);
                assert_eq!(bg.map.len(), 32 * 32, "{game}/{}: map len mismatch", spec.key);
                assert!(bg.palette.is_some(), "{game}/{}: missing palette", spec.key);
                assert!(
                    bg.map.iter().all(|e| e.palette == 0),
                    "{game}/{}: map entries should use palette 0",
                    spec.key
                );
            }
            // Spot-check known scroll speeds & animations:
            if game == "exe1" {
                assert_eq!(bgs["internet"].scroll, (-8, -4));
                assert_eq!(bgs["rocket-comp"].scroll, (-32, 0));
                assert!(!bgs["school-comp"].anims.is_empty());
                assert!(!bgs["power-plant-comp"].anims.is_empty());
            } else if game == "exe2" {
                assert_eq!(bgs["den-area"].scroll, (-8, -4));
                assert_eq!(bgs["under-square"].scroll, (0, -4));
                assert_eq!(bgs["gospel-core"].scroll, (0, 32));
                assert_eq!(bgs["gospel-core"].anims.len(), 3);
            }
        }
    }
}
