--!strict
-- EXE5DS Sol Cross MegaMan's charged shot (weapon 0x6C: 0x02006DD0, 0x0203D110[0x6C], 0x0204732C, 0x020473B2):
-- flat 130-tick charge time regardless of Charge level, firing GunDelSol for the full 60 ticks without needing to
-- hold A/B (120 HP total drain in shade, 240 HP total drain in sunlight).

local gundels = require("../../../chips/gundels/gundels")
local regions = require("@exelib/regions")
local sun_beam_sprite = asset.sprite("sun-beam")

local BEAM: SunBeamLook = { sprite = sun_beam_sprite, palette = 0 }
local BEAM_IN_SUN: SunBeamLook = { sprite = sun_beam_sprite, palette = 2 }

local sol_gundelsol = {}

sol_gundelsol.action = gundels.action {
    id = "megaman/sol-gundelsol/action",
    gun = gundels.gun(0),
    firing_ticks = 60,
    beam = BEAM,
    beam_in_sun = BEAM_IN_SUN,
    region = regions.wide,
    no_hold = true,
}

function sol_gundelsol.setup(navi: Object): Action
    navi.attack_damage = 0
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    return sol_gundelsol.action
end

sol_gundelsol.weapon = new.weapon {
    id = "megaman/sol-gundelsol",
    -- Flat 130 ticks regardless of Charge+ level (0x0203D110[0x6C]).
    charge_ticks = { 130, 130, 130, 130, 130 },
    setup = sol_gundelsol.setup,
}

return sol_gundelsol
