# EXE3 (Battle Network 3): ROM Map, Battle Core Disassembly & Engine Gap Analysis

This document is the technical foundation for adding **Mega Man Battle Network 3 / Rockman EXE 3** (`exe3`) to **saito**.
It extends [exe4-against-exe3.md](exe4-against-exe3.md) with cross-version ROM/asset tables verified directly against all four Legacy Collection Vol. 1 ROMs (`A6BE`, `A3XE`, `A6BJ`, `A3XJ`), Capstone disassembly of EXE3's core battle routines, Capcom's original module taxonomy from Legacy Collection symbols, and a concrete inventory of engine/extraction/UI gaps in `saito`.

---

## 1. Target ROMs & Legacy Collection Vol. 1 Layout

Steam App ID: **`1798010`** (*Mega Man Battle Network Legacy Collection Vol. 1*).
Archives live under `<install>/exe/data/`; official HD chip artwork (`512×551` PNGs, same binary table format as `vol2.dat`) lives in `<install>/launcher/data/vol1.dat` (3,033 entries).

| Code | Archive & Entry | Version / Region | Header Title | Rev | IWRAM Code Copy (`0x08000214` pool) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `A6BE` | `exe3.dat` → `exe3/rom_e.srl` | White, US | `MEGA_EXE3_WH` | 0 | `0x08235F00` → `0x03005E00` (`0x17AC` B) |
| `A3XE` | `exe3b.dat` → `exe3b/rom_b_e.srl` | Blue, US | `MEGA_EXE3_BL` | 0 | `0x08235F00` → `0x03005E00` (`0x17AC` B) |
| `A6BJ` | `exe3.dat` → `exe3/rom.srl` | White, Japan | `ROCKMAN_EXE3` | 1 | `0x08235B00` → `0x03005E00` (`0x17AC` B) |
| `A3XJ` | `exe3b.dat` → `exe3b/rom_b.srl` | Black, Japan | `ROCK_EXE3_BK` | 1 | `0x08235B00` → `0x03005E00` (`0x17AC` B) |

### 1.1 Legacy Collection Vol. 1 HD Chip Art (`vol1.dat`)

