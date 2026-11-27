--!strict
-- EXE3's battle sprites and dialogue portraits from `SpritePointersList`:
--   A6BE: [0x08002704] = 0x0802477C | A3XE: [0x08002704] = 0x08024764
--   A6BJ: [0x080026CC] = 0x080248DC | A3XJ: [0x080026CC] = 0x080248C4
-- Nine categories (byte offsets 0x00..=0x20):
--   - Category 0 (0x00): overworld (1 slot)
--   - Categories 1..=7 (0x04..=0x1C): battle sprites (Battle MegaMan is "04-00")
--   - Category 8 (0x20): dialogue portraits (MegaMan "20-00", Lan "20-01")

local place = require("@exelib/extract/place")
local lists = require("@exelib/extract/sprites")

local sprites = {}

local SPRITE_LIST = place.pointer(0x0800_2704, 0x0800_26CC)
local CATEGORIES = 9
local FIRST_BATTLE_CATEGORY = 2 -- 1-based index into categories[] -> byte offset 0x04
local LAST_BATTLE_CATEGORY = 8  -- byte offset 0x1C
local PORTRAITS = 0x20

-- Battle sprites across categories 0x04..=0x1C.
local function battle(rom: Rom): { Sheet }
    local categories = lists.categories(rom, place.at(rom, SPRITE_LIST), CATEGORIES)
    local out: { Sheet } = {}
    for k = FIRST_BATTLE_CATEGORY, LAST_BATTLE_CATEGORY do
        for index = 0, lists.slots(categories, k) - 1 do
            local sheet = lists.sheet(rom, rom:u32(categories[k] + 4 * index), (k - 1) * 4, index)
            if sheet ~= nil then
                table.insert(out, sheet)
            end
        end
    end
    return out
end

-- Named portraits in category 0x20.
local function portraits(rom: Rom, names: AssetNames): { Sheet }
    local table_at = rom:u32(place.at(rom, SPRITE_LIST) + PORTRAITS)
    local indexes: { number } = {}
    for index in ((names.sprites or {})[PORTRAITS] or {}) do
        table.insert(indexes, index)
    end
    table.sort(indexes)
    local out: { Sheet } = {}
    for _, index in indexes do
        local archive = sprite.archive(rom, rom:u32(table_at + 4 * index))
        local sheet = if archive ~= nil then sprite.portrait(archive, PORTRAITS, index) else nil
        if sheet ~= nil then
            table.insert(out, sheet)
        end
    end
    return out
end

function sprites.sprites(rom: Rom, names: AssetNames): { Sheet }
    local out = battle(rom)
    for _, sheet in portraits(rom, names) do
        table.insert(out, sheet)
    end
    return out
end

return sprites
