//! On-demand index into Battle Network Legacy Collection Vol. 2's `launcher/data/vol2.dat`
//! for official high-resolution chip artwork (512×551 PNGs, cropped to 512×432 at decode time).
//!
//! `vol2.dat` begins with a little-endian `u32` entry count followed by `count` 12-byte records
//! `(crc32_hash: u32, offset: u32, size: u32)`. Each chip artwork entry's filename ends in
//! `"{rom_chip_id:03}.png"`; because IEEE CRC32 is affine-linear over XOR, any chip ID's hash is
//! derived directly from the game's `000.png` base hash:
//! `base ^ crc32(b"000.png") ^ crc32(format!("{id:03}.png").as_bytes())`.

use saito_assets::HdChipArtIndex;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Known `000.png` ("NO DATA") CRC32 base hashes for Vol. 2 games inside `launcher/data/vol2.dat`.
fn base_hash(game: &str) -> Option<u32> {
    match game {
        "exe4" => Some(0x95e9_bce6),
        "exe5" => Some(0x8292_a8a5),
        "exe6" => Some(0xbb1f_9460),
        _ => None,
    }
}

/// Standard IEEE 802.3 CRC32 (matches `zlib.crc32`).
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Compute the `vol2.dat` entry hash for `game`'s ROM chip `rom_id`.
pub(crate) fn chip_hash(game: &str, rom_id: u16) -> Option<u32> {
    let base = base_hash(game)?;
    let zero = crc32(b"000.png");
    let suffix = format!("{rom_id:03}.png");
    Some(base ^ zero ^ crc32(suffix.as_bytes()))
}

/// Parse a `vol2.dat` table from `reader` into `hash -> (offset, size)`.
fn read_table(mut reader: impl Read) -> std::io::Result<HashMap<u32, (u64, u32)>> {
    let mut count_buf = [0u8; 4];
    reader.read_exact(&mut count_buf)?;
    let count = u32::from_le_bytes(count_buf) as usize;
    // Sanity bound: vol2.dat has ~4729 entries; cap at 200k to avoid bogus allocations.
    if count == 0 || count > 200_000 {
        return Ok(HashMap::new());
    }
    let mut raw = vec![0u8; count * 12];
    reader.read_exact(&mut raw)?;
    let mut map = HashMap::with_capacity(count);
    for chunk in raw.chunks_exact(12) {
        let hash = u32::from_le_bytes(chunk[0..4].try_into().unwrap());
        let off = u32::from_le_bytes(chunk[4..8].try_into().unwrap()) as u64;
        let size = u32::from_le_bytes(chunk[8..12].try_into().unwrap());
        map.insert(hash, (off, size));
    }
    Ok(map)
}

/// Build an on-demand [`HdChipArtIndex`] for `game` from `vol2_dat`, mapping local chip keys to
/// `(offset, size)` in `vol2_dat`. Returns `None` if the game has no Vol. 2 mapping, the archive
/// cannot be read, or no chip entries match.
pub fn index_from_vol2(vol2_dat: &Path, game: &str, chips: &BTreeMap<u16, String>) -> Option<HdChipArtIndex> {
    base_hash(game)?;
    let file = File::open(vol2_dat).ok()?;
    let table = read_table(file).ok()?;
    let mut entries = BTreeMap::new();
    for (&rom_id, key) in chips {
        if rom_id == 0 {
            continue;
        }
        if let Some(hash) = chip_hash(game, rom_id)
            && let Some(&loc) = table.get(&hash)
        {
            let local = saito_content_api::keys::local(key).to_string();
            entries.insert(local, loc);
        }
    }
    if entries.is_empty() {
        return None;
    }
    Some(HdChipArtIndex { archive: vol2_dat.to_path_buf(), entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affine_crc32_matches_known_vol2_hashes() {
        // Verified against Steam MMBN Legacy Collection Vol. 2 launcher/data/vol2.dat:
        // EXE4: 000 = 0x95e9bce6, 001 (Cannon) = 0xa8899556, 002 (HiCannon) = 0xef29ef86, 280 (VidemnDS) = 0xee252fa2.
        assert_eq!(chip_hash("exe4", 0), Some(0x95e9_bce6));
        assert_eq!(chip_hash("exe4", 1), Some(0xa889_9556));
        assert_eq!(chip_hash("exe4", 2), Some(0xef29_ef86));
        assert_eq!(chip_hash("exe4", 280), Some(0xee25_2fa2));
        // EXE5: 000 = 0x8292a8a5, 001 (Cannon) = 0xbff28115.
        assert_eq!(chip_hash("exe5", 0), Some(0x8292_a8a5));
        assert_eq!(chip_hash("exe5", 1), Some(0xbff2_8115));
        // EXE6: 000 = 0xbb1f9460, 313 (Falzar) = 0x060b2788.
        assert_eq!(chip_hash("exe6", 0), Some(0xbb1f_9460));
        assert_eq!(chip_hash("exe6", 313), Some(0x060b_2788));
        assert_eq!(chip_hash("exe1", 1), None);
    }

    #[test]
    fn reads_vol2_table_and_skips_missing_or_zero_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vol2.dat");
        let h1 = chip_hash("exe6", 1).unwrap();
        let h0 = chip_hash("exe6", 0).unwrap();
        let mut buf = Vec::new();
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(&h0.to_le_bytes());
        buf.extend_from_slice(&100u32.to_le_bytes());
        buf.extend_from_slice(&50u32.to_le_bytes());
        buf.extend_from_slice(&h1.to_le_bytes());
        buf.extend_from_slice(&200u32.to_le_bytes());
        buf.extend_from_slice(&1234u32.to_le_bytes());
        std::fs::write(&path, buf).unwrap();

        let mut chips = BTreeMap::new();
        chips.insert(0, "exe6:nodata".to_string());
        chips.insert(1, "exe6:cannon".to_string());
        chips.insert(999, "exe6:missing".to_string());
        let idx = index_from_vol2(&path, "exe6", &chips).expect("should find cannon");
        assert_eq!(idx.entries.len(), 1);
        assert_eq!(idx.entries.get("cannon"), Some(&(200, 1234)));
    }
}
