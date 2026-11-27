--!strict
-- EXE3's battle field and 30 battle backgrounds (including the 8 NetBattle stages: [0, 4, 5, 6, 23, 16, 2, 10]).
-- Verified across all four Legacy Collection Vol. 1 ROMs (docs/design/exe3-extract-map.md §2.1 & §2.2).
--
-- Key differences from EXE4-6:
--   1. Field has 11 panel types (0..=10; 10 is Sand). LZ77 field tiles decompress to 0x6900 bytes at VRAM 0x06001620
--      (`first_tile = 0xB1`). Panel blocks, highlight block, and front edges store relative tile numbers; EXE3 adds a
--      per-block u16 offset (`PanelBlockOffsets`, `0x0391`, `FrontEdgeOffsets`), applied here via `gba.map`'s wrapping
--      `first_tile` parameter `(0x10000 - add) % 0x10000`.
--   2. Backgrounds live in the Area 0xF0 table `[[BgAreaF0Table]]` (30 x 12-byte entries): 3-layer tiles header at +0,
--      palette (+4 header) at +4, and map archive at +8 whose LZ77 stream starts at `+0x10` (not `+0x0C`).
--   3. Primitive Gap #1 (`gfx.anims`): EXE3 dispatches per-BG animation handlers via Thumb jump table `[0x08006070]`
--      calling `0x08001FA0` with single `GfxAnim` structs, and its `cmd=0` palette anim counts words (`8`) instead of
--      bytes (`32`). `anims` is left empty here until `saito-extract` adds single-anim / word-count support.

local place = require("@exelib/extract/place")
local colors = require("@exelib/extract/colors")

local band = bit32.band
local at = place.at
local pointer, pointer_in_each, stated = place.pointer, place.pointer_in_each, place.stated

local stage = {}

-- ---- Field ---------------------------------------------------------------------------------------------------

local FIELD_TILES = pointer(0x0800_6054, 0x0800_5FE8)
local FIELD_FIRST_TILE = 0x1620 // 32 -- 0xB1 (VRAM 0x06001620)
local FIELD_PALETTES = pointer(0x0800_5FEC, 0x0800_5F80)
local FIELD_PALETTE_BUFFER = 0x0200_9810

local PANEL_BLOCKS = pointer(0x0800_AF58, 0x0800_AED4)
local PANEL_BLOCK_OFFSETS = pointer(0x0800_AF5C, 0x0800_AED8)
local PANEL_TYPES = 11

local HIGHLIGHT_BLOCK = pointer(0x0800_B044, 0x0800_AFC0)
local HIGHLIGHT_TILE_ADD = 0x0391

local FRONT_EDGES = pointer(0x0800_B0A4, 0x0800_B020)
local FRONT_EDGE_OFFSETS = pointer(0x0800_B0A8, 0x0800_B024)

-- EXE3's 4 cycling panel palettes (0x0800B0DC): all run with initial_timer = 14, period = 14 ticks.
local PANEL_PALETTE_ANIMS: { { table: Place, count: number, dest: number } } = {
    { table = pointer(0x0800_B190, 0x0800_B10C), count = 9, dest = 0x0200_9850 }, -- slot 2
    { table = pointer(0x0800_B1B8, 0x0800_B134), count = 9, dest = 0x0200_98D0 }, -- slot 6
    { table = pointer(0x0800_B1E0, 0x0800_B15C), count = 6, dest = 0x0200_9870 }, -- slot 3
    { table = pointer(0x0800_B1FC, 0x0800_B178), count = 6, dest = 0x0200_98F0 }, -- slot 7
}
local PANEL_PALETTE_TICKS = 14

