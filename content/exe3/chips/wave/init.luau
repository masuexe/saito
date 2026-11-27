--!strict
-- EXE3 Wave & Shockwave chips:
-- ShockWav (0x02C), SonicWav (0x02D), DynaWave (0x02E), BigWave (0x071),
-- Wave (0x03D), RedWave (0x03E), MudWave (0x03F).

local collision = require("../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")

local BURST_SPRITE = asset.sprite("burst")
local HIT_SPARKS = asset.sprite("hit-sparks")
local IMPACT_FX = new.effect { sprite = asset.sprite("impact"), anim = 0, palette = 0, steps = 12 }
local AQUA_FX = new.effect { sprite = HIT_SPARKS, anim = 3, palette = 0, steps = 12 }
local FIRE_FX = new.effect { sprite = HIT_SPARKS, anim = 2, palette = 0, steps = 12 }
local WOOD_FX = new.effect { sprite = HIT_SPARKS, anim = 5, palette = 0, steps = 12 }
local CRACK_SOUND = asset.sound("panel-crack")
local WAVE_SOUND = asset.sound("spreader")
local BLAST_SOUND = asset.sound("hit-bomb")

-- Mode: 0 = shockwave (blocked by holes), 1 = aqua wave, 2 = redwave (lava), 3 = mudwave (breaks panels)
type WaveState = { step_ticks: number, tick: number, mode: number }

local WAVE_KIND = new.kind {
    id = "wave/crawler",
    pool = "attack",
    state = { step_ticks = "u8", tick = "u8", mode = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            local s = me.state :: WaveState
            -- Ground shockwaves & waves stop on missing/broken panels.
            if not field.solid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            sprite:load(BURST_SPRITE)
            sprite.shadow = "hidden"
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me.anim, me.anim_loaded = 0, 0xFF
            me:set_coordinates_from_panel()
            s.tick = 0
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: WaveState
            if s.tick == 0 then
                local elem = me.element
                local spark = if elem == 1
                    then sparks.fire
                    elseif elem == 2 then sparks.aqua
                    elseif elem == 4 then sparks.wood
                    else sparks.plain
                local fx = if s.mode == 1
                    then AQUA_FX
                    elseif s.mode == 2 then FIRE_FX
                    elseif s.mode == 3 then WOOD_FX
                    else IMPACT_FX
                local self_type = if s.mode == 0 then collision.attack else collision.piercing
                battle.hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = elem,
                    z = 0x08_0000,
                    region = regions.single,
                    hit_effect = spark,
                    target = collision.hits_navis,
                    self_type = self_type,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 3,
                })
                battle.effect(me.pos, fx)
                if s.mode == 2 then
                    field.set_type(me.panel_x, me.panel_y, "lava")
                    battle.play_sound(BLAST_SOUND)
                elseif s.mode == 3 then
                    field.break_panel(me.panel_x, me.panel_y)
                    battle.play_sound(CRACK_SOUND)
                elseif s.mode == 1 then
                    battle.play_sound(WAVE_SOUND)
                else
                    battle.play_sound(CRACK_SOUND)
                end
            end
            s.tick += 1
            if s.tick >= s.step_ticks then
                local nx = me.panel_x + me.facing
                if not field.valid(nx, me.panel_y) or not field.solid(nx, me.panel_y) then
                    me:set_lifecycle("destroy")
                    return
                end
                me.panel_x = nx
                me:set_coordinates_from_panel()
                s.tick = 0
            end
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

local function spawn_wave(user: Object, x: number, y: number, elem: number, damage: number, step_ticks: number, mode: number)
    if not field.valid(x, y) then
        return
    end
    local o = battle.spawn(WAVE_KIND)
    if not o then
        return
    end
    o.panel_x, o.panel_y = x, y
    o.alliance, o.flip = user.alliance, user.flip
    o.element = elem
    o:set_damage_word(damage)
    local s = o.state :: WaveState
    s.step_ticks = step_ticks
    s.mode = mode
end

type WaveActState = { timer: number }

local function make_wave_action(id: string, elem: number, step_ticks: number, mode: number, all_rows: boolean): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        update = function(me: Object, s: WaveActState)
            if me.step_init == 0 then
                me:set_animation(0x12)
                me:open_counter_window()
                me:set_status("using_action", true)
                s.timer = 0
                me.step_init = 4
            end
            if s.timer == 6 then
                local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                local sx = me.panel_x + me.facing
                if all_rows then
                    for row = 1, 3 do
                        spawn_wave(me, sx, row, elem, damage, step_ticks, mode)
                    end
                    battle.shake_camera(2, 12)
                else
                    spawn_wave(me, sx, me.panel_y, elem, damage, step_ticks, mode)
                end
            end
            s.timer += 1
            if s.timer >= 20 then
                me:exit_attack()
            end
        end,
    }
end

local shockwav: Chip = {
    action = make_wave_action("shockwav/action", 0, 12, 0, false),
    codes = { "D", "H", "J", "L", "R" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 10,
    damage = 60,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local sonicwav: Chip = {
    action = make_wave_action("sonicwav/action", 0, 9, 0, false),
    codes = { "G", "I", "M", "S", "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 26,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local dynawave: Chip = {
    action = make_wave_action("dynawave/action", 0, 6, 0, false),
    codes = { "E", "N", "Q", "T", "V" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 42,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local bigwave: Chip = {
    action = make_wave_action("bigwave/action", 0, 6, 0, true),
    codes = { "E", "J", "M", "P", "Y" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 82,
    damage = 190,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local wave: Chip = {
    action = make_wave_action("wave/action", 2, 7, 1, true),
    codes = { "E", "I", "L", "S", "T" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 60,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local redwave: Chip = {
    action = make_wave_action("redwave/action", 1, 7, 2, true),
    codes = { "B", "F", "J", "R", "U" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 72,
    damage = 90,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local mudwave: Chip = {
    action = make_wave_action("mudwave/action", 4, 7, 3, true),
    codes = { "D", "G", "M", "V", "Z" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 84,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return {
    shockwav = shockwav,
    sonicwav = sonicwav,
    dynawave = dynawave,
    bigwave = bigwave,
    wave = wave,
    redwave = redwave,
    mudwave = mudwave,
}
