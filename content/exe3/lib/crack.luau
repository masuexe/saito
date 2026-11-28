--!strict
-- EXE3's PanlOut1 and PanlOut3 over @exelib/crack.

local anims = require("./anims")
local attachment = require("../objects/attachment")
local collision = require("../rules/collision")
local common = require("@exelib/crack")
local panlout_tool = attachment.look { sprite = asset.sprite("panlout-tool"), palette = 0 }
local small_puff_sprite = asset.sprite("small-puff")

local crack = {}

-- Verified against A6BE (`0x080B2308`, `0x080B2316`, `0x080B2394`): MegaMan anim 0x24,
-- held attachment `0c-07` anim 0, and panel-break dust effect 0x15 (`14-02` `small-puff` anim 1).
local CRACK = common.make {
    id = "crack",
    anim = anims.PANLOUT,
    hold = function(me: Object)
        attachment.attach(me, panlout_tool, "related", 0)
    end,
    sprite = small_puff_sprite,
    sprite_anim = 1,
    sound = asset.sound("crack-start"),
    break_sound = asset.sound("panel-crack"),
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
