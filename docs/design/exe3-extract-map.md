# EXE3 Asset & Extraction Map (Verified Across All 4 Real ROMs)

All addresses, table layouts, and format details below were reverse-engineered and verified directly against the four real GBA ROMs in **Mega Man Battle Network Legacy Collection Vol. 1**:

| Code | Version / Region | Archive Path | Checksum Addend |
| :--- | :--- | :--- | :--- |
| `A6BE` | Mega Man Battle Network 3 White (US) — **base** | `exe/data/exe3.dat` → `exe3/rom_e.srl` | `+0x16` |
| `A3XE` | Mega Man Battle Network 3 Blue (US) | `exe/data/exe3b.dat` → `exe3b/rom_b_e.srl` | `+0x22` |
| `A6BJ` | Rockman EXE 3 (White, Japan) | `exe/data/exe3.dat` → `exe3/rom.srl` | `+0x16` |
| `A3XJ` | Rockman EXE 3 Black (Japan) | `exe/data/exe3b.dat` → `exe3b/rom_b.srl` | `+0x22` |

### Code Address Shifts Across the Four ROMs
- **Battle & Custom core (`0x08000000..0x08016000`)**: `A3XE == A6BE` (shift `0`); `A6BJ == A3XJ` (shift `-0x6C` to `-0x9C` relative to US).
- **System / BG / Chatbox (`0x08022000..0x08048000`)**: `A3XE == A6BE - 0x18`, `A6BJ == A6BE + 0x160`, `A3XJ == A6BE + 0x148`.
- **Launcher HD Chip Art**: `launcher/data/vol1.dat`, EXE3 base hash `0xf188_d12f` (already supported in `saito-extract`).

---

## 1. Master 4-ROM Pointer Table

Notation: `[addr] = target` means the ROM stores a 32-bit pointer literal at `addr` pointing to `target` (used with `place.pointer(us, jp)` or `place.pointer_in_each({ a6be, a3xe, a6bj, a3xj })`).

