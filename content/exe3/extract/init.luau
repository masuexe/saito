--!strict
-- EXE3's extraction recipe (docs/design/exe3-extract-map.md): how its pack's assets are extracted from a complete
-- Legacy Collection Vol. 1's four ROMs (its `roms`: White US `A6BE`, Blue US `A3XE`, White Japan `A6BJ`, Black Japan
-- `A3XJ`). It reads the ROMs through the primitives (content/saito/extract.d.luau) and returns the asset model.
--
-- What the four ROMs have alike (battle sprites, battle backgrounds, sound, battle field, most of the HUD and custom
-- screen) is read from White US (`A6BE`, the base version); version-exclusive Giga chips (White 304..=308; Blue
-- 302, 303, 309..=311) are read from their version's US ROM (versions.luau); Japanese words are read from White Japan
-- (`A6BJ`, lettering.luau).

local names = require("@self/names")
local versions = require("@self/versions")
local sprites = require("@self/sprites")
local hud = require("@self/hud")
local custom = require("@self/custom")
local lettering = require("@self/lettering")
local stage = require("@self/stage")

local recipe: Recipe = {
    -- Every EXE3 ROM of Legacy Collection Vol. 1, in slot order (1..=4):
    -- White US (A6BE), Blue US (A3XE), White JP (A6BJ), Black JP (A3XJ).
    roms = { "A6BE", "A3XE", "A6BJ", "A3XJ" },
    extract = function(set: Set): Extracted
        local n = names()
        local chip_versions = versions.chips()
        local rom = set.A6BE
        local battle_sprites = sprites.sprites(rom, n)
        local h = hud.hud(set, n, chip_versions)
        h.languages = { { name = lettering.LANGUAGE, value = lettering.hud(set, h, n) } }
        h.language = "en"
        local c = custom.custom(set, n, chip_versions)
        c.languages = { { name = lettering.LANGUAGE, value = lettering.custom(set, c) } }
        local backgrounds = stage.backgrounds(rom)
        local field = stage.field(rom)
        return {
            names = n,
            graphics = { sprites = battle_sprites, field = field, backgrounds = backgrounds, hud = h, custom = c },
            sound = { from = rom },
        }
    end,
}

return recipe