-- Read `count` map entries at `addr`, adding `tile_add` to each tile index (via gba.map's wrapping subtraction).
local function map_with_add(rom: Rom, addr: number, count: number, tile_add: number): Map
    local sub = (0x10000 - tile_add) % 0x10000
    return gba.map(rom, addr, count, sub)
end

-- EXE3's 11 panel type names by number from compat/panels.toml.
function stage.panel_names(): { string }
    local types: { [string]: any } = data("compat/panels.toml").types or {}
    local by_number: { [number]: string } = {}
    for key, entry in types do
        local n = if string.match(key, "^%d+$") then tonumber(key) else nil
        if n == nil or n > 0xFF then
            error(string.format("panels.toml: type %q is no number", key), 0)
        end
        by_number[n] = entry.name
    end
    local out: { string } = {}
    for n = 0, PANEL_TYPES - 1 do
        local name: string? = by_number[n]
        if name == nil then
            error(string.format("EXE3 panel %d: panels.toml has no type %d", n, n), 0)
        end
        table.insert(out, name)
    end
    return out
end

-- Battle field from `rom`.
function stage.field(rom: Rom): Field
    local panel_types = stage.panel_names()
    local raw_tiles = rom:lz77(at(rom, FIELD_TILES))
    if raw_tiles == nil then
        error(string.format("EXE3 field tiles at %#x aren't LZ77", at(rom, FIELD_TILES)), 0)
    end
    local tiles = gba.tiles(raw_tiles, 0, raw_tiles:len())
    local raw_palettes = gba.palettes(rom, at(rom, FIELD_PALETTES), 8)
    local palettes: { Palette } = {}
    for _, p in raw_palettes do
        local masked: Palette = colors.masked(p)
        table.insert(palettes, masked)
    end

    local blocks_base = at(rom, PANEL_BLOCKS)
    local offsets_base = at(rom, PANEL_BLOCK_OFFSETS)
    local panels: { Map } = {}
    for i = 0, PANEL_TYPES * 6 - 1 do
        local add = rom:u16(offsets_base + 2 * i)
        table.insert(panels, map_with_add(rom, blocks_base + 32 * i, 15, add))
    end

    local hl = map_with_add(rom, at(rom, HIGHLIGHT_BLOCK), 15, HIGHLIGHT_TILE_ADD)
    local edges_base = at(rom, FRONT_EDGES)
    local edge_offs = at(rom, FRONT_EDGE_OFFSETS)
    local front_edges: { Map } = {
        map_with_add(rom, edges_base, 5, rom:u16(edge_offs)),
        map_with_add(rom, edges_base + 32, 5, rom:u16(edge_offs + 2)),
    }

    local palette_anims: { PaletteAnim } = {}
    for _, anim in PANEL_PALETTE_ANIMS do
        local tbl = at(rom, anim.table)
        local frames: { { any } } = {}
        for i = 0, anim.count - 1 do
            local pal = colors.masked(gba.palette(rom, rom:u32(tbl + 4 * i)))
            table.insert(frames, { pal, PANEL_PALETTE_TICKS } :: { any })
        end
        table.insert(palette_anims, {
            slot = (anim.dest - FIELD_PALETTE_BUFFER) // 32,
            frames = frames,
            initial_timer = PANEL_PALETTE_TICKS,
        })
    end

    return {
        tiles = tiles,
        first_tile = FIELD_FIRST_TILE,
        palettes = palettes,
        first_palette = 1,
        palette_anims = palette_anims,
        panel_types = panel_types,
        panels = panels,
        front_edges = front_edges,
        highlights = { hl, hl },
    }
end

-- ---- Backgrounds ---------------------------------------------------------------------------------------------

-- Area 0xF0 table pointer literal: `[[BG_AREA_F0]]` -> 30 x 12-byte background records.
local BG_AREA_F0 = pointer_in_each({ 0x0802_2FA8, 0x0802_2F90, 0x0802_3108, 0x0802_30F0 })
local BACKGROUND_COUNT = 30

-- Scroll callback code base (`ScrollCBs`), verified across all 4 ROMs.
stage.SCROLLERS = stated({ 0x0800_1E9C, 0x0800_1E9C, 0x0800_1E64, 0x0800_1E64 })
stage.SCROLLER_STEPS = {
    [0x00] = { 0, 0 },
    [0x04] = { -8, -4 },
    [0x20] = { 0, -4 },
    [0x34] = { 0, 4 },
    [0x48] = { 8, 0 },
    [0x5C] = { -8, 0 },
    [0x70] = { -80, 0 },
    [0x88] = { 0, 0 },
}

-- Decode one EXE3 Area 0xF0 background record at `entry` (+0 tiles hdr, +4 pal, +8 map archive with LZ77 at +0x10).
local function decode_background(rom: Rom, entry: number): Background?
    local gfx = rom:u32(entry)
    local pal_src = rom:u32(entry + 4)
    local map_src = rom:u32(entry + 8)
    if not rom:holds(gfx) or not rom:holds(map_src) then
        return nil
    end
    local size_bytes = rom:u32(gfx) * 4
    local lz_off = rom:u32(gfx + 4)
    local vram_off = rom:u32(gfx + 8)
    local raw_tiles = rom:lz77(gfx + lz_off)
    if raw_tiles == nil then
        return nil
    end
    local tiles = gba.tiles(raw_tiles, 0, math.min(size_bytes, raw_tiles:len()))
    local first_tile = band(vram_off, 0xFFFF) // 32

    local w, h = rom:u8(map_src), rom:u8(map_src + 1)
    local raw_map = rom:lz77(map_src + 0x10)
    if raw_map == nil then
        return nil
    end
    local map = gba.map(raw_map, 0, w * h)
    local palette = if rom:holds(pal_src) then colors.masked(gba.palette(rom, pal_src + 4)) else nil

    return {
        tiles = tiles,
        first_tile = first_tile,
        map = map,
        map_width = w,
        map_height = h,
        palette = palette,
        scroll = { 0, 0 },
        -- See docs/design/exe3-extract-map.md §3 Gap #1 (`gfx.anims` single-anim & word-count cmd=0).
        anims = {},
    }
end

-- All 30 battle backgrounds by ID (0..=29).
function stage.backgrounds(rom: Rom): { [number]: Background, n: number }
    local table_ptr = rom:u32(at(rom, BG_AREA_F0))
    local out: { [number]: Background, n: number } = { n = BACKGROUND_COUNT }
    for id = 0, BACKGROUND_COUNT - 1 do
        local b = decode_background(rom, table_ptr + 12 * id)
        if b ~= nil then
            out[id] = b
        end
    end
    return out
end

return stage
