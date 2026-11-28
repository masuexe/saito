--!strict
-- MegaMan's base (Normal Style) form in EXE3.

local buster = require("../weapons/buster")
local charged_shot = require("../weapons/charged_shot")
local megaman_sprite = asset.sprite("megaman")

local form: FormDef = {
    base = true,
    sprite = megaman_sprite,
    element = "null",
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = charged_shot.weapon },
    palette = 0,
    idle_anim = 0,
}

return { base = form }
