--!strict
-- EXE3 Poison chips: PoisMask (0x06C), PoisFace (0x06D), and Anubis (0x0CC).
-- Also forms the PoisPhar Program Advance (PoisMask + PoisFace + Anubis)!

local anims = require("../lib/anims")
local collision = require("../rules/collision")
local dimming_chips = require("@exelib/dimming")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")

local POISON_FX = new.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0, steps = 14 }
local PUFF_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }
local POISON_SOUND = asset.sound("panel-poison")
local APPEAR_SOUND = asset.sound("appear")

-- PoisMask / PoisFace: fires poison mist clouds onto enemy panels (dealing 100 damage + poisoning panels!).
type MaskState = { timer: number }

local function make_poison_action(id: string, wide: boolean): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        update = function(me: Object, s: MaskState)
            if me.step_init == 0 then
                me:set_animation(anims.buster_shoot(me))
                me:set_status("using_action", true)
                s.timer = 0
                me.step_init = 4
            end
            if s.timer == 2 then
                local tx = me.panel_x + me.facing
                if field.valid(tx, me.panel_y) then
                    local region = if wide then regions.wide_long else regions.line
                    local dmg = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                    battle.hitbox(me, {
                        panel_x = tx,
                        panel_y = me.panel_y,
                        element = 0,
                        z = 0x10_0000,
                        region = region,
                        hit_effect = sparks.plain,
                        target = collision.hits_navis,
                        self_type = collision.attack,
                        damage = bit32.band(dmg, 0xFFFF),
                        hit_mod = 3,
                    })
                    battle.region_effects(tx, me.panel_y, region, me.alliance, POISON_FX, 0x10_0000)
                    battle.play_sound(POISON_SOUND)
                end
            end
            s.timer += 1
            if s.timer >= 18 then
                me:exit_attack()
            end
        end,
    }
end

-- Anubis statue: drains opponent's HP continuously while standing!
type AnubisState = { life: number, tick: number, drain: number }

local ANUBIS_KIND = new.kind {
    id = "poison/anubis-statue",
    pool = "attack",
    state = { life = "u16", tick = "u16", drain = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            sprite:load(asset.sprite("grab-shot"))
            sprite.shadow = "with_sprite"
            me.anim, me.anim_loaded = 0, 0xFF
            me:set_coordinates_from_panel()
            local s = me.state :: AnubisState
            s.life = 480
            s.tick = 0
            battle.play_sound(APPEAR_SOUND)
            battle.effect(me.pos, PUFF_FX)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: AnubisState
            s.life -= 1
            if s.life <= 0 then
                battle.effect(me.pos, PUFF_FX)
                me:set_lifecycle("destroy")
                return
            end
            if s.tick % 4 == 0 then
                local opp = battle.player(bit32.bxor(me.alliance, 1))
                if opp and not opp:status("dead") and opp.hp > 0 then
                    opp.hp = math.max(0, opp.hp - s.drain)
                    if s.tick % 24 == 0 then
                        battle.effect(opp.pos, POISON_FX)
                        battle.play_sound(POISON_SOUND)
                    end
                end
            end
            s.tick += 1
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

type AnubisCtrlState = { drain: number }

local ANUBIS_CTRL = new.kind {
    id = "poison/anubis-controller",
    pool = "effect",
    state = { drain = "u8" },
    update = dimming_chips.phases {
        name = "Anubis",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local user = me.related1
                local s = me.state :: AnubisCtrlState
                if user then
                    local tx = user.panel_x + user.facing
                    local ty = user.panel_y
                    if field.valid(tx, ty) then
                        local o = battle.spawn(ANUBIS_KIND)
                        if o then
                            o.panel_x, o.panel_y = tx, ty
                            o.alliance, o.flip = user.alliance, user.flip
                            ;(o.state :: AnubisState).drain = s.drain
                        end
                    end
                end
                me.timer = 20
                me.phase_init = 4
            end
            local t = me.timer - 1
            me.timer = t
            if t < 0 then
                dimming_chips.done(me)
            end
        end,
    },
}

local function anubis_hook(drain: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(ANUBIS_CTRL, user, spec)
        if o then
            local st = o.state :: AnubisCtrlState
            st.drain = drain
        end
        return o
    end
end

local poismask: Chip = {
    action = make_poison_action("poismask/action", false),
    codes = { "A", "F", "N", "Q", "V", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 30,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local poisface: Chip = {
    action = make_poison_action("poisface/action", true),
    codes = { "A", "N", "Q", "V", "W", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 38,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local anubis: Chip = {
    codes = { "A" },
    element = "null",
    family = "summon",
    class = "mega",
    rarity = 4,
    mb = 90,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    dimming = anubis_hook(1),
}

return {
    poismask = poismask,
    poisface = poisface,
    anubis = anubis,
}
