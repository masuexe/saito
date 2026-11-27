--!strict
-- EXE3 Geddon dimming controller over @exelib/geddon.

local common = require("@exelib/geddon/controller")
local quake = require("@exelib/geddon/quake")
local rising_bubble = require("../../objects/rising_bubble")
local small_puff_sprite = asset.sprite("small-puff")

local quakes = quake.make {
    id = "geddon/quake",
    puff = new.effect { sprite = small_puff_sprite, anim = 1, palette = 0, steps = 16 },
    bubble = rising_bubble.kind,
    sounds = {
        puff = asset.sound("panel-crack"),
        bubble = asset.sound("panel-poison"),
        poison = asset.sound("panel-poison"),
    },
}

local made = common.make { id = "geddon/controller", name = "Geddon", quake = quakes.spawn, hold_panel = false }

return { kind = made.kind, hook = made.hook, quake = quakes }
