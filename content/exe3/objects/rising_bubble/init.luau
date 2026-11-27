--!strict
-- EXE3's rising bubble effect (plus chips and Geddon poison).

local common = require("@exelib/rising_bubble")
local small_puff_sprite = asset.sprite("small-puff")

local rising_bubble = {}

rising_bubble.kind = common.make_kind("rising-bubble", small_puff_sprite)

function rising_bubble.spawn(pos: Vec3, palette: number): Object?
    return common.spawn(rising_bubble.kind, pos, palette)
end

return rising_bubble
