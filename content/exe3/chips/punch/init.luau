--!strict
-- EXE3 Punch, Hammer, Dash & AirSwrd chips:
-- GutPunch (0x02F), GutStrgt (0x030), GutImpct (0x031), Hammer (0x040), DashAtk (0x035), AirSwrd (0x02A).

local collision = require("../rules/collision")
local element = require("@exelib/element")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")

local IMPACT_FX = new.effect { sprite = asset.sprite("impact"), anim = 0, palette = 0, steps = 16 }
local BURST_FX = new.effect { sprite = asset.sprite("burst"), anim = 0, palette = 0, steps = 12 }
local SLASH_FX = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0, steps = 12 }
local HEAVY_SOUND = asset.sound("obstacle-throw")
local CRACK_SOUND = asset.sound("crack-shot")
local SWING_SOUND = asset.sound("sword-swing")
local WIND_SOUND = asset.sound("spreader")

local BREAK_ELEM = element.byte("null", { "break" })
local WIND_SWORD_ELEM = element.byte("null", { "wind", "sword" })

local function push_entities_at(me: Object, tx: number, ty: number, push_dist: number, launch_obstacles: boolean, damage: number)
    -- Push enemy actors back along facing direction.
    local opp_side = bit32.bxor(me.alliance, 1)
    for _, opp in battle.alive_actors(opp_side) do
        if opp.panel_x == tx and opp.panel_y == ty and not opp:status("dead") and not opp:status("super_armor") then
            for _ = 1, push_dist do
                local nx = opp.panel_x + me.facing
                if field.valid(nx, ty) then
                    local p = field.panel(nx, ty)
                    if p and p.alliance == opp.alliance and p.kind ~= "missing" and p.kind ~= "broken" and opp:can_move() then
                        opp.panel_x = nx
                        opp:set_coordinates_from_panel()
                        opp:update_collision_panels()
                    else
                        break
                    end
                else
                    break
                end
            end
        end
    end
    -- Launch or destroy obstacles standing on (tx, ty).
    if launch_obstacles then
        for _, obj in field.all_objects() do
            if obj.active and obj.panel_x == tx and obj.panel_y == ty and obstacle.present(obj) then
                local land_x = math.clamp(tx + me.facing * 3, 1, 6)
                obstacle.throw(obj, me.alliance, land_x, ty, 4, damage)
                battle.play_sound(HEAVY_SOUND)
            end
        end
    end
end

-- =============================================================================
-- GutPunch / GutStrgt / GutImpct / Hammer
-- =============================================================================
type PunchState = { timer: number }

local function make_punch_action(
    id: string,
    reach: number,
    push_dist: number,
    launch_obstacles: boolean,
    destroys_obstacles: boolean,
    cracks_panel: boolean
): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        update = function(me: Object, s: PunchState)
            if me.step_init == 0 then
                me:set_animation(0x12)
                me:open_counter_window()
                me:set_status("using_action", true)
                s.timer = 0
                me.step_init = 4
            end
            if s.timer == 6 then
                local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                local tx = me.panel_x + me.facing
                if field.valid(tx, me.panel_y) then
                    local region = if reach >= 2 then regions.long else regions.single
                    battle.hitbox(me, {
                        panel_x = tx,
                        panel_y = me.panel_y,
                        element = BREAK_ELEM,
                        z = 0x10_0000,
                        region = region,
                        hit_effect = sparks.breaking,
                        target = collision.hits_navis,
                        self_type = collision.breaking,
                        damage = damage,
                        hit_mod = 3,
                    })
                    battle.region_effects(tx, me.panel_y, region, me.alliance, IMPACT_FX, 0x10_0000)
                    battle.play_sound(CRACK_SOUND)
                    battle.shake_camera(if push_dist >= 2 then 2 else 1, 10)
                    for step = 0, reach - 1 do
                        local px = tx + me.facing * step
                        if field.valid(px, me.panel_y) then
                            if destroys_obstacles then
                                for _, obj in field.all_objects() do
                                    if obj.active and obj.panel_x == px and obj.panel_y == me.panel_y and obstacle.present(obj) then
                                        obstacle.remove(obj)
                                    end
                                end
                            else
                                push_entities_at(me, px, me.panel_y, push_dist, launch_obstacles, damage)
                            end
                            if cracks_panel then
                                field.crack(px, me.panel_y)
                            end
                        end
                    end
                end
            end
            s.timer += 1
            if s.timer >= 20 then
                me:exit_attack()
            end
        end,
    }
end

