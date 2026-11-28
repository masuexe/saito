--!strict
-- EXE3's Battle HUD graphics: 8x16 HUD font, 8x16 tiled dialogue font, HUD tiles/palettes, Custom Gauge tiles/frame,
-- HP box, enemy HP palette & digits, chip icons (0x20-byte chip table), 13 battle banners (0xFF-terminated glyph-index
-- arrays), PAUSE tiles, HUD text lines (terminator 0xE7), and chatbox tiles/palette/arrow.

local place = require("@exelib/extract/place")
local colors = require("@exelib/extract/colors")
local texts = require("@exelib/extract/texts")
local glyphs = require("@exelib/extract/glyphs")
local versions = require("./versions")

local at = place.at
local pointer, pointer_in_each, stated = place.pointer, place.pointer_in_each, place.stated

local hud = {}

-- ---- Fonts and text lines --------------------------------------------------------------------------------------

local FONT = pointer(0x0801_0FB0, 0x0801_0F14)
local FONT_GLYPHS = 0xE4
-- Documented for Gap #2 (`font.dialogue` 8x16 tiled support):
hud.DIALOGUE_FONT = pointer_in_each({ 0x0802_6A14, 0x0802_69FC, 0x0802_6B74, 0x0802_6B5C })

local TEXTS = pointer(0x0801_0AA0, 0x0801_0A04)
local TEXT_END = 0xE7

function hud.palette(rom: Rom, a: number): Palette
    return colors.masked(gba.palette(rom, a))
end