| Asset / Table | `A6BE` (White US) | `A3XE` (Blue US) | `A6BJ` (White JP) | `A3XJ` (Black JP) | Notes |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `SpritePointersList` | `[08002704]=0802477C` | `[08002704]=08024764` | `[080026CC]=080248DC` | `[080026CC]=080248C4` | 9 categories (`0x00..=0x20`) |
| `ScrollCBs` (code base) | `08001E9C` | `08001E9C` | `08001E64` | `08001E64` | `place.stated` |
| `FieldPalettes` (DMA[0]) | `[08005FEC]=08703340` | `[08005FEC]=08702D2C` | `[08005F80]=08704138` | `[08005F80]=08703BB0` | `0x100` B → `0x02009830` (slots 1..=8) |
| `HudObjTiles` (DMA[1]) | `[08005FF8]=086AEC28` | `[08005FF8]` | `[08005F8C]=086AF980` | `[08005F8C]` | `0x4C0` B → `0x06017000` (enemy HP digits) |
| `HudSpritePal` (DMA[2]) | `[08006004]=086B0168` | `[08006004]` | `[08005F98]=086B0EC0` | `[08005F98]` | `0x40` B → `0x02009100` |
| `HudEnemyPal` (DMA[3]) | `[08006010]=086A6508` | `[08006010]` | `[08005FA4]=086A7260` | `[08005FA4]` | `0x20` B → `0x020090E0` |
| `HudGaugeTiles` (DMA[4]) | `[0800601C]=086AF3E8` | `[0800601C]` | `[08005FB0]=086B0140` | `[08005FB0]` | `0x6C0` B → `0x06008600` (`first_tile=0x30`) |
| `HudHpTiles` (DMA[5]) | `[08006028]=086AF0E8` | `[08006028]` | `[08005FBC]=086AFE40` | `[08005FBC]` | `0x300` B → `0x0600A000` (`first_tile=0x100`) |
| `FieldTilesLZ77` | `[08006054]=08701240` | `[08006054]=08700C2C` | `[08005FE8]=08702038` | `[08005FE8]=08701AB0` | LZ77 → `0x6900` B @ `0x06001620` (`0xB1`) |
| `BgAnimJumpTable` | `[08006070]=08006074` | `[08006070]=08006074` | `[08006004]=08006008` | `[08006004]=08006008` | 30 Thumb handlers calling `0x08001FA0` |
| `PanelBlocks` | `[0800AF58]=087005A0` | `[0800AF58]=086FFF8C` | `[0800AED4]=08701398` | `[0800AED4]=08700E10` | 66 × 32 B (`6*type + 3*owner + row - 1`) |
| `PanelBlockOffsets` | `[0800AF5C]=0800AF60` | `[0800AF5C]=0800AF60` | `[0800AED8]=0800AEDC` | `[0800AED8]=0800AEDC` | 66 × `u16` base-tile addends |
| `HighlightBlock` | `[0800B044]=08700DE0` | `[0800B044]=087007CC` | `[0800AFC0]=08701BD8` | `[0800AFC0]=08701650` | 32 B, base-tile addend `+0x0391` |
| `FrontEdges` | `[0800B0A4]=08700E00` | `[0800B0A4]=087007EC` | `[0800B020]=08701BF8` | `[0800B020]=08701670` | 2 × 32 B (5 entries/owner) |
| `FrontEdgeOffsets` | `[0800B0A8]=0800B0AC` | `[0800B0A8]=0800B0AC` | `[0800B024]=0800B028` | `[0800B024]=0800B028` | 2 × `u16`: `[0x0397, 0x039B]` |
| `PanelPalAnim0` (slot 2) | `[0800B190]=0800B194` | same | `[0800B10C]=0800B110` | same | 9 frames × 14 ticks → `0x02009850` |
| `PanelPalAnim1` (slot 6) | `[0800B1B8]=0800B1BC` | same | `[0800B134]=0800B138` | same | 9 frames × 14 ticks → `0x020098D0` |
| `PanelPalAnim2` (slot 3) | `[0800B1E0]=0800B1E4` | same | `[0800B15C]=0800B160` | same | 6 frames × 14 ticks → `0x02009870` |
| `PanelPalAnim3` (slot 7) | `[0800B1FC]=0800B200` | same | `[0800B178]=0800B17C` | same | 6 frames × 14 ticks → `0x020098F0` |
| `CustomWindowMap` | `[0800EBBC]=086B01A8` | `[0800EBBC]=086B02B4` | `[0800EB38]=086B0F00` | `[0800EB38]=086B10B8` | 15 × 20 map entries |
| `CustomWindowTiles` | `[0800EBC4]=086AFB68` | `[0800EBC4]=086AFC74` | `[0800EB40]=086B08C0` | `[0800EB40]=086B0A78` | `0x5A0` B (45 tiles) → `0x06008020` |
| `CustomFramePal` | `[0800EBD0]=086B0108` | `[0800EBD0]=086B0214` | `[0800EB4C]=086B0E60` | `[0800EB4C]=086B1018` | `0x80` B (4 palettes) → `0x02009930` |
| `CustomAuxTiles` | `[0800EBDC]=086B0400` | `[0800EBDC]=086B050C` | `[0800EB58]=086B1158` | `[0800EB58]=086B1310` | `0xB40` B (90 tiles) → `0x06009000` |
| `CustomIconPal` (+gray) | `[0800EBE8]=086A64C8` | `[0800EBE8]=086A65D4` | `[0800EB64]=086A7220` | `[0800EB64]=086A73D8` | `0x40` B (icon + gray) → `0x02009970` |
| `CustomCursorTiles` | `[0800EBF4]=086B0F40` | `[0800EBF4]=086B104C` | `[0800EB70]=086B1C98` | `[0800EB70]=086B1E50` | `0x80` B (4 tiles) → `0x06016E00` |
| `CustomCursorPal` | `[0800EC00]=086B0168` | `[0800EC00]=086B0274` | `[0800EB7C]=086B0EC0` | `[0800EB7C]=086B1078` | `0x40` B → `0x02009100` |
| `CustomCodes` | `[0800EFD4]=086B16C0` | `[0800EFD4]=086B17CC` | `[0800EF50]=086B2418` | `[0800EF50]=086B25D0` | 28 × `0x40` B (8×16 codes A..Z, `*`, blank) |
| `CustomEmptyIcon` | `[0800F058]=086B1D80` | `[0800F058]=086B1E8C` | `[0800EFD4]=086B2AD8` | `[0800EFD4]=086B2C90` | `0x80` B (2×2 tiles) |
| `CustomSlotCodes` | `[0800F294]=086B0FC0` | `[0800F294]=086B10CC` | `[0800F210]=086B1D18` | `[0800F210]=086B1ED0` | 28 × `0x40` B (16×8 slot codes) |
| `CustomEmblemFrames` | `[0800F348]=0800F338` | same | `[0800F2C4]=0800F2B4` | same | 4 × ptr to 2×3 map |
| `CustomEmblemSteps` | `[0800F34C]=0800F2F8` | same | `[0800F2C8]=0800F274` | same | 16 frame-index bytes |
| `CustomCursorTable` | `[0800F3C0]=0800F3C4` | same | `[0800F33C]=0800F340` | same | `[slot, OK, ADD]` corner tables (EXE3 format!) |
| `CustomRegular0 / 1` | `[0800F488/8C]` | same | `[0800F404/08]` | same | 2 × `0x180` B (Regular chip frame) |
| `HudBannerDigits / Pal` | `[08010794/88]` | same | `[08010704/06F8]` | same | 10+1 digits (`0x40` B ea) & palette |
| `HudBannerTbl/Offs/Font` | `[08010810/14/18]` | same | `[08010780/84/88]` | same | 13 banners, `u16` offsets, `0x40` B glyphs |
| `HudPauseTiles` | `[08010A14]=086A6248` | `[08010A14]=086A6354` | `[08010978]=086A7060` | `[08010978]=086A7218` | `0x280` B (PAUSE tiles) |
| `HudTexts` | `[08010AA0]=087090E0` | `[08010AA0]=08708ACC` | `[08010A04]=0870A0A4` | `[08010A04]=08709B1C` | 51 entries, `u16` offsets, terminator `0xE7` |
| `HudGaugeFrameMap` | `08010BE0` (stated) | `08010BE0` | `08010B44` | `08010B44` | 18 × 2 map entries |
| `HudHpPalettes` | `[08010DE0]=086AFAA8` | `[08010DE0]=086AFBB4` | `[08010D44]=086B0800` | `[08010D44]=086B09B8` | 3 × `0x20` B (normal, heal, low) |
| `HudHpBoxMap` | `[08010DE8]=08010DEC` | same | `[08010D4C]=08010D50` | same | 6 × 2 map entries |
| `HudFont8x16` | `[08010FB0]=08681ADC` | `[08010FB0]=08681BDC` | `[08010F14]=086829F8` | `[08010F14]=08682BA4` | `0x40` B/glyph (US 0x70, JP 0xE4) |
| `ChipTable` | `08011510` (stated) | `08011510` | `08011474` | `08011474` | 385 × `0x20` B records |
| `CustomPicOK / Pal` | `[080150D8/DC]` | same | `[0801503C/40]` | same | `0x700` B (8×7 tiles!) + `0x20` B pal |
| `CustomPicOKPick / Pal` | `[080150D0/D4]` | same | `[08015034/38]` | same | `0x700` B (8×7 tiles!) + `0x20` B pal |
| `CustomPicAdd / Pal` | `[080150EC/E8]` | same | `[08015050/4C]` | same | `0x700` B (8×7 tiles!) + `0x20` B pal |
| `BgAreaF0Table` | `[08022FA8]=080250EC` | `[08022F90]=080250D4` | `[08023108]=0802524C` | `[080230F0]=08025234` | `[[BgAreaF0Table]]` = 30 × 12 B BG entries |
| `ChatboxTiles` | `[080267B8]=08684F8C` | `[080267A0]=0868508C` | `[08026918]=08685EA4` | `[08026900]=08686050` | `0x120` B (9 tiles) → `0x0600D700` |
| `ChatboxPalette` | `[080267C4]=0868433C` | `[080267AC]=0868443C` | `[08026924]=08685258` | `[0802690C]=08685404` | `0x20` B → `0x02009160` |
| `DialogueFont` | `[08026A14]=08679A5C` | `[080269FC]=08679B5C` | `[08026B74]=0867A978` | `[08026B5C]=0867AB24` | **8×16 tiled 4bpp (`0x40` B/glyph)!** |
| `ChatboxArrow` | `[08026C58]=086848BC` | `[08026C40]=086849BC` | `[08026DB8]=086857D8` | `[08026DA0]=08685984` | 3 × `0x80` B (16×16 frames) |
| `ElementIcons` | `[08033128]=087CE958` | `[08033110]=087CE544` | `[080335E0]=087C9D18` | `[080335C8]=087C994C` | `0x80` B per element (2×2 tiles) |
| `ElementIconsPal` | `[08033134]=086A64C8` | `[0803311C]=086A65D4` | `[080335EC]=086A7220` | `[080335D4]=086A73D8` | `0x20` B |
| `NaviCustParts` | `08039420` (stated) | `08039408` | `080398D8` | `080398C0` | 204 × `0x10` B (50 usable programs) |
| `NetBattleStages` | `0803EAB0` (stated) | `0803EA98` | `0803EF68` | `0803EF50` | 8 BGs `[0, 4, 5, 6, 23, 16, 2, 10]` |
| `gSongTable` (M4A) | `08148B6C` | `08148B54` | `08149254` | `0814923C` | 331 songs/SFX |