Verified affine-linear IEEE CRC32 `000.png` base hashes inside `launcher/data/vol1.dat` (matching `vol2.dat`'s exact `base ^ crc32("000.png") ^ crc32("{id:03}.png")` scheme in `crates/saito-extract/src/hd_art.rs`):

| Game | `000.png` Base Hash | `001.png` (Cannon) Hash | Matched Chip Entries (`1..=512`) | Max Chip ID |
| :--- | :--- | :--- | :--- | :--- |
| `exe1` | `0xdf7e_f9a9` | `0xe21e_d019` | 175 | 198 |
| `exe2` | `0xe6f3_c56c` | `0xdb93_ecdc` | 266 | 270 |
| **`exe3`** | **`0xf188_d12f`** | **`0xcce8_f89f`** | **287** | **308** |

---

## 2. Verified Cross-ROM Asset & Data Tables

All addresses below were verified across all four Vol. 1 ROMs (and cross-checked against `tangobattle/tango`'s `tango-gamesupport-bn3` / `bn3-dataview` and `ssbmars/BBN3`).

| Table / Routine | `A6BE` (White US) | `A3XE` (Blue US) | `A6BJ` (White JP) | `A3XJ` (Black JP) | Notes |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Chip Table** (`0x20` B × 385) | `0x08011510` | `0x08011510` | `0x08011474` | `0x08011474` | 385 valid entries (`1..=312` playable incl. Mega/Giga, `320..=351` PAs) |
| **Chip Names ptrs** | `0x08027AD4` | `0x08027ABC` | `0x08027C34` | `0x08027C1C` | Text archive pointers |
| **Chip Descriptions ptrs** | `0x0800E46C` | `0x0800E46C` | `0x0800E3E8` | `0x0800E3E8` | Text archive pointers |
| **Chip Icon / Elem Palette ptr** | `0x08033134` | `0x0803311C` | `0x080335EC` | `0x080335D4` | Shared icon/element palette |
| **Element Icons ptr** | `0x08033128` | `0x08033110` | `0x080335E0` | `0x080335C8` | Element icon 4bpp tiles |
| **NaviCust Table** (204 parts) | `0x08039420` | `0x08039408` | `0x080398D8` | `0x080398C0` | Ref'd by `0x0803AF6C` (A6BE) / `0x0803B424` (A6BJ) |
| **NaviCust Names ptr** | `0x08027AE4` | `0x08027ACC` | `0x08027C44` | `0x08027C2C` | |
| **NaviCust Descriptions ptr** | `0x0802EA94` | `0x0802EA7C` | `0x0802EF4C` | `0x0802EF34` | |
| **SpritePointersList** | `0x0802477C` | `0x08024764` | `0x080248DC` | `0x080248C4` | Literal at `0x08002704` (US) / `0x080026CC` (JP) |
| **M4A `m4aSongNumStart`** | `0x081335EC` | `0x081336EC` | `0x08133E88` | `0x08133F98` | Detected directly by existing `crates/m4a/src/rom.rs`! |
| **M4A `gSongTable`** (331 entries) | `0x08148B6C` | `0x08148C6C` | `0x08149408` | `0x08149518` | `player_table`: `0x081489EC` / `0x08148AEC` / `0x08149288` / `0x08149398`; sound mode `0x0093D400` |
| **PlaySFX / PlayMusic** | `0x080005B8` / `0x080005C8` | same | same | same | Identical across all 4 ROMs |
| **Round Start Ret** | `0x080059A8` | `0x080059A8` | `0x0800593C` | `0x0800593C` | End of `BattleInit` (`0x0800590C` in A6BE) |
| **Round End Set Win / Loss** | `0x0800946A` / `0x08009472` | same | `0x080093E6` / `0x080093EE` | same | Inside `BattleRoundEnd` (`0x0800942E` in A6BE) |

### 2.1 Sprite Archive Compatibility (`saito-extract/src/sprite.rs`)

EXE3 uses the **exact same Battle Network sprite archive container format** as EXE4/5/6 (`BASE = 4`, `u32` animation offset table, `0x14`-byte frame entries, bit-31 LZ77 compression flag with 4-byte uncompressed size prefix).
`SpritePointersList` holds 9 categories (`+0x00..=+0x20`) with identical slot counts across all 4 ROMs:

| Category | Offset | Slots | `A6BE` Table Addr | Notes |
| :--- | :--- | :--- | :--- | :--- |
| 0 | `+0x00` | 1 | `0x080247A0` | Overworld / special |
| 1 | `+0x04` | 41 | `0x080247A4` | Battle MegaMan / Navis (confirmed valid `sprite.rs` format) |
| 2 | `+0x08` | 19 | `0x08024848` | Battle sprites (LZ77-compressed entries, confirmed valid) |
| 3 | `+0x0C` | 58 | `0x08024894` | Battle sprites (confirmed valid) |
| 4 | `+0x10` | 102 | `0x0802497C` | Battle attacks / effects (confirmed valid) |
| 5 | `+0x14` | 28 | `0x08024B14` | Battle sprites (confirmed valid) |
| 6 | `+0x18` | 105 | `0x08024B84` | Viruses / bosses |
| 7 | `+0x1C` | 104 | `0x08024D28` | Effects / UI sprites (confirmed valid) |
| 8 | `+0x20` | — | `0x08024EC8` | Portraits / mugshots (confirmed valid) |

---

## 3. Core Battle Systems Disassembly Summary (`A6BE`)

Capcom's original EXE3 source module breakdown (from `StraDaMa/MMBNLC-PC-Function-Offsets` `v1.0.3/EXE3_PC.txt`, 20,543 total functions) confirms the exact battle module boundaries:
`em00atk` (272 fns, MegaMan chip attacks), `custom` (185 fns), `subbug` (156 fns, NaviCust bugs/effects), `subnavi` (115 fns, NaviCust), `cockpit` (88 fns, battle HUD), `obj` (84 fns), `btl` (70 fns), `btlinit` (60 fns), `btlnml` (58 fns), `emsub` (53 fns), `em00` + `em00sub0` (79 fns, battle MegaMan), `result` (37 fns), `btlsub` (34 fns).

### 3.1 Object Pools & Dispatch Tables (`A6BE`)
- **Pools** (from [exe4-against-exe3.md](exe4-against-exe3.md) §3.1):
  - Pool 1 (Actors): `0x02037270`, **8 slots** × `0xD4` bytes → Kind update table **`0x0800369C`** (104 entries; `[0] = 0x080B02C9`, Battle MegaMan)
  - Pool 3 (Attacks): `0x020389A0`, **32 slots** × `0xD4` bytes → Kind update table **`0x08003980`** (132 entries)
  - Pool 4 (Effects): `0x02034FC0`, **32 slots** × `0xC4` bytes → Kind update table **`0x08003EBC`** (71 entries)
- **MegaMan Chip Action Table (`em00atk`)**: **`0x080B0DA4`** (62 entries, matching chip record `fam` `+0x07` range `0..=61`).
- **NaviCust Effect / Bug Tables**: **`0x0803C3E8`** (56 entries) and **`0x0803C7FC`** (50 entries).

### 3.2 Chip Record (`0x20` bytes @ `0x08011510`)
- `+0x00..+0x05`: 6 codes (`0..25` = A..Z, `26` = `*`, `0xFF` = none)
- `+0x06`: element (`0` Null [258], `1` Elec [23], `2` Heat [35], `3` Aqua [24], `4` Wood [23], `0xFF` special [22])
- `+0x07` / `+0x08`: `family` (`0..=61`) / `subfamily` (360 unique pairs)
- `+0x09`: rarity, `+0x0A`: MB
- `+0x0C` (`u16`): attack power, `+0x0E` (`u16`): library number
- `+0x13`: flags (Standard / Mega `0x01` / Giga `0x02` / Dimming `0x80` / etc.)
- `+0x14` / `+0x18` / `+0x1C` (`u32`): ROM pointers to icon, chip picture, chip palette
- Legal PvP chip IDs: White `1..=301, 304..=308, 312`; Blue `1..=303, 309..=312`.

### 3.3 Collision, Damage & Hit Registration (IWRAM `0x03007020`, Setup `0x080ACA38`)
Disassembled directly from `A6BE` (`0x08235F00 + 0x1220`):
- **No damage word**: `0x080ACA5C` copies object `+0x2C` (`u16`) directly into collision `+0x14`. No bit-15 doubling, no bit-14 status encoding.
- **Per-element damage buckets**: `0x03007084` accumulates attacker `+0x14` raw into `[target_col + 0x1A + 2 * attacker_elem]`. Elemental weakness and panel scaling happen later during hit intake, not during registration.
- **Body / Invincibility checks**: `0x030070C8` (bits `0x03` vs `0x01`), `0x030070EA` (bit `0x20` vs `0x28`), `0x0300710C` (bit `0x40` vs `0x80`). Side/alignment mask `0xFE000000` at `0x0300704E`. Element `5` bypasses guard bit `0x04` check (`0x03007060`).
- **Hit modifier is a max-index (`0..=5`), NOT a bitfield**: `0x0300708E` computes `target[+0x0B] = max(target[+0x0B], attacker[+0x0A])`. Intake looks up reaction flags in `0x080AF088` / `0x080AF3B0` (no direction push rows in hit modifier).
- **Dual status bytes**: `0x03007098` copies **both** `attacker[+0x0C]` → `target[+0x0E]` and `attacker[+0x0D]` → `target[+0x0F]` when `attacker[+0x0C] != 0`.
- **Reaction nibble**: `0x030070A4` compares `attacker[+0x04]` against `target[+0x11]` (high nibble inequality or low nibble `>`).

### 3.4 Battle Flow, Stages & Style Change
- **Battle Mode State Machine** (`0x080091FC`, 11 states indexed by `[r5 + 1]` in steps of 4): `0x00`..`0x28` (`0x0C` = `0x0800942F` round result → `0x10` win, `0x14` loss, `0x18` draw/timeout).
- **NetBattle Stages** (`0x0803EA98` → weights `0x0803EABC` = 8×`32`, stage IDs `0x0803EAB0` = `[0, 4, 5, 6, 23, 16, 2, 10]`, drawn by SIO master via `GetRNG2`).
- **Style Change** (fixed pre-battle, packed as `(style_type << 3) | element`):
  - `NaviStatsInit` (`0x0804735E`, `0x40`-byte stat block at `0x02037920 + 0x40*side`):
    - `Guts` (`1`): `+0x26 = 2` (buster multiplier), `+0x08 = 1` (SuperArmor / Guts)
    - `Custom` (`2`): `+0x13 = 6` (hand size +1)
    - `Team` (`3`): `+0x14 = 6` (Mega chip limit +1)
    - `Shadow` (`6`): `+0x0D = 0` (vs `1`)
    - Default Giga limit `+0x15 = 1`; base HP memory `+0x2C`.
  - Charge shot & element helper (`0x080B5544` / `0x080B5570`, table `0x080B5980`): sets MegaMan's charge shot chip ID (`[r5 + 0x66]`) and charge shot power by style/element/charge level.
  - **FirstBarrier collision note**: `BattleMegaManInit` (`0x080B048C`) sets barrier ID `8`, HP `100` — which collides with `saito-battle`'s currently hardcoded BubbleWrap ID `8`.

---

## 4. `saito` Engine, Extraction & UI Gap Inventory

### 4.1 Extraction & ROM Discovery (`crates/saito-extract`, `crates/m4a`, `crates/saito`)
1. **Vol. 1 + Vol. 2 multi-volume support** *(implemented first in this branch for `saito-extract`)*:
   - `source.rs`: Discover both Steam AppID `1798010` (Vol. 1) and `1798020` (Vol. 2), merge archives into `Install`, locate `vol1.dat` and `vol2.dat` across merged volumes.
   - `hd_art.rs` & `lib.rs`: Support `VOL1_FILE = "vol1.dat"`, Vol. 1 `base_hash` (`exe1`/`exe2`/`exe3`), `compact_vol1`, `is_valid_vol1`, and volume-aware `attach_hd_chip_art_with_fallback`.
   - Follow-up in `crates/saito`: Settings/UI (`settings.rs`, `screens/settings.rs`, `games.rs`, `settings.slint`) to allow configuring both Vol. 1 and Vol. 2 folders manually when not installed via Steam, and importing `vol1.dat` on mobile/web.
2. **M4A & Sprite verification**:
   - `m4a::rom::extract` already matches EXE3's `m4aSongNumStart` signature (`0x081335EC` in A6BE). Need full bank extraction check on all 331 songs for any unsupported track commands.
   - Verify `gfx.rs` (BG animation commands) and `prims.rs` (`font.dialogue` glyph dimensions) against EXE3 ROM data when writing `content/exe3/extract/`.

### 4.2 `saito-battle` Combat Core Gaps (EXE1–3 Lineage)
1. **Optional Damage Word** (`collision.rs` L419, L451, L797–820; `battle.rs` L2477–2479):
   - Currently unconditionally decodes bit `0x8000` as ×2 damage and masks by `damage_word.damage`. Add a `DamageWord::Plain` (or optional `double_bit`) rule so EXE3 passes raw `u16` damage unchanged.
2. **Hit Modifier Mode** (`collision.rs` L678–681; `kinds/player/intake.rs` L555–575; `reactions.rs` L581–615):
   - Currently merges hit modifiers by bitwise OR and decodes bit 0/1 + `push_reading` bit-scan.
   - Add `HitModifierMode::MaxIndexTable` (merging with `max()` and decoding through a rule-provided flag table, with optional/disabled `push_reading`).
3. **Dual Status Slots** (`collision.rs` L155–156, L465, L631–633; `intake.rs` L150, L603–607):
   - Extend `CollisionData` from a single `status_base`/`status_final` to a pair (`[Option<StatusHandle>; 2]`), propagating and applying both bytes as EXE3 does (`0x03007098`).
4. **Data-Driven Barriers & Auras** (`kinds/player/intake.rs` L156–306):
   - Replace hardcoded barrier IDs (`4`, `8`, `0xA`, `0x10`) with rule-defined barrier specs (regrow, timer, wind/elec pop, threshold vs HP). Fixes the ID `8` collision between EXE3's FirstBarrier (100 HP) and EXE4–6's BubbleWrap.
5. **No Emotion Window / No Counter Hit / Configurable Final Turn** (`content/rules.rs` L1387–1418; `battle.rs` L1220, L1959–1994; `kinds/player/status.rs` L336–356):
   - Make `EmotionRules` optional (`Option<EmotionRules>`), gate Counter Hit bookkeeping/paralysis and mood updates, and make the 15-turn `late_turns()` / Damage Judge configurable in `FlowRules`.
6. **Custom Screen ADD Mode** (`custom/screen.rs` L901–1788, L1835–1952):
   - Add EXE3's **ADD** button flow: enter free-pick mode (ignoring `Together::goes_with` code/name compatibility), select up to 5 chips to discard from the folder, close the custom screen with an empty turn hand, and permanently expand custom capacity (`+N`, capped at 10) for the rest of the round.
7. **Intake-Time Elemental Scaling Verification** (`collision.rs` L687–719):
   - Audit whether EXE3's per-element damage buckets vs barrier/aura absorption order requires delaying weakness/panel multipliers from `resolve_hit` to `intake`.

### 4.3 Match, Render & App UI Gaps
- `crates/saito-match/src/capabilities.rs`: Add `EXE3` capabilities (`flat_standard_copy_limit: true` [4 copies per chip], no patch cards / ABD / CS prob fix / random battle / 5DS) and update `no_game_hardcoding.rs`.
- `crates/saito/src/screens/play/stage.rs`: Add EXE3 boss/postgame stages to `NON_REGULAR_BACKGROUNDS` and match-point BGM arms.
- `crates/saito/src/builds/grid.rs` & NaviCust UI: Support EXE3's **Style-dependent allowed NaviCust colors** (White/Pink/Yellow + Style color) and pre-battle Style Type × Element selection (plus optional ModTools/Error Codes).
- `crates/saito-render/src/objects.rs` L554: Already keeps aura/barrier HP numbers visible for `"exe3"`!

---

## 5. Phased Implementation Plan

1. **Phase 1 — Vol. 1 ROM & Asset Pipeline** *(started)*:
   - `saito-extract` Vol. 1 (`1798010`) + `vol1.dat` HD art support + real-ROM M4A/sprite smoke verification.
   - `content/exe3/extract/` recipe (sprites, chip icons/art, panels, 8 NetBattle stages, HUD, custom screen, fonts, M4A bank) + `crates/exe3-compat` (chip/NaviCust/style IDs & save import).
2. **Phase 2 — `saito-battle` Generation-1 Abstractions**:
   - Plain damage mode, `HitModifierMode::MaxIndexTable`, dual status pair, data-driven barrier table, optional Emotion/Counter/FinalTurn rules, Custom Screen ADD mode.
   - All changes gated behind `Rules` defaults so EXE4/5/6 golden tests and digests stay 100% unchanged.
3. **Phase 3 — `content/exe3` Vertical Slice (Playable MVP)**:
   - MegaMan (Normal + Styles), Buster & Style charge shots, core panels/stages, Custom Screen + ADD + Folder/PA rules, NaviCust core programs, and an initial batch of staple PvP chips (Cannon, Sword, AreaGrab, Invis, Barrier/Aura, Recovery, Bombs, etc.).
   - Wire `capabilities::EXE3`, `stage.rs`, and dual-volume Settings UI.
4. **Phase 4 — Full Chip Library, NaviCust, Styles & Frame-Exact Oracle Verification**:
   - Port remaining Standard / Mega / Giga chips, Program Advances, Style-specific mechanics (Shield/Reflect, Ground crack, Shadow AntiDmg, Bug Style bugs), and validate against real EXE3 mGBA traces.
