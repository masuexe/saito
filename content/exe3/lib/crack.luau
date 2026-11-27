--!strict
-- EXE3's PanlOut1 and PanlOut3 over @exelib/crack.

local collision = require("../rules/collision")
local common = require("@exelib/crack")
local dust_sprite = asset.sprite("dust")

local crack = {}

local CRACK = common.make {
    id = "crack",
    anim = 16,
    sprite = dust_sprite,
    sound = asset.sound("crack-start"),
    break_sound = asset.sound("crack-shot"),
    is = collision.thrown,
    hits = collision.hits_navis,
    on_field_only = false,
}

crack.ahead = { { 0, 0 } }
crack.column = { { 0, 0 }, { 0, -1 }, { 0, 1 } }

export type CrackSpec = {
    id: string,
    panels: { { number } },
}

function crack.action(spec: CrackSpec): Action
    return common.action(CRACK, spec.panels, spec.id)
end

return crack
