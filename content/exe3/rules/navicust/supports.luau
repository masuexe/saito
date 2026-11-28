--!strict
-- Active in-battle NaviCust Support hooks for EXE3 PvP: Rush, Beat, and Tango.
-- Wired via rules/init.luau hooks (`opponent_chip`, `fight_tick`, `low_hp`) so each fires once per round.

local api = require("../api")
local barriers = require("../../lib/barriers")
local collision = require("../collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local statuses = require("../status")

local supports = {}

local BURST_EFFECT = new.effect { sprite = asset.sprite("burst"), anim = 0, palette = 0, steps = 16 }
local HEAL_EFFECT = new.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0, steps = 16 }
local CHIP_DESTROY_SPARK = new.effect { sprite = asset.sprite("hit-marker"), anim = 0, palette = 0, steps = 15 }

local RUSH_SOUND = asset.sound("appear")
local BEAT_SOUND = asset.sound("damage-bonus")
local TANGO_SOUND = asset.sound("sfx-recover")

local RUSH_CHIPS: { [string]: boolean } = {
    invis = true,
    mole1 = true,
    mole2 = true,
    mole3 = true,
    shadow = true,
}

local function trigger_rush(side: number, opp_navi: Object)
    local s = api.state(side)
    s.rush_used = true
    -- Strip opponent invisibility/underground and inflict paralysis!
    opp_navi:set_status("invisible", false)
    opp_navi:set_status("submerged", false)
    opp_navi:set_status_timer("flash", 0)
    opp_navi:set_status_timer("submerged", 0)
    local opp_side = opp_navi.alliance
    api.set_shadow_ticks(opp_side, 0)

    local owner = battle.player(side)
    if owner then
        battle.hitbox(owner, {
            panel_x = opp_navi.panel_x,
            panel_y = opp_navi.panel_y,
            element = 0,
            z = 0x10_0000,
            region = regions.single,
            hit_effect = sparks.heavy,
            target = collision.hits_navis,
            self_type = collision.piercing,
            damage = 0,
            hit_mod = 3,
            status = statuses.paralyze,
        })
    else
        opp_navi:set_status_timer("paralyze", 120)
        opp_navi:set_status("paralyzed", true)
    end
    battle.effect(opp_navi.pos, BURST_EFFECT)
    battle.play_sound(RUSH_SOUND)
end

-- Called from `hooks.opponent_chip(side, opp_navi)`:
-- `side` is the defender whose Rush/Beat may intercept `opp_navi`'s chip.
function supports.opponent_chip(side: number, opp_navi: Object): boolean
    local stats = battle.navi(side)
    if stats.support_bug then
        return false
    end
    local s = api.state(side)
    local chip = opp_navi.attack_chip
    if chip == nil then
        return false
    end
    local chip_id = chip.id or ""

    -- Rush: catches Invis / Mole / Shadow chips, cancels stealth, and paralyzes opponent.
    if s.rush_armed and not s.rush_used and RUSH_CHIPS[chip_id] then
        trigger_rush(side, opp_navi)
        return true
    end

    -- Beat: steals opponent's first Mega or Giga chip!
    if s.beat_armed and not s.beat_used and (chip.class == "mega" or chip.class == "giga") then
        s.beat_used = true
        battle.effect(opp_navi.pos, CHIP_DESTROY_SPARK)
        battle.play_sound(BEAT_SOUND)
        return true
    end

    return false
end

-- Also catches opponent who becomes invisible/submerged via non-chip means (e.g. BugStyle/AntiDmg).
function supports.fight_tick(side: number)
    local stats = battle.navi(side)
    if stats.support_bug then
        return
    end
    local s = api.state(side)
    if not s.rush_armed or s.rush_used then
        return
    end
    local opp = battle.player(bit32.bxor(side, 1))
    if opp and not opp:status("dead") and (opp:status("invisible") or opp:status("submerged")) then
        trigger_rush(side, opp)
    end
end

-- Called from `hooks.low_hp(side, navi)`:
-- Tango activates once when HP drops to <= 25%: heals 300 HP and raises a 100-HP Barrier!
function supports.low_hp(side: number, navi: Object): boolean
    local stats = battle.navi(side)
    if stats.support_bug then
        return false
    end
    local s = api.state(side)
    if not s.tango_armed or s.tango_used then
        return false
    end
    s.tango_used = true
    local max_hp = navi.max_hp
    navi.hp = math.min(max_hp, navi.hp + 300)
    barriers.raise(navi, barriers.barrier_100)
    battle.effect(navi.pos, HEAL_EFFECT)
    battle.play_sound(TANGO_SOUND)
    return true
end

return supports