---

## 2. Detailed Subsystem Formats

### 2.1 Battle Field (`0x08005FCE`, `0x0800AF0A`, `0x0800B0DC`)
- **Panel Types (11 total: `0..=10`)**: `missing` (0), `broken` (1), `normal` (2), `cracked` (3), `poison` (4), `metal` (5), `grass` (6), `ice` (7), `lava` (8), `holy` (9), `sand` (10). Flag words live at `0x08236F84` (copied to IWRAM `0x03006E84`).
- **Field Tiles**: LZ77-compressed (`[0x08006054]`), decompressing to `0x6900` bytes (840 tiles) loaded at VRAM `0x06001620` (`first_tile = 0xB1`, whereas EXE4–6 use `0x06001460` / `0xA3`).
- **Palette Buffer**: EWRAM `0x02009810` (field BG palettes 1..=8 at `0x02009830`).
- **Relative Tile Offsets in Panel/Highlight/Edge Maps**: Unlike EXE4–6 where map entries store final VRAM tile indices, EXE3's `PanelBlocks` (66 × 32 B = 15 entries per panel), `HighlightBlock`, and `FrontEdges` store *relative* tile numbers; rendering adds a `u16` base offset (`PanelBlockOffsets[i]`, `0x0391` for highlight, `FrontEdgeOffsets[owner]` = `0x0397` / `0x039B`). In Luau, `gba.map(rom, addr, count, (0x10000 - add) % 0x10000)` converts these directly to absolute tile numbers with zero Rust changes.
- **Panel Palette Cycling Animations (4 animations, `0x0800B0DC`)**:
  - Slot 2 (`0x02009850`): 9 frames × 14 ticks (`initial_timer = 14`)
  - Slot 6 (`0x020098D0`): 9 frames × 14 ticks (`initial_timer = 14`)
  - Slot 3 (`0x02009870`): 6 frames × 14 ticks (`initial_timer = 14`)
  - Slot 7 (`0x020098F0`): 6 frames × 14 ticks (`initial_timer = 14`)

