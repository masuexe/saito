--!strict
-- MegaMan's charged shot in EXE5 (`megaman/charged-shot`): the original's weapon routine 1 (0x0800F5BC, EXE6's
-- `sub_8011A7E`), which sets up the shared charged shot (@exelib/megaman/charged_shot, EXE6's action 0x16, EXE5's
-- action 0x1E: the same code but for its muzzle flash's attachment row, 13 for EXE6's 5, the same look, and its wait,
-- 0x080EC9F0, which only the operation battle makes, marking the panel in front for 10 ticks) with EXE5's
-- buster parts, projectile and attachment, and (Attack + 1) * 10 damage. (Started without its setup, by the role
-- `forced_charged_shot`, it fires the last charged shot's projectile, or EXE6's plain shot, as EXE6's action did: EXE5's
-- table's first row is EXE5's plain shot, which the port has yet to check.) Unlike EXE6's it has no worn-out rule, and it always draws: on half the draws the shot is the one the
-- navi's charged-shot program names (NaviStats +0x4F, the plain shot when 0), else the charged shot.

local attachment = require("../../../objects/attachment")
local buster = require("../../../lib/buster")
local common = require("@exelib/megaman/charged_shot")
local marker = require("../../../lib/navi_chips/marker")
local projectile = require("../../../lib/projectile")
local variants = require("../../../objects/projectile/variants")
local operation_battle = require("../../../lib/operation_battle")

local exe5 = {}

-- The shot (EXE5's action 0x1E).
exe5.action = common.action {
    id = "megaman/charged-shot/action",
    buster = buster,
    projectiles = projectile,
    attachment = attachment.kind,
    shot_sound = asset.sound("buster-shot"),
    plain = variants.plain,
    -- 0x080EC9F0: in the operation battle, the wait, its panel in front marked (0x080E9FD2: style 8, 10
    -- ticks); else none.
    waits = function(me: Object): boolean
        if not operation_battle() then
            return false
        end
        local front = if bit32.bxor(me.alliance, me.flip) == 0 then 1 else -1
        marker.spawn(me, me.panel_x + front, me.panel_y, 10, 8, true)
        return true
    end,
}
local charged_shot = { action = exe5.action }

-- 0x0800F5BC.
function exe5.setup(navi: Object): Action
    local stats = battle.navi(navi.alliance)
    navi.attack_damage = (stats.attack + 1) * 10
    local shot = variants.charged
    if bit32.band(battle.rng_positive(), 0x10) ~= 0 then
        shot = stats.charge_shot_kind or variants.plain
    end
    local s = navi:action_state(charged_shot.action) :: { projectile: ProjectileVariant? }
    s.projectile = shot
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    return charged_shot.action
end

-- Per-effect bitmask matching `saito_match::cs_prob_fix::EXE5`.
local CS_PROB_FIX_BITS: { [ProjectileVariant]: number } = {
    [variants.breaking] = 0x0001,
    [variants.charged_attack_a0] = 0x0002,
    [variants.charged_blinding] = 0x0004,
    [variants.charged_confusing] = 0x0008,
    [variants.charged_hp_bug] = 0x0010,
    [variants.charged_lava] = 0x0020,
    [variants.charged_panel_breaking] = 0x0040,
    [variants.charged_paralyzing] = 0x0080,
    [variants.charged_poisoning] = 0x0100,
    [variants.charged_uninstalling] = 0x0200,
    [variants.chip_destruction] = 0x0400,
}

-- 0x0800FDD0: the patch cards' ChrgS (B↓ChrgS), the charged shot without the draw: (Attack + 1) * 10 damage, always
-- the shot the navi's charged-shot program names (NaviStats +0x4F; the charged shot, row 6, when 0).
-- When the host enables the probability fix for the active `charge_shot_kind`, it draws at 1/2 like `exe5.setup`.
function exe5.program_setup(navi: Object): Action
    local stats = battle.navi(navi.alliance)
    navi.attack_damage = (stats.attack + 1) * 10
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    local s = navi:action_state(charged_shot.action) :: { projectile: ProjectileVariant? }
    -- (The program's 0 is the projectile table's first row, the plain shot: none, here.)
    local program = stats.charge_shot_kind
    local shot = if program == nil or program == variants.plain then variants.charged else program
    local bit = if program then CS_PROB_FIX_BITS[program] else nil
    if bit and bit32.btest(battle.cs_prob_fix(), bit) then
        if bit32.band(battle.rng_positive(), 0x10) == 0 then
            shot = variants.charged
        end
    end
    s.projectile = shot
    return charged_shot.action
end

exe5.program = new.weapon {
    id = "megaman/program-charged-shot",
    -- The charge table's row 0x63, by Charge 0 to 5: past 4 it reads on into row 0x64's first entry.
    charge_ticks = { 100, 90, 80, 70, 60, 80 },
    setup = exe5.program_setup,
}

exe5.weapon = new.weapon {
    id = "megaman/charged-shot",
    -- The charge table's row 1 (0x0801CA6C, EXE6's values); Charge 5 reads on into row 2's first entry.
    charge_ticks = { 100, 90, 80, 70, 60, 180 },
    setup = exe5.setup,
}

return exe5