-- =============================================================================
-- DashAtk: dashes forward across the entire row piercing enemies and returns!
-- =============================================================================
type DashState = { timer: number, orig_x: number, cur_x: number, returning: boolean }

local DASHATK_ACTION = new.action {
    id = "dashatk/action",
    state = { timer = "u16", orig_x = "u8", cur_x = "u8", returning = "bool" },
    update = function(me: Object, s: DashState)
        if me.step_init == 0 then
            me:set_animation(0x12)
            me:open_counter_window()
            me:set_status("using_action", true)
            s.timer = 0
            s.orig_x = me.panel_x
            s.cur_x = me.panel_x
            s.returning = false
            battle.play_sound(WIND_SOUND)
            me.step_init = 4
        end
        s.timer += 1
        if not s.returning then
            if s.timer % 3 == 0 then
                local nx = s.cur_x + me.facing
                if field.valid(nx, me.panel_y) then
                    s.cur_x = nx
                    me.panel_x = nx
                    me:set_coordinates_from_panel()
                    me:update_collision_panels()
                    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                    battle.hitbox(me, {
                        panel_x = nx,
                        panel_y = me.panel_y,
                        element = 0,
                        z = 0x10_0000,
                        region = regions.single,
                        hit_effect = sparks.impact,
                        target = collision.hits_navis,
                        self_type = collision.piercing,
                        damage = damage,
                        hit_mod = 3,
                    })
                    battle.effect(me.pos, BURST_FX)
                else
                    s.returning = true
                    s.timer = 0
                end
            end
        else
            if s.timer >= 6 then
                me.panel_x = s.orig_x
                me:set_coordinates_from_panel()
                me:update_collision_panels()
                me:exit_attack()
            end
        end
    end,
}

-- =============================================================================
-- AirSwrd: wind-slashing sword that cuts ahead and pushes enemy back 1 panel!
-- =============================================================================
type AirSwrdState = { timer: number }

local AIRSWRD_ACTION = new.action {
    id = "airswrd/action",
    state = { timer = "u16" },
    update = function(me: Object, s: AirSwrdState)
        if me.step_init == 0 then
            me:set_animation(0x12)
            me:open_counter_window()
            me:set_status("using_action", true)
            s.timer = 0
            me.step_init = 4
        end
        if s.timer == 5 then
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            local tx = me.panel_x + me.facing
            if field.valid(tx, me.panel_y) then
                battle.hitbox(me, {
                    panel_x = tx,
                    panel_y = me.panel_y,
                    element = WIND_SWORD_ELEM,
                    z = 0x10_0000,
                    region = regions.wide,
                    hit_effect = sparks.plain,
                    target = collision.hits_navis,
                    self_type = collision.slash,
                    damage = damage,
                    hit_mod = 3,
                })
                battle.region_effects(tx, me.panel_y, regions.wide, me.alliance, SLASH_FX, 0x10_0000)
                battle.play_sound(SWING_SOUND)
                battle.play_sound(WIND_SOUND)
                for row = 1, 3 do
                    if math.abs(row - me.panel_y) <= 1 then
                        push_entities_at(me, tx, row, 1, false, damage)
                    end
                end
            end
        end
        s.timer += 1
        if s.timer >= 18 then
            me:exit_attack()
        end
    end,
}

local gutpunch: Chip = {
    action = make_punch_action("gutpunch/action", 1, 1, true, false, false),
    codes = { "B", "C", "D", "E", "F" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 0,
    mb = 14,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local gutstrgt: Chip = {
    action = make_punch_action("gutstrgt/action", 2, 1, true, false, false),
    codes = { "O", "P", "Q", "R", "S" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 1,
    mb = 30,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local gutimpct: Chip = {
    action = make_punch_action("gutimpct/action", 1, 2, true, false, true),
    codes = { "G", "H", "I", "J", "K" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 160,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local hammer: Chip = {
    action = make_punch_action("hammer/action", 1, 0, false, true, true),
    codes = { "B", "G", "L", "P", "T", "*" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 1,
    mb = 16,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local dashatk: Chip = {
    action = DASHATK_ACTION,
    codes = { "C", "D", "G", "J", "Z", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 11,
    damage = 90,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local airswrd: Chip = {
    action = AIRSWRD_ACTION,
    codes = { "C", "E", "H", "J", "R", "*" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return {
    gutpunch = gutpunch,
    gutstrgt = gutstrgt,
    gutimpct = gutimpct,
    hammer = hammer,
    dashatk = dashatk,
    airswrd = airswrd,
}
