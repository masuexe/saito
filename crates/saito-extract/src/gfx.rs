//! GFX animation lists (the primitive `gfx.anims`, docs/design/extraction-recipes.md §3.6): a background's
//! palette frames, tile frames (each tile flipped as its word says) and EXE4's palette transform, which the
//! command byte in the data selects.
use crate::rom::Rom;
use saito_assets::*;
use std::collections::HashMap;

/// The animations listed at `list` (a pointer each, up to the first word that isn't one), the palette
/// animations' targets read against the palette buffer at `palette_buffer`.
pub fn gfx_anims(rom: &Rom, list: u32, palette_buffer: u32) -> Vec<GfxAnim> {
    let mut out = Vec::new();
    if list == 0 || !rom.contains(list) {
        return out;
    }
    let mut a = list;
    loop {
        let p = rom.u32(a);
        if p & 0x8000_0000 != 0 || p == 0 || !rom.contains(p) {
            break;
        }
        if let Some(anim) = gfx_anim(rom, p, palette_buffer, false) {
            out.push(anim);
        }
        a += 4;
    }
    out
}

/// A single graphics animation struct at `p`, the palette animations' targets read against the palette
/// buffer at `palette_buffer`. When `word_units` is true, `cmd == 0` palette counts are read in 32-bit
/// words (8 words = 1 palette of 32 bytes, as in EXE3) rather than bytes (32 bytes = 1 palette, EXE4–6).
pub fn gfx_anim(rom: &Rom, p: u32, palette_buffer: u32, word_units: bool) -> Option<GfxAnim> {
    if p & 0x8000_0000 != 0 || p == 0 || !rom.contains(p) || !rom.has(p, 12) {
        return None;
    }
    let (param0, param1) = (rom.u32(p), rom.u32(p + 4));
    let command = rom.u8(p + 8);
    let param2 = rom.u8(p + 10);
    let target = match command {
        0 => {
            let first = param0.checked_sub(palette_buffer)? / 32;
            if first >= 16 {
                return None;
            }
            let count = if word_units { param1 / 8 } else { param1 / 32 };
            AnimTarget::Palettes {
                first: first as u8,
                count: count as u8,
            }
        }
        4 => AnimTarget::Tiles {
            first: ((param1 & 0xFFFF) / 32) as u16,
            count: param2 as u16,
        },
        // A palette transform (EXE4's 0x080024BC): its mode (0 adds, 4 takes
        // away), over the palettes shown (the buffer's copy 0x200 on, which
        // 0x0800258C makes each frame and shifts).
        0xC => {
            let first = param1.checked_sub(palette_buffer + 0x200)? / 32;
            if first >= 16 || !matches!(param0, 0 | 4) {
                return None;
            }
            AnimTarget::PaletteShift { first: first as u8, count: param2, darken: param0 == 4 }
        }
        _ => return None,
    };
    let mut frames = Vec::new();
    let mut seen: HashMap<u32, usize> = HashMap::new();
    let mut jumps = std::collections::HashSet::new();
    let mut loop_to = 0usize;
    let mut e = p + 12;
    let repeat_from = loop {
        if frames.len() > 512 {
            break None;
        }
        match rom.u32(e) {
            0 => break None,
            1 => break Some(loop_to),
            2 => {
                if !jumps.insert(e) {
                    return None;
                }
                let dest = rom.u32(e + 4);
                if let Some(&i) = seen.get(&dest) {
                    break Some(i);
                }
                loop_to = frames.len();
                e = dest;
                continue;
            }
            data => {
                seen.insert(e, frames.len());
                let delay = rom.u32(e + 4) as u16;
                let frame = match target {
                    // (The color, its word's top bit set.)
                    AnimTarget::PaletteShift { .. } => GfxAnimFrame { shift: (data & 0x7FFF) as u16, delay, ..Default::default() },
                    AnimTarget::Palettes { count, .. } => GfxAnimFrame {
                        palettes: palettes_from_bytes(rom.bytes(data, 32 * count as usize)),
                        delay,
                        ..Default::default()
                    },
                    AnimTarget::Tiles { count, .. } => {
                        // Each u16: a tile of the source (bits 0-9) and its
                        // flips (10, 11).
                        let mut tiles = Tiles::default();
                        for i in 0..count as u32 {
                            let w = rom.u16(data + 2 * i);
                            let src =
                                Tiles::from_4bpp(rom.bytes(param0 + 32 * (w & 0x3FF) as u32, 32));
                            let t = src.get(0).unwrap();
                            let (h, v) = (w & 0x400 != 0, w & 0x800 != 0);
                            let flipped: Vec<u8> = (0..64)
                                .map(|k| {
                                    let (x, y) = (k % 8, k / 8);
                                    t[(if v { 7 - y } else { y }) * 8 + if h { 7 - x } else { x }]
                                })
                                .collect();
                            tiles.push(&flipped);
                        }
                        GfxAnimFrame {
                            tiles,
                            delay,
                            ..Default::default()
                        }
                    }
                    AnimTarget::Nothing => unreachable!(),
                };
                frames.push(frame);
                e += 8;
            }
        }
    };
    Some(GfxAnim {
        target,
        frames,
        repeat_from,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn animation_control_cycle_is_rejected() {
        let mut data = vec![0; 20];
        data[0..4].copy_from_slice(&0x0300_1960u32.to_le_bytes());
        data[4..8].copy_from_slice(&32u32.to_le_bytes());
        data[12..16].copy_from_slice(&2u32.to_le_bytes());
        data[16..20].copy_from_slice(&0x0800_000cu32.to_le_bytes());
        assert!(gfx_anim(&Rom(data), 0x0800_0000, 0x0300_1960, false).is_none());
    }

    #[test]
    fn word_units_cmd0_extracts_one_palette_for_eight_words() {
        // Header (12 bytes) + 1 frame entry (8 bytes) + loop cmd (4 bytes) + 1 palette (32 bytes).
        let pal_off = 24u32;
        let mut data = vec![0u8; pal_off as usize + 32];
        let pal_buf = 0x0200_9810u32;
        // Target slot 2: pal_buf + 2 * 32
        data[0..4].copy_from_slice(&(pal_buf + 64).to_le_bytes());
        // param1 = 8 (32-bit words = 1 palette of 32 bytes in EXE3)
        data[4..8].copy_from_slice(&8u32.to_le_bytes());
        // command = 0 (palette animation)
        data[8] = 0;
        // Frame 0: pointer to palette at 0x0800_0018, delay 14
        data[12..16].copy_from_slice(&(0x0800_0000 + pal_off).to_le_bytes());
        data[16..20].copy_from_slice(&14u32.to_le_bytes());
        // Loop control word = 1
        data[20..24].copy_from_slice(&1u32.to_le_bytes());
        // Palette color 0 = 0x1234, color 15 = 0x5678
        data[pal_off as usize..pal_off as usize + 2].copy_from_slice(&0x1234u16.to_le_bytes());
        data[pal_off as usize + 30..pal_off as usize + 32].copy_from_slice(&0x5678u16.to_le_bytes());

        let rom = Rom(data);
        // Byte-counted mode (EXE4–6): 8 / 32 == 0 palettes.
        let bytes_mode = gfx_anim(&rom, 0x0800_0000, pal_buf, false).expect("parses in byte mode");
        assert_eq!(bytes_mode.target, AnimTarget::Palettes { first: 2, count: 0 });
        assert!(bytes_mode.frames[0].palettes.is_empty());

        // Word-counted mode (EXE3): 8 / 8 == 1 palette.
        let words_mode = gfx_anim(&rom, 0x0800_0000, pal_buf, true).expect("parses in word mode");
        assert_eq!(words_mode.target, AnimTarget::Palettes { first: 2, count: 1 });
        assert_eq!(words_mode.repeat_from, Some(0));
        assert_eq!(words_mode.frames.len(), 1);
        assert_eq!(words_mode.frames[0].delay, 14);
        assert_eq!(words_mode.frames[0].palettes.len(), 1);
        assert_eq!(words_mode.frames[0].palettes[0][0], 0x1234);
        assert_eq!(words_mode.frames[0].palettes[0][15], 0x5678);
    }
}
