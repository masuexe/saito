//! An EXE3 save file (the `.sav` an emulator keeps), verified against all four
//! Legacy Collection Vol. 1 ROMs (`A6BE`, `A3XE`, `A6BJ`, `A3XJ`, save routines
//! at `0x080470F0..0x080471E0` and `0x08132640`).
//!
//! Unlike EXE4–6, EXE3's save image is **unmasked and unshifted** — a direct
//! `0x57B0`-byte dump of EWRAM `0x02000000..0x020057B0`:
//! - `0x0030`: Event flags (`flag n` is bit `0x80 >> (n % 8)` at `0x0030 + n / 8`).
//!   NaviCust part compression learned flags sit at `0x1700 + part_id`.
//! - `0x1300`: NaviCust part list (25 entries × 8 bytes: `+0 id`, `+2 col`, `+3 row`,
//!   `+4 rotation`, `+5 compressed`).
//! - `0x1410`: 3 chip folders × `0x78` bytes (30 entries × **4 bytes**: `id: u16` at `+0`,
//!   `code: u8` at `+2`). Unlike EXE4, EXE3 does NOT rotate the Regular chip to index 0;
//!   the battle shuffle places `folder[reg_idx]` at `order[0]`.
//! - `0x1881`: Equipped Style byte `(style_type << 3) | element`.
//! - `0x1882`: Equipped folder index (`0..=2`).
//! - `0x1898`: Regular memory (raw byte).
//! - `0x189C`: Equipped folder's Regular chip index (`0..=29`, or `0xFF` none).
//! - `0x189D`: Per-folder Regular chip indices (3 bytes, one per folder).
//! - `0x18A0`, `0x18A2`: Current HP and Max HP (`u16` LE).
//! - `0x1A20`: HP Memory key-item count (`0x19C0 + 0x60`); base max HP = `100 + 20 * count`.
//! - `0x1D90`: NaviCust 5×5 grid (25 bytes; 1-based index into the list at `0x1300`, `0` empty).
//! - `0x1DD8`: Checksum (`u32` LE) = sum of all `0x57B0` bytes (with `0x1DD8..0x1DDC` zeroed)
//!   `+ 0x16` (White / `A6BE` / `A6BJ`) or `+ 0x22` (Blue / Black / `A3XE` / `A3XJ`).
//!   Note: in EXE3 both US and JP ROMs sum from byte 0 (unlike EXE4 JP which skips byte 0).
//! - `0x1E00`: Game name magic (`b"ROCKMANEXE3 20021002"`, 20 bytes).
//! - `0x5770`: MegaMan's `0x40`-byte NaviStats block.

use crate::{Style, Version};

/// Size of an EXE3 save image in bytes (EWRAM `0x02000000..0x020057B0`).
pub const IMAGE_SIZE: usize = 0x57B0;
const CHECKSUM: usize = 0x1DD8;
const GAME_NAME: usize = 0x1E00;
/// The 20-byte game signature written at `0x1E00` in all 4 EXE3 ROMs.
pub const NAME: &[u8; 20] = b"ROCKMANEXE3 20021002";

/// Version-specific checksum addend (`0x08047140` / `0x08132640`).
pub fn checksum_start(v: Version) -> u32 {
    match v {
        Version::White => 0x16,
        Version::Blue => 0x22,
    }
}

const EVENT_FLAGS: usize = 0x0030;
const NAVICUST: usize = 0x1300;
pub const NAVICUST_PARTS: usize = 25;
const FOLDERS: usize = 0x1410;
pub const FOLDERS_COUNT: usize = 3;
pub const FOLDER_SIZE: usize = 30;
const FOLDER_ENTRY_BYTES: usize = 4;
const FOLDER_BYTES: usize = FOLDER_SIZE * FOLDER_ENTRY_BYTES; // 0x78
const STYLE: usize = 0x1881;
const EQUIPPED_FOLDER: usize = 0x1882;
const REGULAR_MEMORY: usize = 0x1898;
const REGULAR_CHIP: usize = 0x189C;
const REGULAR_CHIPS: usize = 0x189D;
const HP: usize = 0x18A0;
const MAX_HP: usize = 0x18A2;
const HP_MEMORY_ITEM: usize = 0x1A20;
const NAVICUST_GRID: usize = 0x1D90;
pub const NAVICUST_SIZE: usize = 5;
const NAVI_STATS: usize = 0x5770;
const COMPRESSED_FLAG_BASE: u16 = 0x1700;

