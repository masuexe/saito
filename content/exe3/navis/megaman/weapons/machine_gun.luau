--!strict
-- EXE3 Guts Style Machine Gun (A6BE 0x080B1EDC, triggered by 5 quick B presses via `rapid_presses`
-- -> `actions.forced_charged_shot` at 0x080B55FA):
-- Fires 6 rapid buster shots in sequence using the Guts Style buster-shoot animation (`0x0C + style_index`),
-- each dealing `(stats.attack + 1) * buster_mult` damage and honoring BrakBust (`stats.brake_buster`).
-- Verified against projectile table `0x080D5CE4`:
--   - Shots 1..5 (rows 0x0A / 0x0B): `hit_mod = 1` (flinch without mercy flash so all 6 shots connect)
--   - Shot 6     (rows 0x0F / 0x10): `hit_mod = 3` (flinch + mercy invulnerability flash)

local anims = require("../../../lib/anims")
local collision = require("../../../rules/collision")
local projectile = require("../../../lib/projectile")
local shot = require("../../../objects/projectile")
local sparks = require("../../../lib/sparks")

local machine_gun = {}

local START, SHOOT, RECOVER = 0x0, 0x4, 0x8
local SHOTS = 6
local SHOT_TICKS = 4
local SHOT_Z = 0x18_0000
local SHOT_SOUND = asset.sound("buster-shot")

local SHOT = shot.variant {
    id = "guts/machine-gun/shot",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 1,
    element = "null",
    spark = sparks.plain,
}

local BREAK_SHOT = shot.variant {
    id = "guts/machine-gun/break-shot",
    collision = { is = collision.breaking, hits = collision.hits_navis },
    hit_mod = 1,
    element = "null",
    spark = sparks.breaking,
}

local FINAL_SHOT = shot.variant {
    id = "guts/machine-gun/final-shot",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 3,
    element = "null",
    spark = sparks.plain,
}

local FINAL_BREAK_SHOT = shot.variant {
    id = "guts/machine-gun/final-break-shot",
    collision = { is = collision.breaking, hits = collision.hits_navis },
    hit_mod = 3,
    element = "null",
    spark = sparks.breaking,
}

type State = { timer: number }

local function start(me: Object, _s: State)
    me.attack_count = SHOTS
    me:set_status("using_action", true)
    me.step, me.step_init = SHOOT, 0
end

local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(anims.buster_shoot(me))
        me.anim_loaded = 0xFF
        me.step_init = 4
        s.timer = 0
    end
    if s.timer == 1 then
        local stats = battle.navi(me.alliance)
        local mult: number = stats.buster_mult or 1
        if mult < 1 then
            mult = 1
        end
        local damage = (stats.attack + 1) * mult
        local breaking: boolean = stats.brake_buster == true
        local last = me.attack_count <= 1
        local used = if last then (if breaking then FINAL_BREAK_SHOT else FINAL_SHOT) else (if breaking then BREAK_SHOT else SHOT)
        battle.play_sound(SHOT_SOUND)
        projectile.fire(me, { variant = used, damage = damage, z = SHOT_Z })
    end
    s.timer += 1
    if s.timer > SHOT_TICKS then
        me.attack_count -= 1
        if me.attack_count > 0 then
            me.step_init = 0
        else
            me.step, me.step_init = RECOVER, 0
        end
    end
end

local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = 1
        me.step_init = 4
    end
    s.timer -= 1
    if s.timer <= 0 then
        me.related1 = nil
        me.rapid_presses, me.rapid_window = 0, 0
        me:exit_attack()
    end
end

machine_gun.action = new.action {
    id = "guts/machine-gun",
    state = { timer = "i16" },
    update = function(me: Object, s: State)
        local step = me.step
        if step == START then
            start(me, s)
        elseif step == SHOOT then
            shoot(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("Guts Machine Gun phase %#x out of range", step))
        end
    end,
}

return machine_gun
