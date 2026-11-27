--!strict
-- EXE3's Custom Screen graphics (0x0800EB30..0x0800F490 and 0x08015050).
-- Verified across all four Legacy Collection Vol. 1 ROMs (docs/design/exe3-extract-map.md §2.4).
--
-- Key differences from EXE4-6:
--   1. Chip art and OK / OK_PICKED / ADD pictures are 8x7 tiles (64x56 px, 0x700 bytes = 56 tiles) instead of
--      EXE4-6's 7x6 tiles (0x540 bytes = 42 tiles). See docs/design/exe3-extract-map.md §3 Gap #3.
--   2. Cursor corner words (`[0x0800F3C0]`: [slot, OK, ADD]) pack signed x in bits 0..7, signed y in bits 8..15,
--      and flip flags (0x1000 hflip, 0x2000 vflip) in bits 16..31.
--   3. EXE3 has ADD (instead of UNITE/SHUFFLE) and 4 frame palettes (0x80 bytes at [0x0800EBD0]).

local place = require("@exelib/extract/place")
local colors = require("@exelib/extract/colors")
local hud = require("./hud")
local versions = require("./versions")

local band, rshift = bit32.band, bit32.rshift
local at = place.at
local pointer, pointer_in_each = place.pointer, place.pointer_in_each

local custom = {}

type Block = { place: Place, len: number }
type Details = { gfx: Place, palette: Place }

-- Custom screen DMA list (0x0800EBBC..0x0800EC08 US / 0x0800EB38..0x0800EB84 JP):
local WINDOW_MAP = pointer(0x0800_EBBC, 0x0800_EB38)
local MAP_CELLS = 15 * 20
local WINDOW_TILES: Block = { place = pointer(0x0800_EBC4, 0x0800_EB40), len = 0x5A0 }
local FRAME_PALETTES = pointer(0x0800_EBD0, 0x0800_EB4C) -- 4 x 0x20 bytes
local AUX_TILES: Block = { place = pointer(0x0800_EBDC, 0x0800_EB58), len = 0xB40 }
local ICON_PALETTE = pointer(0x0800_EBE8, 0x0800_EB64)   -- +0x00 icon, +0x20 gray
local CURSOR: Block = { place = pointer(0x0800_EBF4, 0x0800_EB70), len = 0x80 }
local CURSOR_PALETTE = pointer(0x0800_EC00, 0x0800_EB7C)

-- Chip details & slot glyphs:
local CODES = pointer(0x0800_EFD4, 0x0800_EF50)
local CODE_GLYPHS = 28
local EMPTY_ICON = pointer(0x0800_F058, 0x0800_EFD4)
local SLOT_CODES = pointer(0x0800_F294, 0x0800_F210)
local SLOT_CODE_GLYPHS = 28

-- Element icons on BG3 (`0x0800EDB4`: 5 icons in AUX_TILES at 0x99..0xAC, 2x2 = 0x80 bytes each:
-- 0=null, 1=elec, 2=heat/fire, 3=aqua, 4=wood), mapped to rules/elements.luau's 16 families:
-- 0=fire, 1=aqua, 2=elec, 3=wood, 4..15=null.
local FAMILY_TO_ROM_ELEMENT: { number } = { 2, 3, 1, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 }

-- Window emblem over picked column (4 frame maps of 2x3 + 16-byte step sequence):
local EMBLEM_FRAMES = pointer(0x0800_F348, 0x0800_F2C4)
local EMBLEM_STEPS = pointer(0x0800_F34C, 0x0800_F2C8)
local EMBLEM_STEP_COUNT = 16
local EMBLEM_WIDTH, EMBLEM_HEIGHT = 2, 3

-- Cursor corner pointer table ([0] slot, [1] OK, [2] ADD):
local CURSOR_TABLE = pointer(0x0800_F3C0, 0x0800_F33C)

-- Regular chip animated frame (2 frames x 0x180 bytes):
local REGULAR_0: Block = { place = pointer(0x0800_F488, 0x0800_F404), len = 0x180 }
local REGULAR_1: Block = { place = pointer(0x0800_F48C, 0x0800_F408), len = 0x180 }

