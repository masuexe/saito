--!strict
-- The patch cards' "ChrgS" charged shot (`patch-cards/chrgs`, the original's
-- weapon routine 0x8D, `sub_80125D0`): nine cards give it, as a bug. It is
-- MegaMan's charged shot (action 0x16), but with (Attack + 1) * 10 damage
-- however worn out the navi is, and its charged-shot program on every shot
-- (the plain charged shot without one), where the plain charged shot's
-- program comes on a lucky draw.

local charged_shot = require("../../navis/megaman/weapons/charged_shot")
local variants = require("../../objects/projectile/variants")

-- Per-effect bitmask matching `saito_match::cs_prob_fix::EXE6`.
local CS_PROB_FIX_BITS: { [ProjectileVariant]: number } = {
    [variants.breaking] = 0x0001,
    [variants.charged_bubbling] = 0x0002,
    [variants.charged_confusing] = 0x0004,
    [variants.charged_freezing] = 0x0008,
    [variants.charged_grass] = 0x0010,
    [variants.charged_hp_bug] = 0x0020,
    [variants.charged_hp_bug_marked] = 0x0040,
    [variants.charged_ice] = 0x0080,
    [variants.charged_paralyzing] = 0x0100,
    [variants.charged_pulling] = 0x0200,
    [variants.charged_road] = 0x0400,
    [variants.charged_road_back] = 0x0800,
    [variants.chip_destruction] = 0x1000,
}

-- `sub_80125D0`.
-- When the host enables the probability fix for the active `charge_shot_kind`,
-- it draws at 1/8 (rare) or 1/2 like `charged_shot.setup`.
local function setup(navi: Object): Action
    local stats = battle.navi(navi.alliance)
    navi.attack_damage = (stats.attack + 1) * 10
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    local s = navi:action_state(charged_shot.action) :: { projectile: ProjectileVariant? }
    local program = stats.charge_shot_kind
    local shot = program or variants.charged
    local bit = if program then CS_PROB_FIX_BITS[program] else nil
    if bit and bit32.btest(battle.cs_prob_fix(), bit) then
        local mask = if program and program.rare_charged_shot then 7 else 1
        if bit32.band(battle.rng_positive(), mask) ~= 0 then
            shot = variants.charged
        end
    end
    s.projectile = shot
    return charged_shot.action
end

return new.weapon {
    id = "patch-cards/chrgs",
    -- The charge table's row 0x8D, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 100, 90, 80, 70, 60, 60, 55, 50 },
    setup = setup,
}