/// A folder's chip entry: its table ID (`u16`) and code (`0`='A'..`25`='Z', `26`='*').
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FolderChip {
    pub id: u16,
    pub code: u8,
}

/// A NaviCust part in the save's list (`0x1300`, 25 × 8 bytes):
/// `id = 4 * program + variant` (`0` empty), board `column` & `row` (`0..=4`),
/// `rotation` (`0..=3` quarter-turns), and whether it is `compressed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Part {
    pub id: u8,
    pub column: u8,
    pub row: u8,
    pub rotation: u8,
    pub compressed: bool,
}

/// Decode a raw 25 × 8-byte NaviCust part list.
pub fn parts(list: &[u8]) -> [Option<Part>; NAVICUST_PARTS] {
    std::array::from_fn(|i| {
        let p = &list[8 * i..8 * i + 8];
        (p[0] != 0).then_some(Part {
            id: p[0],
            column: p[2],
            row: p[3],
            rotation: p[4],
            compressed: p[5] != 0,
        })
    })
}

/// Compute the EXE3 save checksum over a `0x57B0`-byte image for `version`.
pub fn compute_checksum(image: &[u8], version: Version) -> u32 {
    let total: u32 = image[..IMAGE_SIZE].iter().map(|&b| b as u32).sum();
    let csum_bytes: u32 = image[CHECKSUM..CHECKSUM + 4].iter().map(|&b| b as u32).sum();
    total.wrapping_sub(csum_bytes).wrapping_add(checksum_start(version))
}

/// A verified EXE3 save image and its detected version (`White` or `Blue`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Save {
    image: Vec<u8>,
    pub version: Version,
}

impl Save {
    /// Read and verify an EXE3 `.sav` file: checks length (`>= 0x57B0`), the
    /// `"ROCKMANEXE3 20021002"` signature at `0x1E00`, and the checksum at
    /// `0x1DD8` (which identifies White `+0x16` vs Blue/Black `+0x22`).
    pub fn read(file: &[u8]) -> Result<Save, String> {
        let image = file
            .get(..IMAGE_SIZE)
            .ok_or_else(|| format!("{} bytes: no EXE3 save image (expected at least {IMAGE_SIZE:#x})", file.len()))?
            .to_vec();
        if &image[GAME_NAME..GAME_NAME + NAME.len()] != NAME {
            return Err("not an EXE3 save (its game's name isn't ROCKMANEXE3 20021002)".into());
        }
        let stored = u32::from_le_bytes(image[CHECKSUM..CHECKSUM + 4].try_into().unwrap());
        let version = [Version::White, Version::Blue]
            .into_iter()
            .find(|&v| stored == compute_checksum(&image, v))
            .ok_or_else(|| {
                let raw = compute_checksum(&image, Version::White).wrapping_sub(checksum_start(Version::White));
                format!("checksum {stored:#x} is neither White nor Blue (byte sum {raw:#x})")
            })?;
        Ok(Save { image, version })
    }

    /// Construct a [`Save`] from an already-extracted `0x57B0`-byte image with
    /// an explicit version (checks length and game name, skips checksum).
    pub fn from_image(image: &[u8], version: Version) -> Result<Save, String> {
        let image = image
            .get(..IMAGE_SIZE)
            .ok_or_else(|| format!("{} bytes: no EXE3 save image (expected at least {IMAGE_SIZE:#x})", file_len(image)))?
            .to_vec();
        if &image[GAME_NAME..GAME_NAME + NAME.len()] != NAME {
            return Err("not an EXE3 save image (its game's name isn't ROCKMANEXE3 20021002)".into());
        }
        Ok(Save { image, version })
    }

    /// The raw `0x57B0`-byte save image.
    pub fn image(&self) -> &[u8] {
        &self.image
    }

    fn u16(&self, at: usize) -> u16 {
        u16::from_le_bytes([self.image[at], self.image[at + 1]])
    }

    /// Raw equipped Style byte at `0x1881` (`(style_type << 3) | element`).
    pub fn style_byte(&self) -> u8 {
        self.image[STYLE]
    }

    /// Equipped Style unpacked into [`Style`] (`StyleType` × `Element`).
    pub fn style(&self) -> Option<Style> {
        Style::unpack(self.style_byte())
    }

    /// Equipped folder index (`0..=2`).
    pub fn equipped_folder(&self) -> usize {
        self.image[EQUIPPED_FOLDER] as usize
    }