function hud.font(rom: Rom, cell: { string }, dialogue: { string }): (Tiles, { string })
    local n = math.min(#cell + #dialogue, FONT_GLYPHS)
    return gba.tiles(rom, at(rom, FONT), 0x40 * n), glyphs.chars(cell, dialogue, n)
end

function hud.dialogue_font(rom: Rom, cell: { string }, dialogue: { string }): DialogueFont
    local n = math.min(#cell + #dialogue, FONT_GLYPHS)
    return font.dialogue_tiled(rom, at(rom, hud.DIALOGUE_FONT), n, 8, glyphs.chars(cell, dialogue, n))
end

function hud.texts(rom: Rom): { { number } }
    local first_control: number = data("compat/text.toml").first_control
    return texts.lines(rom, at(rom, TEXTS), TEXT_END, first_control)
end

-- ---- HUD DMA blocks, HP box, Custom Gauge, Banners, Chatbox ---------------------------------------------------

-- Battle init DMA list (0x08005FEC US / 0x08005F80 JP):
local HUD_OBJ_TILES = pointer(0x0800_5FF8, 0x0800_5F8C) -- 0x4C0 bytes -> 0x06017000 (enemy HP digits etc.)
local ENEMY_PALETTE = pointer(0x0800_6010, 0x0800_5FA4) -- 0x20 bytes  -> 0x020090E0
local GAUGE_TILES = pointer(0x0800_601C, 0x0800_5FB0)   -- 0x380 B gauge + 0x340 B damage digits -> 0x06008600 (first_tile 0x30)
local GAUGE_BYTES = 0x380
local GAUGE_FIRST_TILE = 0x30
-- EXE3 places full-gauge shimmer frame 0 at tile 9, frame tiles (CUSTOM/caps/top) at 10..16, "L or R" at 17..24,
-- and shimmer frames 1..3 at 25..27. Remap into canonical `Hud` order (0..8 fill, 9..15 frame, 16..19 full, 20..27 L or R).
local GAUGE_TILE_ORDER = {
    0, 1, 2, 3, 4, 5, 6, 7, 8,
    10, 11, 12, 13, 14, 15, 16,
    9, 25, 26, 27,
    17, 18, 19, 20, 21, 22, 23, 24,
}
local HUD_TILES = pointer(0x0800_6028, 0x0800_5FBC)     -- 0x300 bytes -> 0x0600A000 (first_tile 0x100)
local HUD_BYTES = 0x300
local HUD_FIRST_TILE = 0x100

local HP_PALETTES = pointer(0x0801_0DE0, 0x0801_0D44) -- 3 x 0x20 bytes (normal, heal, low)
local HP_BOX = pointer(0x0801_0DE8, 0x0801_0D4C)      -- 6x2 map entries
local GAUGE_FRAME = stated({ 0x0801_0BE0, 0x0801_0BE0, 0x0801_0B44, 0x0801_0B44 }) -- 18x2 map entries

-- Battle HUD OBJ chip icon palette (DMA[4] -> 0x020090E0, index 9 = #000000 black border; distinct from the
-- Custom Screen BG icon palette at 0x0800_EBE8/0x0800_EB64 whose index 9 is #7b9cde blue).
local ICON_PALETTE = pointer(0x0800_6010, 0x0800_5FA4)
local PAUSE = pointer(0x0801_0A14, 0x0801_0978)
local PAUSE_BYTES = 0x280

-- 13 battle banners: 0xFF-terminated glyph index strings indexing u16 byte offsets into banner font (0x40 B/glyph).
local BANNERS = pointer(0x0801_0810, 0x0801_0780)
local BANNER_OFFSETS = pointer(0x0801_0814, 0x0801_0784)
local BANNER_FONT = pointer(0x0801_0818, 0x0801_0788)
local BANNER_DIGITS = pointer(0x0801_0794, 0x0801_0704)
local BANNER_PALETTE = pointer(0x0801_0788, 0x0801_06F8)
local BANNER_COUNT = 13

-- Dimming-chip telops (`telops = "apart"`): 15 glyphs centered from x=0 (local) / x=120 (remote) at y=32,
-- using the HUD's battle damage digits (0..9, '+'), font '×'/'2', and the telop OBJ palette
-- (`0x086A5288` US White / `0x086A61A0` JP White, white/blue letters with dark outline and yellow digits).
local TELOP_PLACES = { { 0, 32 }, { 120, 32 } }
local TELOP_PALETTE = pointer(0x0801_0A20, 0x0801_0984)

-- Chatbox tiles (9 tiles = 0x120 B, first tile 0x2B8), BG15 box/text palette (0x08681A5C: dark charcoal border/text,
-- white interior, silver bevel), OBJ11 arrow palette (0x0868433C: green bounce arrow), 32x8 BG0 map (cols 0..29 visible),
-- and 3-frame 16x16 bounce arrow.
local CHATBOX_TILES = pointer_in_each({ 0x0802_67B8, 0x0802_67A0, 0x0802_6918, 0x0802_6900 })
local CHATBOX_BG_PALETTE = pointer(0x0800_1D38, 0x0800_1D00)
local CHATBOX_ARROW_PALETTE = pointer_in_each({ 0x0802_67C4, 0x0802_67AC, 0x0802_6924, 0x0802_690C })
local CHATBOX_MAP = pointer_in_each({ 0x0802_6B54, 0x0802_6B3C, 0x0802_6CB4, 0x0802_6C9C })
local CHATBOX_FIRST_TILE = 0x2B8
local CHATBOX_ARROW = pointer_in_each({ 0x0802_6C58, 0x0802_6C40, 0x0802_6DB8, 0x0802_6DA0 })
local CHATBOX_TEXT = { 0x33, 0x6C }
local CHATBOX_ARROW_AT = { 0xE0, 0x8D }

-- Chip table (385 x 0x20 bytes; +0x14 icon, +0x18 picture, +0x1C picture palette).
hud.CHIPS = stated({ 0x0801_1510, 0x0801_1510, 0x0801_1474, 0x0801_1474 })
hud.CHIP_STRIDE = 0x20
local ICON_BYTES = 0x80

function hud.chip_ids(names: AssetNames): { number }
    local ids: { number } = {}
    for id in (names.chips or {}) do
        table.insert(ids, id)
    end
    table.sort(ids)
    return ids
end

-- Decode EXE3 banner `id` (0..=12) from its 0xFF-terminated glyph-index string.
function hud.banner(rom: Rom, id: number): BannerLayout
    local str_ptr = rom:u32(at(rom, BANNERS) + 4 * id)
    local offs = at(rom, BANNER_OFFSETS)
    local font_base = at(rom, BANNER_FONT)
    local parts: { Tiles } = {}
    local k = 0
    while k < 32 do
        local idx = rom:u8(str_ptr + k)
        if idx == 0xFF then
            break
        end
        local byte_off = rom:u16(offs + 2 * idx)
        table.insert(parts, gba.tiles(rom, font_base + byte_off, 0x40))
        k += 1
    end
    -- Center banner horizontally on 240px GBA screen (8px per glyph).
    local x = math.max(0, (240 - k * 8) // 2)
    return { x = x, y = 64, kind = 0, glyphs = gba.join(parts) }
end

function hud.pause(rom: Rom): Tiles
    return gba.tiles(rom, at(rom, PAUSE), PAUSE_BYTES)
end

local function chip_icons(
    set: Set,
    names: AssetNames,
    chip_versions: { [number]: string }
): { { key: string, tiles: Tiles? } }
    local keys = names.chips or {}
    local out: { { key: string, tiles: Tiles? } } = {}
    for _, id in hud.chip_ids(names) do
        local rom = versions.chip_source(set, chip_versions, id)
        local p = rom:u32(at(rom, hud.CHIPS) + hud.CHIP_STRIDE * id + 0x14)
        local icon: Tiles? = if rom:holds(p) then gba.tiles(rom, p, ICON_BYTES) else nil
        local entry: { key: string, tiles: Tiles? } = { key = keys[id] :: string, tiles = icon }
        table.insert(out, entry)
    end
    return out
end

local function remap_gauge_frame(rom: Rom, addr: number): Map
    local hex_parts: { string } = {}
    for i = 0, 35 do
        local entry = rom:u16(addr + 2 * i)
        local tile = bit32.band(entry, 0x3FF)
        if tile >= 0x3A and tile <= 0x40 then
            entry -= 1
        end
        table.insert(hex_parts, string.format("%02x%02x", bit32.band(entry, 0xFF), bit32.rshift(entry, 8)))
    end
    return gba.map(bytes.hex(table.concat(hex_parts)), 0, 36)
end

-- Read EXE3's 32x8 BG0 chatbox tilemap into the canonical 30x8 screen map (cols 0..29, tile indices relative to 0x2B8).
local function chatbox_map(rom: Rom, addr: number): Map
    local hex_parts: { string } = {}
    for row = 0, 7 do
        for col = 0, 29 do
            local entry = rom:u16(addr + 2 * (row * 32 + col))
            local tile = bit32.band(entry, 0x3FF)
            local flags = bit32.band(entry, 0xFC00)
            local rel = if tile >= CHATBOX_FIRST_TILE then tile - CHATBOX_FIRST_TILE else 0
            local out = bit32.bor(flags, rel)
            table.insert(hex_parts, string.format("%02x%02x", bit32.band(out, 0xFF), bit32.rshift(out, 8)))
        end
    end
    return gba.map(bytes.hex(table.concat(hex_parts)), 0, 30 * 8)
end

-- Combine EXE3's BG15 dark-charcoal text ink/shadow (indices 1..2) with OBJ11's green bounce arrow colors (indices 6..11).
local function chatbox_text_palette(bg_pal: Palette, arrow_pal: Palette): Palette
    local out: { number } = {}
    for i = 1, 16 do
        out[i] = arrow_pal[i]
    end
    out[2] = bg_pal[2] -- index 1: #292929 dark charcoal ink
    out[3] = bg_pal[4] -- index 2: #cdcdcd light silver drop shadow
    return out
end

local function drawn(rom: Rom, h: Hud)
    -- EXE3's HUD DMA[5] (`HUD_TILES`, 0x300 B) holds the 12 HP-box glyphs (0..9, blank, edge); its battle damage
    -- digits (0..9, '+', '?', ':') sit right after the Custom Gauge tiles at `GAUGE_TILES + 0x380` (0x08010F12 uses
    -- tile 0x4C = 0x30 + 28). Assemble them in the canonical `Hud` glyph order (HP 0..9, blank, edge, damage 0..9,
    -- '?', '+', ':', '×', '2') so `Hud::DAMAGE_DIGIT`, `HIDDEN_GLYPH`, `PLUS_GLYPH`, and `TIMES_GLYPH` line up.
    local gauge_at = at(rom, GAUGE_TILES)
    local dmg_at = gauge_at + 0x380
    local font_at = at(rom, FONT)
    h.tiles = gba.join({
        gba.tiles(rom, at(rom, HUD_TILES), HUD_BYTES),
        gba.tiles(rom, dmg_at, 0x40 * 10), -- 0..9 (DAMAGE_DIGIT = 12)
        gba.tiles(rom, dmg_at + 0x40 * 11, 0x40), -- '?' (HIDDEN_GLYPH = 22)
        gba.tiles(rom, dmg_at + 0x40 * 10, 0x40), -- '+' (PLUS_GLYPH = 23)
        gba.tiles(rom, dmg_at + 0x40 * 12, 0x40), -- ':' (24)
        gba.tiles(rom, font_at + 0x40 * 0x40, 0x40), -- '×' (TIMES_GLYPH = 25)
        gba.tiles(rom, font_at + 0x40 * 0x03, 0x40), -- '2' (26)
    })
    h.first_tile = HUD_FIRST_TILE
    h.gauge_tiles = gba.tiles(rom, gauge_at, GAUGE_BYTES):pick(GAUGE_TILE_ORDER)
    h.gauge_first_tile = GAUGE_FIRST_TILE
    local hp_pals: { Palette } = {}
    for i = 0, 2 do
        table.insert(hp_pals, hud.palette(rom, at(rom, HP_PALETTES) + 0x20 * i))
    end
    h.hp_palettes = hp_pals
    h.gauge_palette = hp_pals[1]
    h.hp_box = gba.map(rom, at(rom, HP_BOX), 12)
    h.gauge_frame = remap_gauge_frame(rom, at(rom, GAUGE_FRAME))
    -- Enemy HP digits live at the start of the HUD OBJ DMA block (10 x 0x40 B = 0x280 B).
    local enemy_digits = gba.tiles(rom, at(rom, HUD_OBJ_TILES), 0x40 * 10)
    h.enemy_digits = { enemy_digits, enemy_digits, enemy_digits }
    h.enemy_palette = hud.palette(rom, at(rom, ENEMY_PALETTE))
    h.icon_palette = hud.palette(rom, at(rom, ICON_PALETTE))
    h.pause = hud.pause(rom)
    h.banner_digits = gba.tiles(rom, at(rom, BANNER_DIGITS), 0x40 * 11)
    h.banner_palette = hud.palette(rom, at(rom, BANNER_PALETTE))
    local box_pal = hud.palette(rom, at(rom, CHATBOX_BG_PALETTE))
    local arrow_pal = hud.palette(rom, at(rom, CHATBOX_ARROW_PALETTE))
    local full_box = chatbox_map(rom, at(rom, CHATBOX_MAP))
    local box_steps = { full_box, full_box, full_box, full_box }
    h.chatbox = {
        tiles = gba.tiles(rom, at(rom, CHATBOX_TILES), 0x120),
        palette = box_pal,
        boxes = { box_steps, box_steps },
        arrow = gba.tiles(rom, at(rom, CHATBOX_ARROW), 3 * 0x80),
        text_palette = chatbox_text_palette(box_pal, arrow_pal),
    }
end

local function worded(rom: Rom, names: AssetNames, h: Hud)
    local cell, dialogue = names.glyphs or {}, names.dialogue_glyphs or {}
    h.font, h.font_chars = hud.font(rom, cell, dialogue)
    h.dialogue_font = hud.dialogue_font(rom, cell, dialogue)
    h.texts = hud.texts(rom)
    local banners: { BannerLayout } = {}
    for id = 0, BANNER_COUNT - 1 do
        table.insert(banners, hud.banner(rom, id))
    end
    h.banners = banners
end

local function telop(rom: Rom): TelopLook
    local dmg_at = at(rom, GAUGE_TILES) + 0x380
    local font_at = at(rom, FONT)
    return {
        places = TELOP_PLACES,
        glyphs = gba.join({
            gba.tiles(rom, dmg_at, 0x40 * 10), -- 0..9
            gba.tiles(rom, dmg_at + 0x40 * 10, 0x40), -- '+'
            gba.tiles(rom, font_at + 0x40 * 0x40, 0x40), -- '×'
            gba.tiles(rom, font_at + 0x40 * 0x03, 0x40), -- '2'
        }),
        palette = hud.palette(rom, at(rom, TELOP_PALETTE)),
    }
end

function hud.hud(set: Set, names: AssetNames, chip_versions: { [number]: string }): Hud
    local h: Hud = {}
    h.chip_icons = chip_icons(set, names, chip_versions)
    local rom = set.A6BE
    h.layout = {
        chatbox_text = CHATBOX_TEXT,
        chatbox_arrows = { CHATBOX_ARROW_AT, CHATBOX_ARROW_AT },
        telop = telop(rom),
    }
    drawn(rom, h)
    worded(rom, names, h)
    return h
end

return hud