-- Chip window pictures (8x7 tiles = 56 tiles = 0x700 bytes each!):
local PICTURE_BYTES = 0x700
custom.OK = { gfx = pointer(0x0801_50D8, 0x0801_503C), palette = pointer(0x0801_50DC, 0x0801_5040) } :: Details
custom.OK_PICKED = { gfx = pointer(0x0801_50D0, 0x0801_5034), palette = pointer(0x0801_50D4, 0x0801_5038) } :: Details
custom.ADD_PICTURE = { gfx = pointer(0x0801_50EC, 0x0801_5050), palette = pointer(0x0801_50E8, 0x0801_504C) } :: Details
custom.ADD_BUTTON = "add"

local function signed_byte(n: number): number
    local b = band(n, 0xFF)
    return if b >= 0x80 then b - 0x100 else b
end

-- EXE3 cursor corners: two frames of four words, each word packing x (byte 0), y (byte 1), and flips in hi16.
local function exe3_corners(rom: Rom, a: number): { { { any } } }
    local frames: { { { any } } } = {}
    for f = 0, 1 do
        local four: { { any } } = {}
        for k = 0, 3 do
            local w = rom:u32(a + 16 * f + 4 * k)
            local x = signed_byte(w)
            local y = signed_byte(rshift(w, 8))
            local hi = rshift(w, 16)
            table.insert(four, { y, x, band(hi, 0x1000) ~= 0, band(hi, 0x2000) ~= 0 } :: { any })
        end
        table.insert(frames, four)
    end
    return frames
end

local function palette(rom: Rom, a: number): Palette
    return colors.masked(gba.palette(rom, a))
end

function custom.tiles(rom: Rom, b: Block): Tiles
    return gba.tiles(rom, at(rom, b.place), b.len)
end

function custom.picture(rom: Rom, d: Details): Picture
    return {
        tiles = gba.tiles(rom, at(rom, d.gfx), PICTURE_BYTES),
        palette = palette(rom, at(rom, d.palette)),
    }
end

local function window_emblem(rom: Rom): WindowEmblem
    local frames_ptr = at(rom, EMBLEM_FRAMES)
    local frames: { Map } = {}
    for i = 0, 3 do
        table.insert(frames, gba.map(rom, rom:u32(frames_ptr + 4 * i), EMBLEM_WIDTH * EMBLEM_HEIGHT))
    end
    local steps_ptr = at(rom, EMBLEM_STEPS)
    local steps: { [number]: number, n: number } = { n = EMBLEM_STEP_COUNT }
    for i = 0, EMBLEM_STEP_COUNT - 1 do
        steps[i] = rom:u8(steps_ptr + i)
    end
    return {
        x = 12,
        y = 0,
        width = EMBLEM_WIDTH,
        height = EMBLEM_HEIGHT,
        first_tile = 0x80,
        tiles = custom.tiles(rom, AUX_TILES),
        frames = frames,
        steps = steps,
    }
end

local function chip_art(set: Set, names: AssetNames, chip_versions: { [number]: string }): { ChipArt }
    local keys = names.chips or {}
    local out: { ChipArt } = {}
    for _, id in hud.chip_ids(names) do
        local rom = versions.chip_source(set, chip_versions, id)
        local record = at(rom, hud.CHIPS) + hud.CHIP_STRIDE * id
        local gfx, pal = rom:u32(record + 0x18), rom:u32(record + 0x1C)
        local picture: Picture = if rom:holds(gfx)
            then {
                tiles = gba.tiles(rom, gfx, PICTURE_BYTES),
                palette = if rom:holds(pal) then palette(rom, pal) else nil,
            } :: Picture
            else {} :: Picture
        local entry: ChipArt = {
            key = keys[id] :: string,
            picture = picture,
            version = chip_versions[id] :: string?,
        }
        table.insert(out, entry)
    end
    return out
end