    /// Folder `f`'s 30 chips (`0x1410 + 0x78 * f`, 4 bytes per entry: `id: u16`, `code: u8`).
    /// Entries with `id == 0` or `code > 26` are treated as empty (`None`).
    pub fn folder(&self, f: usize) -> [Option<FolderChip>; FOLDER_SIZE] {
        assert!(f < FOLDERS_COUNT, "EXE3 has {FOLDERS_COUNT} folders, got {f}");
        let base = FOLDERS + FOLDER_BYTES * f;
        std::array::from_fn(|i| {
            let at = base + FOLDER_ENTRY_BYTES * i;
            let id = self.u16(at);
            let code = self.image[at + 2];
            (id != 0 && code <= 26).then_some(FolderChip { id, code })
        })
    }

    /// Folder `f`'s Regular chip index (`0..=29`), or `None` if `0xFF` / out of range.
    pub fn folder_regular_chip(&self, f: usize) -> Option<usize> {
        if f >= FOLDERS_COUNT {
            return None;
        }
        let i = self.image[REGULAR_CHIPS + f] as usize;
        (i < FOLDER_SIZE).then_some(i)
    }

    /// Equipped folder's active Regular chip index (`0x189C`), or `None` if unset.
    pub fn regular_chip(&self) -> Option<usize> {
        let i = self.image[REGULAR_CHIP] as usize;
        (i < FOLDER_SIZE).then_some(i)
    }

    /// Regular memory capacity (MB).
    pub fn regular_memory(&self) -> u8 {
        self.image[REGULAR_MEMORY]
    }

    /// MegaMan's current HP (`0x18A0`).
    pub fn hp(&self) -> u16 {
        self.u16(HP)
    }

    /// MegaMan's current Max HP including NaviCust bonuses (`0x18A2`).
    pub fn max_hp(&self) -> u16 {
        self.u16(MAX_HP)
    }

    /// Number of HP Memory key items collected (`0x1A20`).
    pub fn hp_memory_count(&self) -> u8 {
        self.image[HP_MEMORY_ITEM]
    }

    /// Base max HP before NaviCust (`100 + 20 * hp_memory_count()`).
    pub fn base_max_hp(&self) -> u16 {
        100 + 20 * self.hp_memory_count() as u16
    }

    /// NaviCust part list (`0x1300`, 25 entries).
    pub fn navicust(&self) -> [Option<Part>; NAVICUST_PARTS] {
        parts(&self.image[NAVICUST..NAVICUST + 8 * NAVICUST_PARTS])
    }

    /// NaviCust 5×5 grid (`0x1D90`), row by row: 0-based index into [`Save::navicust`].
    pub fn navicust_grid(&self) -> [[Option<usize>; NAVICUST_SIZE]; NAVICUST_SIZE] {
        std::array::from_fn(|y| {
            std::array::from_fn(|x| match self.image[NAVICUST_GRID + NAVICUST_SIZE * y + x] {
                0 => None,
                n => Some(n as usize - 1),
            })
        })
    }

    /// Whether the player has learned the compression code for NaviCust `part_id`
    /// (event flag `0x1700 + part_id`).
    pub fn part_compressed_flag(&self, part_id: u8) -> bool {
        self.event_flag(COMPRESSED_FLAG_BASE + part_id as u16)
    }

    /// MegaMan's `0x40`-byte NaviStats block at `0x5770`.
    pub fn navi_stats(&self) -> [u8; crate::codec::NAVI_STATS] {
        self.image[NAVI_STATS..NAVI_STATS + crate::codec::NAVI_STATS].try_into().unwrap()
    }

    /// Test an event flag (`0x0030 + flag / 8`, bit `0x80 >> (flag % 8)`).
    pub fn event_flag(&self, flag: u16) -> bool {
        self.image[EVENT_FLAGS + flag as usize / 8] & (0x80 >> (flag % 8)) != 0
    }
}

