--!strict
-- MegaMan's buster in EXE3: Attack + 1 damage (x2 in Guts Style via `stats.buster_mult`),
-- plus BrakBust breaking-guard support.

local anims = require("../../../lib/anims")
local collision = require("../../../rules/collision")
local projectile = require("../../../lib/projectile")
local shot = require("../../../objects/projectile")
local sparks = require("../../../lib/sparks")

local buster = {}

local SHOOT, RECOVER = 0x0, 0x4
local SHOT_Z = 0x18_0000

local SHOT = shot.variant {
    id = "megaman/buster/shot",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 0,
    element = "null",
    spark = sparks.plain,
}

local BREAK_SHOT = shot.variant {
    id = "megaman/buster/break-shot",
    collision = { is = collision.breaking, hits = collision.hits_navis },
    hit_mod = 0,
    element = "null",
    spark = sparks.breaking,
}

local SHOT_SOUND = asset.sound("buster-shot")
local BLANK_SOUND = asset.sound("buster-blank")

type State = { timer: number, blank: boolean }

local function shoot(me: Object, s: State, variant: ProjectileVariant)
    if me.step_init == 0 then
        local blanks = battle.navi(me.alliance).buster_blanks
        local blank = bit32.band(battle.rng(), 0xF) + 1 <= blanks
        s.blank = blank
        -- EXE3 Battle MegaMan 00-00: 0x0C+style has built-in muzzle flash; 0x14+style is blank/no-flash (0x080B0EF8..0x080B0F14).
        me:set_animation(if blank then anims.buster_blank(me) else anims.buster_shoot(me))
        me:set_status("using_action", true)
        me.step_init = 4
        s.timer = 0
    end
    if s.timer == 1 then
        if not s.blank then
            local stats = battle.navi(me.alliance)
            battle.play_sound(SHOT_SOUND)
            local damage = me.attack_damage
            local used = if stats.brake_buster then BREAK_SHOT else variant
            me.attack_count = projectile.fire(me, { variant = used, damage = damage, z = SHOT_Z })
        else
            battle.play_sound(BLANK_SOUND)
        end
    end
    s.timer += 1
    if s.timer > 4 then
        me.step = RECOVER
        me.step_init = 0
    end
end

local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = me.attack_count
        me.step_init = 4
    end
    s.timer -= 1
    if s.timer < 0 then
        me.related1 = nil
        me:exit_attack(true)
        return
    end
    if not me:can_move() then
        return
    end
    local dir = me:held_direction()
    if dir == 0 or me:step_target(dir) == nil then
        return
    end
    me.related1 = nil
    me:exit_attack(true)
    me:start_move(dir)
end

function buster.shot_action(id: string, variant: ProjectileVariant): Action
    return new.action {
        id = id,
        state = { timer = "i16", blank = "bool" },
        update = function(me: Object, s: State)
            local step = me.step
            if step == SHOOT then
                shoot(me, s, variant)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("buster shot phase %#x out of range", step))
            end
        end,
    }
end

buster.shot = buster.shot_action("megaman/buster/shot", SHOT)

function buster.setup(navi: Object): Action
    local stats = battle.navi(navi.alliance)
    local mult: number = stats.buster_mult or 1
    if mult < 1 then
        mult = 1
    end
    navi.attack_damage = (stats.attack + 1) * mult
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    return buster.shot
end

buster.weapon = new.weapon {
    id = "megaman/buster",
    charge_ticks = { 0, 0, 0, 0, 0 },
    setup = buster.setup,
}

return buster