function custom.custom(set: Set, names: AssetNames, chip_versions: { [number]: string }): CustomScreen
    local chip_arts = chip_art(set, names, chip_versions)
    local rom = set.A6BE
    local cursor_tbl = at(rom, CURSOR_TABLE)
    local ok_corners = exe3_corners(rom, rom:u32(cursor_tbl + 4))
    local add_corners = exe3_corners(rom, rom:u32(cursor_tbl + 8))

    local frame_palettes: { Palette } = {}
    for i = 0, 3 do
        table.insert(frame_palettes, palette(rom, at(rom, FRAME_PALETTES) + 0x20 * i))
    end

    local window_tiles = custom.tiles(rom, WINDOW_TILES)
    local aux_addr = at(rom, AUX_TILES.place)
    -- Picked column border cells (0x0800F108: tiles 0xC2..0xC5 in AUX_TILES, 4 tiles = 0x80 B: empty then filled).
    local column_cells = gba.tiles(rom, aux_addr + 0x840, 0x80)

    -- ADD button (0x0800EF78 / 0x0800EF94: 3x2 at col 11, row 17, palette 9):
    -- selectable (0xB6..0xBB, 6 tiles), unavailable/pressed (0xBC..0xC1, 6 tiles), hidden (fill tile 0x80 x 6).
    local add_fill_tile = gba.tiles(rom, aux_addr, 0x20)
    local add_tiles = gba.join({
        gba.tiles(rom, aux_addr + 0x6C0, 0x180),
        add_fill_tile,
        add_fill_tile,
        add_fill_tile,
        add_fill_tile,
        add_fill_tile,
        add_fill_tile,
    })
    local add_picture = custom.picture(rom, custom.ADD_PICTURE)
    local add_button: ButtonPictures = {
        width = 3,
        height = 2,
        tiles = add_tiles,
        sets = "other",
        hidden = 2,
        cursor = { x = 11 * 8, y = 17 * 8, corners = add_corners },
        palettes = { add_picture.palette :: Palette },
        picture = add_picture,
        place = { x = 11, y = 17, first_tile = 0xB6, palette = 9 },
    }

    -- EXE3 custom screen map patches laid over WINDOW_MAP (`0x0800EB96..0x0800EBAE`, `0x0800EDE6`, `0x0800EE0E`,
    -- `0x0800F054`, `0x0800F1EC`): contiguous BG3 tile allocation 0x15C..0x1FF matching GBA EXE3's exact VRAM tile bases.
    local window_patches: PatchList = {
        first_tile = 0x15C,
        patches = {
            -- Element icon (0x0800EDE6): 2x2 at col 3, row 10, tiles 0x15C..0x15F
            { x = 3, y = 10, width = 2, height = 2, palette = 11, by_column = false },
            -- Power digits: 3x2 right-aligned in cols 6..8, rows 10..11 (tiles 0x160..0x165)
            { x = 6, y = 10, width = 3, height = 2, palette = 11, by_column = true },
            -- 10 dealt chip slots (0x0800F230): 2x3 each (2x2 icon + 2x1 code), tiles 0x166..0x1A1
            { x = 1, y = 13, width = 2, height = 3, palette = 11, by_column = false },
            { x = 3, y = 13, width = 2, height = 3, palette = 11, by_column = false },
            { x = 5, y = 13, width = 2, height = 3, palette = 11, by_column = false },
            { x = 7, y = 13, width = 2, height = 3, palette = 11, by_column = false },
            { x = 9, y = 13, width = 2, height = 3, palette = 11, by_column = false },
            { x = 1, y = 16, width = 2, height = 3, palette = 11, by_column = false },
            { x = 3, y = 16, width = 2, height = 3, palette = 11, by_column = false },
            { x = 5, y = 16, width = 2, height = 3, palette = 11, by_column = false },
            { x = 7, y = 16, width = 2, height = 3, palette = 11, by_column = false },
            { x = 9, y = 16, width = 2, height = 3, palette = 11, by_column = false },
            -- 5 picked column chip icons (0x0800F054): 2x2 each at col 12, rows 3..12, tiles 0x1A2..0x1B5
            { x = 12, y = 3, width = 2, height = 2, palette = 11, by_column = false },
            { x = 12, y = 5, width = 2, height = 2, palette = 11, by_column = false },
            { x = 12, y = 7, width = 2, height = 2, palette = 11, by_column = false },
            { x = 12, y = 9, width = 2, height = 2, palette = 11, by_column = false },
            { x = 12, y = 11, width = 2, height = 2, palette = 11, by_column = false },
            -- Chip code (0x0800EBA4): 1x2 at col 1, row 10, tiles 0x1B6..0x1B7
            { x = 1, y = 10, width = 1, height = 2, palette = 11, by_column = true },
            -- Chip name (0x0800EB96): 8x2 at col 1, row 1, tiles 0x1B8..0x1C7
            { x = 1, y = 1, width = 8, height = 2, palette = 9, by_column = true },
            -- Chip art (0x0800EB96): 8x7 (56 tiles) at col 1, row 3, tiles 0x1C8..0x1FF
            { x = 1, y = 3, width = 8, height = 7, palette = 10, by_column = false },
        },
    }

    -- Fill the 4th power-digit spacer column (col 5, rows 10..11) on WINDOW_MAP with tile 0x900C (solid details-bar
    -- charcoal matching col 2) so no battlefield pixels peek between the element icon and the 3 power digits.
    local map_addr = at(rom, WINDOW_MAP)
    local map_hex: { string } = {}
    for cell = 0, MAP_CELLS - 1 do
        local v = if cell == 10 * 15 + 5 or cell == 11 * 15 + 5 then 0x900C else rom:u16(map_addr + 2 * cell)
        table.insert(map_hex, string.format("%02x%02x", band(v, 0xFF), rshift(v, 8)))
    end
    local window_map = gba.map(bytes.hex(table.concat(map_hex)), 0, MAP_CELLS)

    -- 16-family BG3 element icon strip from AUX_TILES + 0x320 (tiles 0x99..0xAC, opaque charcoal background)
    -- plus matching 6-color slices (palette 11 indices 10..15) so saito-render draws all 16 chip families.
    local icon_pal = palette(rom, at(rom, ICON_PALETTE))
    local elem_six: { number } = { icon_pal[11], icon_pal[12], icon_pal[13], icon_pal[14], icon_pal[15], icon_pal[16] }
    local element_icons: { Tiles } = {}
    local element_colors: { { number } } = {}
    for _, rom_el in FAMILY_TO_ROM_ELEMENT do
        table.insert(element_icons, gba.tiles(rom, aux_addr + 0x320 + 0x80 * rom_el, 0x80))
        table.insert(element_colors, elem_six)
    end
    local elements_strip = gba.join(element_icons)

    -- Pad each 4x3 Regular chip frame (0x180 B = 12 tiles) with 4 blank tiles to canonical 4x4 (32x32) layout.
    local blank4 = gba.tiles(bytes.hex(string.rep("00", 0x80)), 0, 0x80)

    return {
        layout = {
            column_cells = 0xC2,
            turn_limit = 0,
            turn_limit_palette = 9,
            name = 0x1B8,
            art = 0x1C8,
            code = 0x1B6,
            element = 0x15C,
            digits = 0x160,
            slots = 0x166,
            column_icons = 0x1A2,
            name_bar = 0,
            form_names = 0,
            slot_blank = 1,
            detail_blank = 7,
            empty_palette = 9,
            ok_cursor = { x = 0x58, y = 0x70, corners = ok_corners },
            regular = { 1, 100 },
            column_columns = { 11, 14 },
            column_row = 3,
            form_names_at = { 1, 1 },
        },
        -- The window's label: CHIP SELECT over the slots (tiles 0x0F to 0x15, color 4 on 1).
        labels = { { name = "chip-select", first = 0x0F, last = 0x15, top = 3, bottom = 6, letters = { 4 } } },
        buttons = { { name = custom.ADD_BUTTON, value = add_button } },
        window_tiles = window_tiles,
        window_maps = { window_map },
        window_patches = window_patches,
        column_cells = column_cells,
        frame_palettes = frame_palettes,
        icon_palette = palette(rom, at(rom, ICON_PALETTE)),
        gray_palette = palette(rom, at(rom, ICON_PALETTE) + 0x20),
        chip_art = chip_arts,
        pictures = {
            ok = custom.picture(rom, custom.OK),
            ok_picked = custom.picture(rom, custom.OK_PICKED),
        },
        codes = gba.tiles(rom, at(rom, CODES), 0x40 * CODE_GLYPHS),
        elements = elements_strip,
        element_colors = element_colors,
        -- Custom screen chip-power digits (0..9 at AUX_TILES + 0x20, '?' at AUX_TILES + 0x2E0; 0x0800EE58 tile 0x81).
        digits = gba.join({
            gba.tiles(rom, aux_addr + 0x20, 0x40 * 10),
            gba.tiles(rom, aux_addr + 0x2E0, 0x40),
        }),
        slot_codes = gba.tiles(rom, at(rom, SLOT_CODES), 0x40 * SLOT_CODE_GLYPHS),
        empty_icon = gba.tiles(rom, at(rom, EMPTY_ICON), 0x80),
        versioned = { base_version = versions.WHITE, base = {} },
        cursor = custom.tiles(rom, CURSOR),
        cursor_palette = palette(rom, at(rom, CURSOR_PALETTE)),
        regular = gba.join({ custom.tiles(rom, REGULAR_0), blank4, custom.tiles(rom, REGULAR_1), blank4 }),
        window_emblem = window_emblem(rom),
    }
end

return custom