### 2.2 Battle Backgrounds & 8 NetBattle Stages (`0x08022C28`, `0x0800605C`, `0x0803EAB0`)
- **30 Battle Backgrounds (`0..=29`)** stored in Area `0xF0` table `table_ptr = rom:u32(rom:u32(BgAreaF0Table))`.
- **8 NetBattle Stages (`0x0803EAB0`, weights 8×32 at `0x0803EABC`)**: BG IDs `[0, 4, 5, 6, 23, 16, 2, 10]` (ACDC Net, SciLab Net, Yoka Net, Beach Net, Secret Area, Undernet, Lan's HP, Hospital Comp).
- **12-Byte Entry Format** (`entry = table_ptr + 12 * id`):
  - `+0`: 3-layer tiles header pointer (`3 × { size_words: u32, lz77_off: u32, vram_off: u32 }`). BG1 uses layer 0 (`vram_off = 0` → `first_tile = 0`).
  - `+4`: Palette pointer (`u32` byte size at `+0`, followed by 15-bit GBA colors at `+4`).
  - `+8`: Map archive pointer (`w: u8` at `+0`, `h: u8` at `+1`, layer offsets at `+4/+8/+C`, **LZ77 stream starts at `+0x10`**, not `+0x0C` as in EXE4–6!). BG1 map starts at offset `0` of the decompressed buffer (`w * h` halfwords).
- **Scroll Callbacks** (`ScrollCBs` = `0x08001E9C` US / `0x08001E64` JP):
  - `+0x00`: `{ 0, 0 }` (static)
  - `+0x04`: `{ -8, -4 }` (diagonal 2:1)
  - `+0x20`: `{ 0, -4 }` (up)
  - `+0x34`: `{ 0, 4 }` (down)
  - `+0x48`: `{ 8, 0 }` (right)
  - `+0x5C`: `{ -8, 0 }` (left)
  - `+0x70`: `{ -80, 0 }` (fast left)
  - `+0x88`: `{ 0, 0 }` (static)

### 2.3 Battle HUD & Banners (`0x08010740..0x08010FB0`)
- **8×16 HUD Font**: `[0x08010FB0]` US / `[0x08010F14]` JP, `0x40` bytes/glyph (US `0x70` glyphs, JP `0xE4` glyphs).
- **HUD Text Lines**: `[0x08010AA0]` US / `[0x08010A04]` JP, 51 lines indexed by `u16` offsets, string terminator `0xE7` (`first_control = 0xE4`).
- **Battle Banners (13 banners, `0..=12`)**: `[0x08010810]` points to 13 banner string pointers. Unlike EXE4–6 (which store `(x, y, kind, 20 × glyph_ptr)`), each EXE3 banner is a **`0xFF`-terminated array of glyph-index bytes** indexing a `u16` byte-offset table (`[0x08010814]`) into the banner font (`[0x08010818]`, `0x40` bytes per glyph).
- **Chip Table (`0x08011510` US / `0x08011474` JP, 385 × `0x20` bytes)**:
  - `+0x00..+0x05`: 6 chip codes (`0`='A'..`25`='Z', `26`='*', `0xFF` unused)
  - `+0x06`: element (`0` null, `1` elec, `2` heat, `3` aqua, `4` wood)
  - `+0x07`: rarity (`0..=4`)
  - `+0x08`: family (attack script family)
  - `+0x09`: subfamily
  - `+0x0A`: class/flags (`0x01` mega, `0x02` giga, `0x40` PA, `0x80` dim/time-freeze)
  - `+0x0B`: MB
  - `+0x0C..+0x0D`: power (`u16`)
  - `+0x0E..+0x0F`: library index (`u16`)
  - `+0x14`: chip icon pointer (`0x80` bytes, 2×2 tiles)
  - `+0x18`: chip art pointer (`0x700` bytes, **8×7 tiles**)
  - `+0x1C`: chip art palette pointer (`0x20` bytes)
  - **Version-exclusive Giga chips**: White (`A6BE`/`A6BJ`) owns `304..=308` (`0x130..=0x134`: NavRcycl, AlphArmΣ, Bass, Serenade, Balance); Blue/Black (`A3XE`/`A3XJ`) owns `302, 303, 309..=311` (`0x12E, 0x12F, 0x135..=0x137`: DeltaRay, FoldrBak, DarkAura, AlphArmΩ, Bass+).

### 2.4 Custom Screen (`0x0800EB30..0x0800F490`, `0x08015050`)
- **Window Tiles & Map**: `0x5A0` bytes (45 tiles → `0x06008020`), `15×20` map (`[0x0800EBBC]`), 4 frame palettes (`0x80` bytes at `[0x0800EBD0]`), aux tiles (`0xB40` bytes at `[0x0800EBDC]` containing ADD button, column cells, turn limit, and emblem tiles).
- **Cursor Corner Format (`[0x0800F3C0]` → `[slot, OK, ADD]`)**: Each table holds 2 frames × 4 `u32` words. **Important difference from EXE4–5**: EXE3 packs `x` (signed `i8`) in bits `0..7`, `y` (signed `i8`) in bits `8..15`, and flip flags (`0x1000` hflip, `0x2000` vflip) in bits `16..31` (whereas EXE4–5 pack `y` in bits `0..7` and `x` in bits `16..23`).
- **OK / OK_PICKED / ADD Pictures (`[0x080150D8/DC]`, `[0x080150D0/D4]`, `[0x080150EC/E8]`)**: Each is **8×7 tiles (`64×56` px, `0x700` bytes)** + 16-color palette, matching EXE3 chip art.
- **Regular Chip Frame**: Two `0x180`-byte frames at `[0x0800F488]` and `[0x0800F48C]`.
- **Window Emblem**: 4 frame maps (`[0x0800F348]` → 4 pointers × 6 entries, 2×3) and 16-byte step sequence (`[0x0800F34C]`).

### 2.5 Sprites & Portraits (`SpritePointersList`)
- 9 categories (`0x00..=0x20`), same archive format as EXE4–6 (bit 31 = LZ77 compressed with 4-byte size header).
- Category `0x00` (1 slot): overworld.
- Categories `0x04..=0x1C` (41, 19, 58, 102, 28, 105, 104 slots): battle sprites. **Battle MegaMan is `"04-00"`** (styles `"04-01"`..`"04-07"`).
- Category `0x20`: dialogue portraits (MegaMan `"20-00"`, Lan `"20-01"`).

### 2.6 Save File & Battle Records (`crates/exe3-compat`)
- **Save File (`0x57B0` bytes, `0x080470F0..0x080471E0`, `0x08132640`)**: Unencrypted, unshifted raw dump of EWRAM `0x02000000..0x020057B0`.
  - Game Name: `0x1E00` (`"ROCKMANEXE3 20021002"`, 20 bytes).
  - Checksum: `0x1DD8` (`u32` LE) = byte sum of `0x0000..0x57B0` (excluding `0x1DD8..0x1DDC`, **including byte 0 in both US and JP**) `+ 0x16` (White) or `+ 0x22` (Blue/Black).
  - Folders: `0x1410` (3 × `0x78` B = 30 × 4 B: `id: u16` at `+0`, `code: u8` at `+2`; Regular chip is NOT rotated to slot 0).
  - Style: `0x1881` (`(style_type << 3) | element`), Equipped Folder `0x1882`, Regular Memory `0x1898`, Active Regular `0x189C`, Per-Folder Regular `0x189D..0x189F`, HP/MaxHP `0x18A0`/`0x18A2`, HP Memory count `0x1A20` (`base_max_hp = 100 + 20 * count`).
  - NaviCust: `0x1300` (25 × 8 B list), `0x1D90` (5×5 grid, 1-based), compression learned flags at event flag `0x1700 + part_id` (`EVENT_FLAGS = 0x0030`).
  - NaviStats: `0x5770` (`0x40` bytes).
- **Hand Block (`0x24` bytes, `0x02034060 + 0x24*side`)**: `6 × u16` IDs (`0xFFFF` empty), `6 × u16` base damage, `6 × u16` bonus damage.
- **Link Record (`0x100` bytes payload, `0x02036830 + 0x110*side + 0x10`)**: `+0x04` style byte, `+0x08` HP, `+0x0A` MaxHP, `+0x0C` RNG2, `+0x10..+0x50` NaviStats, `+0x77..+0x9F` 0x28-byte extra block.

---

## 3. `saito-extract` & Renderer Primitive Gaps Discovered

Three engine/extractor primitives assume EXE4–6 formats that differ in EXE3. All three are documented here for Track B / follow-up integration without modifying `saito-extract` on Track A:

1. **Gap #1 — `gfx.anims(rom, list, palette_buffer)` (Background GFX Animations)**:
   - `saito-extract`'s `gfx.anims` expects a null-terminated pointer list (`u32[]`) AND interprets `cmd == 0` (palette animation) `param1` as a **byte count** (`param1 / 32`).
   - EXE3 does not store per-BG animation pointer lists: instead, `BgAnimJumpTable` (`[0x08006070]`) dispatches to per-BG Thumb functions that call `bl 0x08001FA0` with individual `GfxAnim` struct pointers. Moreover, EXE3's `cmd == 0` stores `param1` in **32-bit words** (`8` = 1 palette of 32 bytes, so `8 / 32` evaluates to `0` palettes in `saito-extract`). (`cmd == 4` tile animations and `cmd == 0xC` palette shift in BG 4 match EXE4–6.)
   - **Recommended fix in `saito-extract`**: Add `gfx.anim(rom, ptr, palette_buffer, word_counted_palettes?)` (or support a `word_units: boolean` option) so `content/exe3/extract/stage.luau` can extract all animated EXE3 battle backgrounds cleanly. Currently `stage.luau` leaves `anims = {}` to avoid crashing `gfx.anims`.

2. **Gap #2 — `font.dialogue(...)` (8×16 Tiled Dialogue Font)**:
   - `saito-extract`'s `font.dialogue` hardcodes EXE4–6's `0x60`-byte linear 16×12 glyph layout plus a variable-width `advances` table.
   - EXE3's dialogue font (`[0x08026A14]` US / `[0x08026B74]` JP) uses **8×16 tiled 4bpp glyphs (`0x40` bytes/glyph, fixed 8 px advance, no advances table)** — the exact same binary layout as the 8×16 HUD font!
   - **Recommended fix in `saito-extract`**: Add `font.dialogue_tiled_8x16(rom, glyphs, count, chars)` (or an `8x16` mode on `font.dialogue`). Currently `content/exe3/extract/hud.luau` extracts the HUD 8×16 font normally and omits `dialogue_font` until the primitive supports 8×16 tiled dialogue glyphs.

3. **Gap #3 — Custom Screen 8×7 (56-Tile) Chip Art & OK/ADD Pictures**:
   - EXE3 chip art (`chip[+0x18]`) and Custom Screen OK / OK_PICKED / ADD pictures (`0x08015050`) are **8×7 tiles (`64×56` px, `0x700` bytes = 56 tiles)** rather than EXE4–6's 7×6 tiles (`56×48` px, `0x540` bytes = 42 tiles).
   - `saito-extract`'s `Picture` model accepts arbitrary `Tiles`, so `content/exe3/extract/custom.luau` already extracts the full `0x700`-byte 8×7 pictures cleanly; the custom screen renderer / layout will need to support 8×7 (`64×56`) picture dimensions when rendering EXE3 packs.