fn file_len(b: &[u8]) -> usize {
    b.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Element, StyleType};

    fn sample_image() -> Vec<u8> {
        let mut img = vec![0u8; IMAGE_SIZE];
        img[0] = 0x5A; // non-zero first byte to prove EXE3 includes byte 0 in checksum
        img[GAME_NAME..GAME_NAME + NAME.len()].copy_from_slice(NAME);
        img[STYLE] = Style { kind: StyleType::Guts, element: Element::Heat }.pack();
        img[EQUIPPED_FOLDER] = 1;
        img[REGULAR_MEMORY] = 32;
        img[REGULAR_CHIP] = 2;
        img[REGULAR_CHIPS..REGULAR_CHIPS + 3].copy_from_slice(&[0xFF, 2, 0]);
        img[HP..HP + 2].copy_from_slice(&900u16.to_le_bytes());
        img[MAX_HP..MAX_HP + 2].copy_from_slice(&1000u16.to_le_bytes());
        img[HP_MEMORY_ITEM] = 45; // 100 + 20 * 45 = 1000

        // Folder 1: Cannon A, Cannon B, AirShot1 *
        let f1 = FOLDERS + FOLDER_BYTES;
        img[f1..f1 + 4].copy_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Cannon A
        img[f1 + 4..f1 + 8].copy_from_slice(&[0x01, 0x00, 0x01, 0x00]); // Cannon B
        img[f1 + 8..f1 + 12].copy_from_slice(&[0x04, 0x00, 26, 0x00]); // AirShot1 *

        // NaviCust part 0: SprArmor (program 1, variant 0 => id 4) at col 1, row 2, rot 3, compressed
        img[NAVICUST..NAVICUST + 8].copy_from_slice(&[4, 0, 1, 2, 3, 1, 0, 0]);
        img[NAVICUST_GRID + 5 * 2 + 1] = 1;

        // Mark compression flag for part_id 4 (flag 0x1704)
        let flag = COMPRESSED_FLAG_BASE + 4;
        img[EVENT_FLAGS + flag as usize / 8] |= 0x80 >> (flag % 8);

        // NaviStats block at 0x5770
        img[NAVI_STATS + 0x01] = 1; // super_armor
        img[NAVI_STATS + 0x26] = 2; // buster_mult
        img
    }

    fn signed(mut img: Vec<u8>, v: Version) -> Vec<u8> {
        let csum = compute_checksum(&img, v);
        img[CHECKSUM..CHECKSUM + 4].copy_from_slice(&csum.to_le_bytes());
        img
    }

    #[test]
    fn save_reads_white_and_blue_checksums_and_fields() {
        for v in [Version::White, Version::Blue] {
            let raw = signed(sample_image(), v);
            let s = Save::read(&raw).unwrap();
            assert_eq!(s.version, v);
            assert_eq!(s.style(), Some(Style { kind: StyleType::Guts, element: Element::Heat }));
            assert_eq!(s.equipped_folder(), 1);
            assert_eq!((s.regular_chip(), s.folder_regular_chip(0), s.folder_regular_chip(1), s.folder_regular_chip(2)), (Some(2), None, Some(2), Some(0)));
            assert_eq!((s.regular_memory(), s.hp(), s.max_hp(), s.hp_memory_count(), s.base_max_hp()), (32, 900, 1000, 45, 1000));

            let f = s.folder(1);
            assert_eq!(
                f[..3],
                [
                    Some(FolderChip { id: 1, code: 0 }),
                    Some(FolderChip { id: 1, code: 1 }),
                    Some(FolderChip { id: 4, code: 26 }),
                ]
            );
            assert!(f[3..].iter().all(Option::is_none));

            assert_eq!(
                s.navicust()[0],
                Some(Part { id: 4, column: 1, row: 2, rotation: 3, compressed: true })
            );
            assert!(s.navicust()[1..].iter().all(Option::is_none));
            assert_eq!(s.navicust_grid()[2][1], Some(0));
            assert!(s.part_compressed_flag(4));
            assert!(!s.part_compressed_flag(5));

            let ns = crate::codec::navi_stats(&s.navi_stats());
            assert!(ns.super_armor);
            assert_eq!(ns.buster_mult, 2);
        }
    }

    #[test]
    fn save_rejects_bad_checksum_name_or_size() {
        let mut bad = signed(sample_image(), Version::White);
        bad[0] ^= 1; // changing byte 0 must invalidate checksum!
        assert!(Save::read(&bad).unwrap_err().contains("checksum"));

        let mut bad_name = sample_image();
        bad_name[GAME_NAME] = b'X';
        assert!(Save::read(&signed(bad_name, Version::Blue)).unwrap_err().contains("name"));

        assert!(Save::read(&[0u8; 64]).is_err());
    }

    #[test]
    fn from_image_accepts_unsigned_dump() {
        let s = Save::from_image(&sample_image(), Version::Blue).unwrap();
        assert_eq!((s.version, s.base_max_hp()), (Version::Blue, 1000));
    }
}
