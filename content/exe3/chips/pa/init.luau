--!strict
-- EXE3 Complete Program Advances (all 32 real PAs: 0x140..0x15F) + BassGS Giga chip (0x138).
-- All 32 Program Advance recipes are live-wired below.

local anims = require("../lib/anims")
local antis = require("./anti")
local attachment = require("../objects/attachment")
local barrier_chips = require("./barrier")
local barrier_visual = require("../objects/barrier_visual")
local barriers = require("../lib/barriers")
local blades = require("./sword")
local bullet = require("../objects/bullet")
local cannons = require("./cannon")
local collision = require("../rules/collision")
local common_visual = require("@exelib/barriers/visual")
local dimming_chips = require("@exelib/dimming")
local magnums = require("./magnum")
local mega = require("./mega")
local metagels = require("./metagel")
local navis = require("./navis")
local poisons = require("./poison")
local projectile = require("../objects/projectile")
local punches = require("./punch")
local recovs = require("./recov")
local regions = require("@exelib/regions")
local rockcubes = require("./rockcube")
local shuriken = require("../objects/shuriken")
local sparks = require("../lib/sparks")
local spreaders = require("./spreader")
local stages = require("./stage")
local swords = require("../lib/swords")
local traps = require("./trap")
local utilities = require("./utility")
local yoyos = require("./yoyo")

local cannon_sprite = asset.sprite("cannon")
local big_slash_sprite = asset.sprite("big-slash")
local hit_sparks_sprite = asset.sprite("hit-sparks")

local Z_CANON1_ARM = attachment.look { sprite = cannon_sprite, palette = 0 }
local Z_CANON2_ARM = attachment.look { sprite = cannon_sprite, palette = 1 }
local Z_CANON3_ARM = attachment.look { sprite = cannon_sprite, palette = 2 }
local Z_CANON_SHOT = projectile.variant {
    id = "pa/z-canon-shot",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 3,
    element = "null",
    spark = sparks.blast,
}

local CANNON_SOUND = asset.sound("cannon")
local APPEAR_SOUND = asset.sound("appear")
local SWING_SOUND = asset.sound("big-sword-swing")
local SWORD_SOUND = asset.sound("sword-swing")
local POISON_SOUND = asset.sound("panel-poison")
local BARRIER_SOUND = asset.sound("barrier")
local SPREADER_SOUND = asset.sound("spreader")
local BLAST_SOUND = asset.sound("hit-bomb")
local CRACK_SOUND = asset.sound("panel-crack")
local THROW_SOUND = asset.sound("obstacle-throw")
local RECOVER_SOUND = asset.sound("sfx-recover")
local SHIELD_SOUND = asset.sound("shield")
local GRAB_LAND_SOUND = asset.sound("grab-shot-land")

local BIG_SLASH = new.effect { sprite = big_slash_sprite, anim = 0, palette = 0, steps = 15 }
local FIRE_FX = new.effect { sprite = hit_sparks_sprite, anim = 2, palette = 0, steps = 16 }
local AQUA_FX = new.effect { sprite = hit_sparks_sprite, anim = 3, palette = 0, steps = 16 }
local ELEC_FX = new.effect { sprite = hit_sparks_sprite, anim = 4, palette = 0, steps = 16 }
local WOOD_FX = new.effect { sprite = hit_sparks_sprite, anim = 5, palette = 0, steps = 16 }
local BREAK_FX = new.effect { sprite = hit_sparks_sprite, anim = 1, palette = 0, steps = 12 }
local BLAST_FX = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0, steps = 18 }
local IMPACT_FX = new.effect { sprite = asset.sprite("impact"), anim = 0, palette = 0, steps = 18 }
local BURST_FX = new.effect { sprite = asset.sprite("burst"), anim = 0, palette = 0, steps = 14 }
local FLASH_FX = new.effect { sprite = asset.sprite("pink-flash"), anim = 1, palette = 0, steps = 10 }
local HEAL_FX = new.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0, steps = 16 }
local POISON_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 14 }
local PUFF_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }
local YOYO_FX = new.effect { sprite = asset.sprite("yoyo"), anim = 0, palette = 0, steps = 10 }
local HEAT_BLAST_FX = new.effect { sprite = asset.sprite("heat-blast"), anim = 0, palette = 0, steps = 36 }
local BUBBLE_SPLASH_FX = new.effect { sprite = asset.sprite("bubble-splash"), anim = 0, palette = 0, steps = 26 }
local LIGHTNING_FX = new.effect { sprite = asset.sprite("lightning"), anim = 0, palette = 0, steps = 12 }
local WOOD_VINES_FX = new.effect { sprite = asset.sprite("wood-vines"), anim = 0, palette = 0, steps = 27 }
local RATTON_FX = new.effect { sprite = asset.sprite("ratton"), anim = 1, palette = 0, steps = 14 }
local GROUND_CRACK_FX = new.effect { sprite = asset.sprite("ground-crack"), anim = 0, palette = 0, steps = 22 }

local SWORD_HIT = { hits = collision.hits_navis, is = collision.slash }
local PA_CODES = { "A", "B", "C", "D", "E", "*" }

local function dim_hitbox(owner: Object, spec: HitboxSpec)
    local box = battle.hitbox(owner, spec)
    if box then
        box.run_while_dimmed = true
    end
end

local function enemy_target(side: number): (number, number)
    local opp = battle.player(bit32.bxor(side, 1))
    if opp and not opp:status("dead") then
        return opp.panel_x, opp.panel_y
    end
    local actors = battle.alive_actors(bit32.bxor(side, 1))
    if #actors > 0 then
        return actors[1].panel_x, actors[1].panel_y
    end
    return if side == 0 then 5 else 2, 2
end

local function enemy_front_col(side: number): number
    local opp_side = bit32.bxor(side, 1)
    if side == 0 then
        for x = 1, 6 do
            for y = 1, 3 do
                local p = field.panel(x, y)
                if p and p.alliance == opp_side then
                    return x
                end
            end
        end
        return 4
    else
        for x = 6, 1, -1 do
            for y = 1, 3 do
                local p = field.panel(x, y)
                if p and p.alliance == opp_side then
                    return x
                end
            end
        end
        return 3
    end
end

-- =============================================================================
-- 1. Z-Canon1 / Z-Canon2 / Z-Canon3 (0x140..0x142): Invincible rapid Cannon barrage!
-- =============================================================================
type ZTimerState = { timer: number, cooldown: number, shots: number }

local function make_zcanon_action(id: string, arm: AttachmentLook): Action
    return new.action {
        id = id,
        state = { timer = "u16", cooldown = "u8", shots = "u8" },
        update = function(me: Object, s: ZTimerState)
            if me.step_init == 0 then
                me:set_animation(anims.CANNON_RAISE)
                me:set_status("using_action", true)
                me:set_status_timer("flash", 180)
                me:set_status("invisible", true)
                attachment.attach(me, arm, "related", 0)
                s.timer = 180
                s.cooldown = 0
                s.shots = 0
                me.step_init = 4
            end
            if s.cooldown > 0 then
                s.cooldown -= 1
            elseif s.shots < 6 and (me:held("a") or s.timer % 26 == 0) then
                s.shots += 1
                s.cooldown = 16
                local dmg = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                battle.play_sound(CANNON_SOUND)
                projectile.spawn(me, me.panel_x + me.facing, me.panel_y, {
                    variant = Z_CANON_SHOT,
                    damage = dmg,
                    z = 0x18_0000,
                })
            end
            s.timer -= 1
            if s.timer <= 0 or s.shots >= 6 and s.cooldown == 0 then
                me.related1 = nil
                me:exit_attack()
            end
        end,
    }
end

local z_canon1: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 0, code_run = { chip = cannons.cannon, count = 3 } },
    },
    action = make_zcanon_action("z-canon1/action", Z_CANON1_ARM),
}

local z_canon2: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 60,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 1, code_run = { chip = cannons.hicannon, count = 3 } },
    },
    action = make_zcanon_action("z-canon2/action", Z_CANON2_ARM),
}

local z_canon3: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 2, code_run = { chip = cannons["m-cannon"], count = 3 } },
    },
    action = make_zcanon_action("z-canon3/action", Z_CANON3_ARM),
}

-- =============================================================================
-- 2. Z-Punch / Z-Strght / Z-Impact (0x143..0x145): Invincible rapid Guts punch barrage!
-- =============================================================================
local function make_zpunch_action(id: string, reach: Region, shakes: boolean): Action
    return new.action {
        id = id,
        state = { timer = "u16", cooldown = "u8", shots = "u8" },
        update = function(me: Object, s: ZTimerState)
            if me.step_init == 0 then
                me:set_animation(anims.PUNCH)
                me:set_status("using_action", true)
                me:set_status_timer("flash", 180)
                me:set_status("invisible", true)
                s.timer = 180
                s.cooldown = 0
                s.shots = 0
                me.step_init = 4
            end
            if s.cooldown > 0 then
                s.cooldown -= 1
            elseif s.shots < 6 and (me:held("a") or s.timer % 26 == 0) then
                s.shots += 1
                s.cooldown = 16
                me:set_animation(anims.PUNCH)
                local tx = me.panel_x + me.facing
                if field.valid(tx, me.panel_y) then
                    local dmg = bit32.band(int.u32(me.attack_damage + me.extra), 0xFFFF)
                    battle.hitbox(me, {
                        panel_x = tx,
                        panel_y = me.panel_y,
                        element = 0,
                        z = 0x10_0000,
                        region = reach,
                        hit_effect = sparks.breaking,
                        target = collision.hits_navis,
                        self_type = collision.breaking,
                        damage = dmg,
                        hit_mod = 3,
                    })
                    battle.region_effects(tx, me.panel_y, reach, me.alliance, IMPACT_FX, 0x10_0000)
                    battle.play_sound(THROW_SOUND)
                    if shakes then
                        battle.shake_camera(2, 8)
                    end
                end
            end
            s.timer -= 1
            if s.timer <= 0 or s.shots >= 6 and s.cooldown == 0 then
                me:exit_attack()
            end
        end,
    }
end

local z_punch: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 80,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 32, code_run = { chip = punches.gutpunch, count = 3 } },
    },
    action = make_zpunch_action("z-punch/action", regions.single, false),
}

local z_strght: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 33, code_run = { chip = punches.gutstrgt, count = 3 } },
    },
    action = make_zpunch_action("z-strght/action", regions.long, false),
}

local z_impact: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 160,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 34, code_run = { chip = punches.gutimpct, count = 3 } },
    },
    action = make_zpunch_action("z-impact/action", regions.long, true),
}

-- =============================================================================
-- 3. Z-Varibl (0x146): Invincible rapid VarSwrd slash barrage!
-- =============================================================================
local Z_VARIBL_REGIONS = { regions.wide, regions.long, regions.wide_long, regions.line }

local Z_VARIBL_ACTION = new.action {
    id = "z-varibl/action",
    state = { timer = "u16", cooldown = "u8", shots = "u8" },
    update = function(me: Object, s: ZTimerState)
        if me.step_init == 0 then
            me.anim = anims.SWORD_SWING
            me:set_status("using_action", true)
            me:set_status_timer("flash", 180)
            me:set_status("invisible", true)
            attachment.attach(me, swords.blade, "related", 0)
            s.timer = 180
            s.cooldown = 0
            s.shots = 0
            me.step_init = 4
        end
        if s.cooldown > 0 then
            s.cooldown -= 1
        elseif s.shots < 6 and (me:held("a") or s.timer % 26 == 0) then
            s.shots += 1
            s.cooldown = 18
            local tx = me.panel_x + me.facing
            if field.valid(tx, me.panel_y) then
                local reg = Z_VARIBL_REGIONS[((s.shots - 1) % #Z_VARIBL_REGIONS) + 1]
                local dmg = bit32.band(int.u32(me.attack_damage + me.extra), 0xFFFF)
                battle.hitbox(me, {
                    panel_x = tx,
                    panel_y = me.panel_y,
                    element = 0,
                    z = 0x10_0000,
                    region = reg,
                    hit_effect = sparks.heavy,
                    target = collision.hits_navis,
                    self_type = collision.slash,
                    damage = dmg,
                    hit_mod = 3,
                })
                local cx, cy = field.center(tx, me.panel_y)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), BIG_SLASH, me.alliance)
                battle.play_sound(SWING_SOUND)
            end
        end
        s.timer -= 1
        if s.timer <= 0 or s.shots >= 6 and s.cooldown == 0 then
            me.related1 = nil
            me:exit_attack()
        end
    end,
}

local z_varibl: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 160,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 3, code_run = { chip = blades.varswrd, count = 3 } },
    },
    action = Z_VARIBL_ACTION,
}

-- =============================================================================
-- 4. Z-Yoyo1 / Z-Yoyo2 / Z-Yoyo3 (0x147..0x149): Invincible rapid multi-hit Yo-Yo throws!
-- =============================================================================
local function make_zyoyo_action(id: string): Action
    return new.action {
        id = id,
        state = { timer = "u16", cooldown = "u8", shots = "u8" },
        update = function(me: Object, s: ZTimerState)
            if me.step_init == 0 then
                me:set_animation(anims.THROW_LOW)
                me:set_status("using_action", true)
                me:set_status_timer("flash", 180)
                me:set_status("invisible", true)
                s.timer = 180
                s.cooldown = 0
                s.shots = 0
                me.step_init = 4
            end
            if s.cooldown > 0 then
                s.cooldown -= 1
            elseif s.shots < 9 and (me:held("a") or s.timer % 18 == 0) then
                s.shots += 1
                s.cooldown = 12
                local tx = me.panel_x + me.facing
                if field.valid(tx, me.panel_y) then
                    local dmg = bit32.band(int.u32(me.attack_damage + me.extra), 0xFFFF)
                    battle.hitbox(me, {
                        panel_x = tx,
                        panel_y = me.panel_y,
                        element = 0,
                        z = 0x10_0000,
                        region = regions.line,
                        hit_effect = sparks.heavy,
                        target = collision.hits_navis,
                        self_type = collision.attack,
                        damage = dmg,
                        hit_mod = if s.shots == 9 then 3 else 1,
                    })
                    battle.region_effects(tx, me.panel_y, regions.line, me.alliance, YOYO_FX, 0x10_0000)
                    battle.play_sound(SPREADER_SOUND)
                end
            end
            s.timer -= 1
            if s.timer <= 0 or s.shots >= 9 and s.cooldown == 0 then
                me:exit_attack()
            end
        end,
    }
end

local z_yoyo1: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 35, code_run = { chip = yoyos["yo-yo1"], count = 3 } },
    },
    action = make_zyoyo_action("z-yoyo1/action"),
}

local z_yoyo2: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 50,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 36, code_run = { chip = yoyos["yo-yo2"], count = 3 } },
    },
    action = make_zyoyo_action("z-yoyo2/action"),
}

local z_yoyo3: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 60,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 37, code_run = { chip = yoyos["yo-yo3"], count = 3 } },
    },
    action = make_zyoyo_action("z-yoyo3/action"),
}

-- =============================================================================
-- 5. Z-Step1 / Z-Step2 (0x14A..0x14B): Invincible rapid StepSwrd / StepCros slashes!
-- =============================================================================
local function make_zstep_action(id: string, region: Region): Action
    return new.action {
        id = id,
        state = { timer = "u16", cooldown = "u8", shots = "u8" },
        update = function(me: Object, s: ZTimerState)
            if me.step_init == 0 then
                me.anim = anims.SWORD_SWING
                me:set_status("using_action", true)
                me:set_status_timer("flash", 180)
                me:set_status("invisible", true)
                attachment.attach(me, swords.blade, "related", 0)
                s.timer = 180
                s.cooldown = 0
                s.shots = 0
                me.step_init = 4
            end
            if s.cooldown > 0 then
                s.cooldown -= 1
            elseif s.shots < 5 and (me:held("a") or s.timer % 28 == 0) then
                s.shots += 1
                s.cooldown = 20
                local tx = math.clamp(me.panel_x + me.facing * 3, 1, 6)
                local ty = me.panel_y
                local dmg = bit32.band(int.u32(me.attack_damage + me.extra), 0xFFFF)
                battle.hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = region,
                    hit_effect = sparks.heavy,
                    target = collision.hits_navis,
                    self_type = collision.slash,
                    damage = dmg,
                    hit_mod = 3,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), BIG_SLASH, me.alliance)
                battle.play_sound(SWING_SOUND)
            end
            s.timer -= 1
            if s.timer <= 0 or s.shots >= 5 and s.cooldown == 0 then
                me.related1 = nil
                me:exit_attack()
            end
        end,
    }
end

local z_step1: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 130,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 4, code_run = { chip = blades.stepswrd, count = 3 } },
    },
    action = make_zstep_action("z-step1/action", regions.wide),
}

local z_step2: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 130,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 5, code_run = { chip = blades.stepcros, count = 3 } },
    },
    action = make_zstep_action("z-step2/action", regions.x),
}

-- =============================================================================
-- 6. TimeBom+ (`timebom`, 0x14C): 500-dmg TimeBomb placed on enemy front panel!
-- =============================================================================
type BombObjState = { countdown: number }

local TIMEBOM_OBJ = new.kind {
    id = "pa/timebom-plus-bomb",
    pool = "attack",
    state = { countdown = "u16" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            sprite:load(asset.sprite("bomb"))
            sprite.shadow = "with_sprite"
            me.anim, me.anim_loaded = 0, 0xFF
            me:set_coordinates_from_panel()
            ;(me.state :: BombObjState).countdown = 120
            battle.play_sound(GRAB_LAND_SOUND)
            battle.effect(me.pos, PUFF_FX)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: BombObjState
            s.countdown -= 1
            if s.countdown % 40 == 0 and s.countdown > 0 then
                battle.effect(me.pos, FLASH_FX)
                battle.play_sound(asset.sound("sfx-confirm"))
            end
            if s.countdown <= 0 then
                local region = regions.other_area[me.alliance]
                battle.hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = 0,
                    z = 0x10_0000,
                    region = region,
                    hit_effect = sparks.blast,
                    target = collision.hits_navis,
                    self_type = collision.piercing_break,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 5,
                })
                battle.region_effects(me.panel_x, me.panel_y, region, me.alliance, BLAST_FX, 0x10_0000)
                battle.play_sound(BLAST_SOUND)
                battle.shake_camera(3, 22)
                battle.palette_flash(0, 12, true, true)
                me:set_lifecycle("destroy")
                return
            end
            me:update_sprite()
        else
            me:destroy()
        end
    end,
}

local TIMEBOM_CTRL = new.kind {
    id = "pa/timebom-plus-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "TimeBom+",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local col = enemy_front_col(me.alliance)
                local o = battle.spawn(TIMEBOM_OBJ)
                if o then
                    o.panel_x, o.panel_y = col, 2
                    o.alliance, o.flip = me.alliance, me.flip
                    o:set_damage_word(dimming_chips.damage_plus(me, 0))
                end
                battle.play_sound(APPEAR_SOUND)
                me.timer = 18
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

local timebom: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 500,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = {
        { order = 38, code_run = { chip = traps.timebomb, count = 3 } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(TIMEBOM_CTRL, user, spec)
    end,
}

-- =============================================================================
-- 7. H-Burst (0x14D): Spreader x3 code-run -> 5x100 multi-explosion!
-- =============================================================================
type CountState = { hits: number }

local HBURST_KIND = new.kind {
    id = "pa/h-burst-controller",
    pool = "effect",
    state = { hits = "u8" },
    update = dimming_chips.phases {
        name = "H-Burst",
        effect = function(me: Object)
            local s = me.state :: CountState
            if me.phase_init == 0 then
                battle.play_sound(SPREADER_SOUND)
                me.timer = 0
                s.hits = 0
                me.phase_init = 4
            end
            if me.timer % 6 == 2 and s.hits < 5 then
                s.hits += 1
                local tx, ty = enemy_target(me.alliance)
                local reg = if s.hits % 2 == 1 then regions.plus else regions.square
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = reg,
                    hit_effect = sparks.blast,
                    target = collision.hits_navis,
                    self_type = collision.attack,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if s.hits == 5 then 3 else 1,
                })
                battle.region_effects(tx, ty, reg, me.alliance, FLASH_FX, 0x10_0000)
                battle.play_sound(BLAST_SOUND)
                battle.shake_camera(2, 6)
            end
            me.timer += 1
            if me.timer >= 38 then
                dimming_chips.done(me)
            end
        end,
    },
}

local h_burst: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = {
        { order = 6, code_run = { chip = spreaders.spreader, count = 3 } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(HBURST_KIND, user, spec, true)
    end,
}

-- =============================================================================
-- 8 & 9. BubSprd (0x14E) and HeatSprd (0x14F): 300-dmg elemental 3x3 spreaders!
-- =============================================================================
local HEAT_BURST = bullet.variant { region = regions.square, hit_mod = 3, effect = HEAT_BLAST_FX, sound = SPREADER_SOUND }
local BUB_BURST = bullet.variant { region = regions.square, hit_mod = 3, effect = BUBBLE_SPLASH_FX, sound = SPREADER_SOUND }

type SpreadState = { timer: number }

local function make_big_spreader(id: string, elem: number, burst: BulletVariant): Action
    return new.action {
        id = id,
        state = { timer = "u16" },
        update = function(me: Object, s: SpreadState)
            if me.step_init == 0 then
                me:set_animation(anims.buster_shoot(me))
                me:set_status("using_action", true)
                s.timer = 0
                me.step_init = 4
            end
            if s.timer == 2 then
                local dmg = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                battle.play_sound(SPREADER_SOUND)
                bullet.fire(me, me.panel_x + me.facing, me.panel_y, 0x10_0000, burst, elem, dmg)
            end
            s.timer += 1
            if s.timer >= 18 then
                me:exit_attack()
            end
        end,
    }
end

local bubsprd: Chip = {
    codes = PA_CODES,
    element = "aqua",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 300,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 7, sequence = { spreaders.bubbler, spreaders["bub-v"], spreaders.bublside } },
    },
    action = make_big_spreader("bubsprd/action", 2, BUB_BURST),
}

local heatsprd: Chip = {
    codes = PA_CODES,
    element = "fire",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 300,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 8, sequence = { spreaders.heatshot, spreaders["heat-v"], spreaders.heatside } },
    },
    action = make_big_spreader("heatsprd/action", 1, HEAT_BURST),
}

-- =============================================================================
-- 10. LifeSwrd (0x150): Sword + WideSwrd + LongSwrd (400 dmg 3x2 slash!)
-- =============================================================================
local lifeswrd: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 400,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 9, sequence = { blades.sword, blades.wideswrd, blades.longswrd } },
    },
    action = swords.slash {
        id = "lifeswrd/action",
        blade = swords.blade,
        hit = { region = regions.wide_long, collision = SWORD_HIT, hit_mod = 3 },
        effect = BIG_SLASH,
        sound = swords.big_swing,
    },
}

-- =============================================================================
-- 11. ElemSwrd (0x151): FireSwrd + AquaSwrd + ElecSwrd + BambSwrd (4-hit 150 each!)
-- =============================================================================
local ELEM_IDS = { 1, 2, 3, 4 }
local ELEM_SPARKS = { sparks.fire, sparks.aqua, sparks.elec, sparks.wood }
local ELEM_FXS = { FIRE_FX, AQUA_FX, ELEC_FX, WOOD_FX }
local ELEM_BLADES = { swords.fire_blade, swords.aqua_blade, swords.elec_blade, swords.wood_blade }

local ELEMSWRD_ACTION = new.action {
    id = "elemswrd/action",
    state = { timer = "u16", cooldown = "u8", shots = "u8" },
    update = function(me: Object, s: ZTimerState)
        if me.step_init == 0 then
            me.anim = anims.SWORD_SWING
            me:set_status("using_action", true)
            attachment.attach(me, swords.fire_blade, "related", 0)
            s.timer = 0
            s.shots = 0
            me.step_init = 4
        end
        if s.timer % 8 == 2 and s.shots < 4 then
            s.shots += 1
            local idx = s.shots
            attachment.attach(me, ELEM_BLADES[idx], "related", 0)
            local tx = me.panel_x + me.facing
            if field.valid(tx, me.panel_y) then
                local dmg = bit32.band(int.u32(me.attack_damage + me.extra), 0xFFFF)
                battle.hitbox(me, {
                    panel_x = tx,
                    panel_y = me.panel_y,
                    element = ELEM_IDS[idx],
                    z = 0x10_0000,
                    region = regions.wide_long,
                    hit_effect = ELEM_SPARKS[idx],
                    target = collision.hits_navis,
                    self_type = collision.slash,
                    damage = dmg,
                    hit_mod = if idx == 4 then 3 else 1,
                })
                battle.region_effects(tx, me.panel_y, regions.wide_long, me.alliance, ELEM_FXS[idx], 0x10_0000)
                battle.play_sound(SWING_SOUND)
            end
        end
        s.timer += 1
        if s.timer >= 36 then
            me.related1 = nil
            me:exit_attack()
        end
    end,
}

local elemswrd: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 150,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 10, sequence = { blades.fireswrd, blades.aquaswrd, blades.elecswrd, blades.bambswrd } },
    },
    action = ELEMSWRD_ACTION,
}

-- =============================================================================
-- 12. EvilCut (0x152): StepSwrd + HeroSwrd + StepCros (dim triple 150x3 slash!)
-- =============================================================================
local EVILCUT_KIND = new.kind {
    id = "pa/evilcut-controller",
    pool = "effect",
    state = { hits = "u8" },
    update = dimming_chips.phases {
        name = "EvilCut",
        effect = function(me: Object)
            local s = me.state :: CountState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                s.hits = 0
                me.phase_init = 4
            end
            if me.timer % 10 == 4 and s.hits < 3 then
                s.hits += 1
                local tx, ty = enemy_target(me.alliance)
                local reg = if s.hits == 3 then regions.x else regions.wide
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = reg,
                    hit_effect = sparks.heavy,
                    target = collision.hits_navis,
                    self_type = collision.slash,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if s.hits == 3 then 3 else 1,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), BIG_SLASH, me.alliance)
                battle.play_sound(SWING_SOUND)
                battle.palette_flash(0, 6, true, true)
            end
            me.timer += 1
            if me.timer >= 40 then
                dimming_chips.done(me)
            end
        end,
    },
}

local evilcut: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 150,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = {
        { order = 11, sequence = { blades.stepswrd, blades.heroswrd, blades.stepcros } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(EVILCUT_KIND, user, spec, true)
    end,
}

-- =============================================================================
-- 13 & 14. 2xHero (0x153) & DeuxHero (0x15C):
--   2xHero   (4-chip): Slasher + CustSwrd + VarSwrd + ProtoMan V1-V4 (10-hit x 70!)
--   DeuxHero (3-chip): CustSwrd + VarSwrd + ProtoMan V1-V4 (8-hit x 70!)
-- =============================================================================
type HeroState = { hits: number, max_hits: number }

local HERO_COMBO_KIND = new.kind {
    id = "pa/2xhero-controller",
    pool = "effect",
    state = { hits = "u8", max_hits = "u8" },
    update = dimming_chips.phases {
        name = "2xHero",
        effect = function(me: Object)
            local s = me.state :: HeroState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                s.hits = 0
                me.phase_init = 4
            end
            if me.timer % 6 == 2 and s.hits < s.max_hits then
                s.hits += 1
                local tx, ty = enemy_target(me.alliance)
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.wide,
                    hit_effect = sparks.heavy,
                    target = collision.hits_navis,
                    self_type = collision.slash,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if s.hits == s.max_hits then 3 else 1,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), BIG_SLASH, me.alliance)
                battle.play_sound(SWING_SOUND)
            end
            me.timer += 1
            if me.timer >= s.max_hits * 6 + 12 then
                dimming_chips.done(me)
            end
        end,
    },
}

local function hero_combo_hook(max_hits: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(HERO_COMBO_KIND, user, spec, true)
        if o then
            (o.state :: HeroState).max_hits = max_hits
        end
        return o
    end
end

local hero_2x: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 70,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = {
        { order = 12, sequence = { blades.slasher, blades.custswrd, blades.varswrd, navis.protoman } },
        { order = 13, sequence = { blades.slasher, blades.custswrd, blades.varswrd, navis.protomnv2 } },
        { order = 14, sequence = { blades.slasher, blades.custswrd, blades.varswrd, navis.protomnv3 } },
        { order = 15, sequence = { blades.slasher, blades.custswrd, blades.varswrd, navis.protomnv4 } },
    },
    dimming = hero_combo_hook(10),
}

local deuxhero: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 70,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = {
        { order = 16, sequence = { blades.custswrd, blades.varswrd, navis.protoman } },
        { order = 17, sequence = { blades.custswrd, blades.varswrd, navis.protomnv2 } },
        { order = 18, sequence = { blades.custswrd, blades.varswrd, navis.protomnv3 } },
        { order = 19, sequence = { blades.custswrd, blades.varswrd, navis.protomnv4 } },
    },
    dimming = hero_combo_hook(8),
}

-- =============================================================================
-- 15. HyperRat (0x154): Ratton1 + Ratton2 + Ratton3 -> 500-dmg homing rat missile!
-- =============================================================================
type RatState = { step: number }

local HYPERRAT_KIND = new.kind {
    id = "pa/hyperrat-controller",
    pool = "effect",
    state = { step = "u8" },
    update = dimming_chips.phases {
        name = "HyperRat",
        effect = function(me: Object)
            local s = me.state :: RatState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                s.step = 0
                me.phase_init = 4
            end
            if me.timer == 2 then
                local tx, ty = enemy_target(me.alliance)
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), RATTON_FX)
            end
            if me.timer == 14 then
                local tx, ty = enemy_target(me.alliance)
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.plus,
                    hit_effect = sparks.blast,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 5,
                })
                battle.region_effects(tx, ty, regions.plus, me.alliance, BLAST_FX, 0x10_0000)
                battle.play_sound(BLAST_SOUND)
                battle.shake_camera(3, 18)
            end
            me.timer += 1
            if me.timer >= 38 then
                dimming_chips.done(me)
            end
        end,
    },
}

local hyperrat: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 500,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = {
        { order = 39, sequence = { magnums.ratton1, magnums.ratton2, magnums.ratton3 } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(HYPERRAT_KIND, user, spec, true)
    end,
}

-- =============================================================================
-- 16. EverCrse (0x155): CrsShld1 + CrsShld2 + CrsShld3 -> multi-hit CurseShield!
-- =============================================================================
local EVERCRSE_KIND = new.kind {
    id = "pa/evercrse-controller",
    pool = "effect",
    state = { hits = "u8" },
    update = dimming_chips.phases {
        name = "EverCrse",
        effect = function(me: Object)
            local s = me.state :: CountState
            if me.phase_init == 0 then
                battle.play_sound(SHIELD_SOUND)
                me.timer = 0
                s.hits = 0
                me.phase_init = 4
            end
            if me.timer % 8 == 4 and s.hits < 5 then
                s.hits += 1
                local tx, ty = enemy_target(me.alliance)
                local last = s.hits == 5
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = sparks.charged,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if last then 3 else 1,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), BURST_FX)
                battle.play_sound(SWORD_SOUND)
            end
            me.timer += 1
            if me.timer >= 50 then
                dimming_chips.done(me)
            end
        end,
    },
}

local evercrse: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = {
        { order = 40, sequence = { utilities.crsshld1, utilities.crsshld2, utilities.crsshld3 } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(EVERCRSE_KIND, user, spec, true)
    end,
}

-- =============================================================================
-- 17. GelRain (0x156): MetaGel1/2/3 -> 3x150 Aqua gel rain stealing front column!
-- =============================================================================
local GELRAIN_KIND = new.kind {
    id = "pa/gelrain-controller",
    pool = "effect",
    state = { hits = "u8" },
    update = dimming_chips.phases {
        name = "GelRain",
        effect = function(me: Object)
            local s = me.state :: CountState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                s.hits = 0
                me.phase_init = 4
            end
            if me.timer % 10 == 4 and s.hits < 3 then
                s.hits += 1
                local col = enemy_front_col(me.alliance)
                local last = s.hits == 3
                dim_hitbox(me, {
                    panel_x = col,
                    panel_y = 2,
                    element = 2,
                    z = 0x10_0000,
                    region = regions.wide,
                    hit_effect = sparks.aqua,
                    target = collision.hits_navis,
                    self_type = collision.thrown,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if last then 3 else 1,
                })
                battle.region_effects(col, 2, regions.wide, me.alliance, BUBBLE_SPLASH_FX, 0x10_0000)
                battle.play_sound(GRAB_LAND_SOUND)
                if last then
                    local back_col = bit32.bxor(me.alliance, 1) * 5 + 1
                    if col ~= back_col then
                        for y = 1, 3 do
                            local p = field.panel(col, y)
                            if p and p.alliance ~= me.alliance and not field.check(col, y, 0x0060_0000, 0) then
                                field.set_alliance(col, y, me.alliance)
                            end
                        end
                    end
                end
            end
            me.timer += 1
            if me.timer >= 42 then
                dimming_chips.done(me)
            end
        end,
    },
}

local gelrain: Chip = {
    codes = PA_CODES,
    element = "aqua",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 150,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = {
        { order = 20, code_run = { chip = metagels.metagel1, count = 3 } },
        { order = 21, code_run = { chip = metagels.metagel2, count = 3 } },
        { order = 22, code_run = { chip = metagels.metagel3, count = 3 } },
        { order = 23, sequence = { metagels.metagel1, metagels.metagel2, metagels.metagel3 } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(GELRAIN_KIND, user, spec, true)
    end,
}

-- =============================================================================
-- 18. PoisPhar (0x157): PoisMask + PoisFace + Anubis (Pharaoh rapid poison drain!)
-- =============================================================================
type PharState = { life: number, tick: number }

local PHAR_STATUE = new.kind {
    id = "pa/poisphar-statue",
    pool = "attack",
    state = { life = "u16", tick = "u16" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            if battle.over() or not field.valid(me.panel_x, me.panel_y) then
                me:free()
                return
            end
            me.visible = true
            local sprite = me.sprite
            sprite:load(asset.sprite("anubis"))
            sprite.shadow = "with_sprite"
            me.anim, me.anim_loaded = 0, 0xFF
            me:set_coordinates_from_panel()
            local s = me.state :: PharState
            s.life = 540
            s.tick = 0
            battle.play_sound(APPEAR_SOUND)
            battle.effect(me.pos, PUFF_FX)
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: PharState
            s.life -= 1
            if s.life <= 0 then
                battle.effect(me.pos, PUFF_FX)
                me:set_lifecycle("destroy")
                return
            end
            if s.tick % 2 == 0 then
                local opp = battle.player(bit32.bxor(me.alliance, 1))
                if opp and not opp:status("dead") and opp.hp > 0 then
                    opp.hp = math.max(0, opp.hp - 2)
                    if s.tick % 20 == 0 then
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

local POISPHAR_CTRL = new.kind {
    id = "pa/poisphar-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "PoisPhar",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local user = me.related1
                if user then
                    local tx = user.panel_x + user.facing
                    local ty = user.panel_y
                    if field.valid(tx, ty) then
                        local o = battle.spawn(PHAR_STATUE)
                        if o then
                            o.panel_x, o.panel_y = tx, ty
                            o.alliance, o.flip = user.alliance, user.flip
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

local poisphar: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming" },
    program_advances = {
        { order = 24, sequence = { poisons.poismask, poisons.poisface, poisons.anubis } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(POISPHAR_CTRL, user, spec)
    end,
}

-- =============================================================================
-- 19. BodyGrd (0x158): AntiDmg + AntiNavi + Muramasa (12 diving shurikens x 100!)
-- =============================================================================
type StrikerState = { left: number, tick: number }

local BODYGRD_STRIKER = new.kind {
    id = "pa/bodygrd-striker",
    pool = "effect",
    state = { left = "u8", tick = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            local s = me.state :: StrikerState
            s.left = 12
            s.tick = 0
            me:set_lifecycle("update")
        elseif l == "update" then
            if battle.over() then
                me:set_lifecycle("destroy")
                return
            end
            local s = me.state :: StrikerState
            if s.tick == 0 and s.left > 0 then
                s.left -= 1
                s.tick = 16
                local tx, ty = shuriken.target_enemy(me)
                if tx and ty then
                    shuriken.spawn(me, tx, ty, 0, if s.left == 0 then 3 else 1, me.damage)
                end
            else
                s.tick -= 1
            end
            if s.left == 0 and s.tick == 0 then
                me:set_lifecycle("destroy")
            end
        else
            me:destroy()
        end
    end,
}

local BODYGRD_CTRL = new.kind {
    id = "pa/bodygrd-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "BodyGrd",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local o = battle.spawn(BODYGRD_STRIKER)
                if o then
                    o.alliance, o.flip = me.alliance, me.flip
                    o:set_damage_word(me.damage)
                end
                battle.play_sound(APPEAR_SOUND)
                me.timer = 18
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

local bodygrd: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = {
        { order = 25, sequence = { antis.antidmg, antis.antinavi, blades.muramasa } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(BODYGRD_CTRL, user, spec, true)
    end,
}

-- =============================================================================
-- 20. 500Barr (0x159): Barrier + Barr100 + Barr200 -> raises 500-HP barrier (ID 4)!
-- =============================================================================
local BARR500_KIND = new.kind {
    id = "pa/500barr-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "500Barr",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local user = me.related1 or battle.player(me.alliance)
                if user then
                    user:raise_barrier {
                        id = 4,
                        behavior = "plain",
                        hp = 500,
                        threshold = 0,
                        timer = 0xFFFF,
                        weak_element = 0,
                    }
                    local old = user.barrier_visual
                    if old then
                        common_visual.finish(old)
                    end
                    barrier_visual.spawn(user, barriers.barrier_200)
                    battle.play_sound(BARRIER_SOUND)
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

local barr_500: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming" },
    program_advances = {
        { order = 26, sequence = { barrier_chips.barrier, barrier_chips.barr100, barrier_chips.barr200 } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(BARR500_KIND, user, spec, true)
    end,
}

-- =============================================================================
-- 21. BigHeart (0x15A): HolyPanl + Recov300 + Roll -> full heal + 6x50 Roll attack!
-- =============================================================================
type BigHeartState = { hits: number, healed: boolean }

local BIGHEART_KIND = new.kind {
    id = "pa/bigheart-controller",
    pool = "effect",
    state = { hits = "u8", healed = "bool" },
    update = dimming_chips.phases {
        name = "BigHeart",
        effect = function(me: Object)
            local s = me.state :: BigHeartState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                s.hits = 0
                s.healed = false
                me.phase_init = 4
            end
            if me.timer == 6 and not s.healed then
                s.healed = true
                local user = me.related1 or battle.player(me.alliance)
                if user and not user:status("dead") then
                    user.hp = user.max_hp
                    battle.effect(user.pos, HEAL_FX)
                    battle.play_sound(RECOVER_SOUND)
                end
            end
            if me.timer >= 14 and me.timer % 6 == 2 and s.hits < 6 then
                s.hits += 1
                local tx, ty = enemy_target(me.alliance)
                local last = s.hits == 6
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = sparks.charged,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if last then 3 else 1,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), FLASH_FX)
                battle.play_sound(SPREADER_SOUND)
            end
            me.timer += 1
            if me.timer >= 56 then
                dimming_chips.done(me)
            end
        end,
    },
}

local bigheart: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 50,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = {
        { order = 27, sequence = { stages.holypanl, recovs.recov300, navis.roll } },
        { order = 28, sequence = { stages.holypanl, recovs.recov300, navis.rollv2 } },
        { order = 29, sequence = { stages.holypanl, recovs.recov300, navis.rollv3 } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(BIGHEART_KIND, user, spec, true)
    end,
}

-- =============================================================================
-- 22. GtsShoot (0x15B): Guard + DashAtk + GutsMan -> 300-dmg breaking row charge!
-- =============================================================================
local GTSSHOOT_KIND = new.kind {
    id = "pa/gtsshoot-controller",
    pool = "effect",
    state = { hits = "u8" },
    update = dimming_chips.phases {
        name = "GtsShoot",
        effect = function(me: Object)
            local s = me.state :: CountState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                s.hits = 0
                me.phase_init = 4
            end
            if me.timer % 4 == 2 and s.hits < 6 then
                local x = if me.alliance == 0 then (1 + s.hits) else (6 - s.hits)
                s.hits += 1
                if field.valid(x, me.panel_y) then
                    dim_hitbox(me, {
                        panel_x = x,
                        panel_y = me.panel_y,
                        element = 0,
                        z = 0x10_0000,
                        region = regions.single,
                        hit_effect = sparks.breaking,
                        target = collision.hits_navis,
                        self_type = collision.breaking,
                        damage = me.damage,
                        stamina = me.stamina,
                        hit_mod = 5,
                    })
                    local cx, cy = field.center(x, me.panel_y)
                    battle.effect(Vec3.new(cx, cy, 0x10_0000), IMPACT_FX)
                    battle.play_sound(THROW_SOUND)
                end
            end
            me.timer += 1
            if me.timer >= 36 then
                dimming_chips.done(me)
            end
        end,
    },
}

local gtsshoot: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 300,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = {
        { order = 41, sequence = { traps.guard, punches.dashatk, navis.gutsman } },
        { order = 42, sequence = { traps.guard, punches.dashatk, navis.gutsmanv2 } },
        { order = 43, sequence = { traps.guard, punches.dashatk, navis.gutsmanv3 } },
        { order = 44, sequence = { traps.guard, punches.dashatk, navis.gutsmanv4 } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(GTSSHOOT_KIND, user, spec)
    end,
}

-- =============================================================================
-- 23. MomQuake (0x15D): RockCube + RockCube + GodStone -> 3x200 breaking quake!
-- =============================================================================
local MOMQUAKE_KIND = new.kind {
    id = "pa/momquake-controller",
    pool = "effect",
    state = { hits = "u8" },
    update = dimming_chips.phases {
        name = "MomQuake",
        effect = function(me: Object)
            local s = me.state :: CountState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                s.hits = 0
                me.phase_init = 4
            end
            if me.timer % 12 == 6 and s.hits < 3 then
                s.hits += 1
                local region = regions.other_area[me.alliance]
                local last = s.hits == 3
                dim_hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = 0,
                    z = 0x10_0000,
                    region = region,
                    hit_effect = sparks.breaking,
                    target = collision.hits_navis,
                    self_type = collision.breaking,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if last then 5 else 1,
                })
                if last then
                    local opp_side = bit32.bxor(me.alliance, 1)
                    for x = 1, 6 do
                        for y = 1, 3 do
                            local p = field.panel(x, y)
                            if p and p.alliance == opp_side then
                                field.crack(x, y)
                            end
                        end
                    end
                end
                battle.region_effects(me.panel_x, me.panel_y, region, me.alliance, GROUND_CRACK_FX, 0x08_0000)
                battle.play_sound(CRACK_SOUND)
                battle.play_sound(THROW_SOUND)
                battle.shake_camera(3, 10)
            end
            me.timer += 1
            if me.timer >= 48 then
                dimming_chips.done(me)
            end
        end,
    },
}

local momquake: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 200,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = {
        { order = 30, sequence = { rockcubes.rockcube, rockcubes.rockcube, mega.godstone } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(MOMQUAKE_KIND, user, spec, true)
    end,
}

-- =============================================================================
-- 24. PrixPowr (0x15E): Team1 + Team2 + KingMan/MistMan/BowlMan -> 300-dmg N1 combo!
-- =============================================================================
local PRIXPOWR_KIND = new.kind {
    id = "pa/prixpowr-controller",
    pool = "effect",
    state = { hits = "u8" },
    update = dimming_chips.phases {
        name = "PrixPowr",
        effect = function(me: Object)
            local s = me.state :: CountState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                s.hits = 0
                me.phase_init = 4
            end
            if me.timer % 12 == 6 and s.hits < 3 then
                s.hits += 1
                local tx, ty = enemy_target(me.alliance)
                local reg = if s.hits == 3 then regions.other_area[me.alliance] else regions.plus
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = reg,
                    hit_effect = sparks.breaking,
                    target = collision.hits_navis,
                    self_type = collision.piercing_break,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if s.hits == 3 then 5 else 1,
                })
                battle.region_effects(tx, ty, reg, me.alliance, BLAST_FX, 0x10_0000)
                battle.play_sound(BLAST_SOUND)
                battle.shake_camera(3, 12)
            end
            me.timer += 1
            if me.timer >= 48 then
                dimming_chips.done(me)
            end
        end,
    },
}

local prixpowr: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 300,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = {
        { order = 45, sequence = { utilities.team1, utilities.team2, navis.kingman } },
        { order = 46, sequence = { utilities.team1, utilities.team2, navis.kingmanv2 } },
        { order = 47, sequence = { utilities.team1, utilities.team2, navis.kingmanv3 } },
        { order = 48, sequence = { utilities.team1, utilities.team2, navis.kingmanv4 } },
        { order = 49, sequence = { utilities.team1, utilities.team2, navis.kingmanv5 } },
        { order = 50, sequence = { utilities.team1, utilities.team2, navis.mistmanv5 } },
        { order = 51, sequence = { utilities.team1, utilities.team2, navis.bowlmanv5 } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(PRIXPOWR_KIND, user, spec, true)
    end,
}

-- =============================================================================
-- 25. MstrStyl (0x15F): Salamndr + Fountain + Bolt + GaiaBlad (6x100 Master Style!)
-- =============================================================================
local MSTR_ELEMS = { 1, 2, 3, 4, 0, 0 }
local MSTR_SPARKS = { sparks.fire, sparks.aqua, sparks.elec, sparks.wood, sparks.breaking, sparks.heavy }
local MSTR_FX = { HEAT_BLAST_FX, BUBBLE_SPLASH_FX, LIGHTNING_FX, WOOD_VINES_FX, BREAK_FX, BIG_SLASH }

local MSTRSTYL_KIND = new.kind {
    id = "pa/mstrstyl-controller",
    pool = "effect",
    state = { hits = "u8" },
    update = dimming_chips.phases {
        name = "MstrStyl",
        effect = function(me: Object)
            local s = me.state :: CountState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                battle.palette_flash(1, 14, true, true)
                me.timer = 0
                s.hits = 0
                me.phase_init = 4
            end
            if me.timer % 8 == 4 and s.hits < 6 then
                s.hits += 1
                local idx = s.hits
                local tx, ty = enemy_target(me.alliance)
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = MSTR_ELEMS[idx],
                    z = 0x10_0000,
                    region = regions.wide_long,
                    hit_effect = MSTR_SPARKS[idx],
                    target = collision.hits_navis,
                    self_type = collision.slash,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if idx == 6 then 5 else 1,
                })
                battle.region_effects(tx, ty, regions.wide, me.alliance, MSTR_FX[idx], 0x10_0000)
                battle.play_sound(if idx == 6 then SWING_SOUND else SWORD_SOUND)
            end
            me.timer += 1
            if me.timer >= 60 then
                dimming_chips.done(me)
            end
        end,
    },
}

local mstrstyl: Chip = {
    codes = PA_CODES,
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 99,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = {
        { order = 31, sequence = { mega.salamndr, mega.fountain, mega.bolt, mega.gaiablad } },
    },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(MSTRSTYL_KIND, user, spec, true)
    end,
}

-- =============================================================================
-- 26. BassGS (0x138): Vanishing World — 700-dmg dim Giga blast across enemy area!
-- =============================================================================
local BASSGS_KIND = new.kind {
    id = "pa/bassgs-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "BassGS",
        effect = function(me: Object)
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                battle.palette_flash(0, 14, true, true)
                me.timer = 0
                me.phase_init = 4
            end
            if me.timer == 16 then
                local region = regions.other_area[me.alliance]
                dim_hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = 0,
                    z = 0x10_0000,
                    region = region,
                    hit_effect = sparks.blast,
                    target = collision.hits_navis,
                    self_type = collision.piercing_break,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 5,
                })
                battle.region_effects(me.panel_x, me.panel_y, region, me.alliance, BLAST_FX, 0x10_0000)
                battle.play_sound(BLAST_SOUND)
                battle.play_sound(CRACK_SOUND)
                battle.shake_camera(3, 24)
                battle.palette_flash(1, 12, true, true)
            end
            me.timer += 1
            if me.timer >= 46 then
                dimming_chips.done(me)
            end
        end,
    },
}

local bassgs: Chip = {
    codes = { "X" },
    element = "null",
    family = "break",
    class = "giga",
    rarity = 4,
    mb = 99,
    damage = 700,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(BASSGS_KIND, user, spec, true)
    end,
}

return {
    ["z-canon1"] = z_canon1,
    ["z-canon2"] = z_canon2,
    ["z-canon3"] = z_canon3,
    ["z-punch"] = z_punch,
    ["z-strght"] = z_strght,
    ["z-impact"] = z_impact,
    ["z-varibl"] = z_varibl,
    ["z-yoyo1"] = z_yoyo1,
    ["z-yoyo2"] = z_yoyo2,
    ["z-yoyo3"] = z_yoyo3,
    ["z-step1"] = z_step1,
    ["z-step2"] = z_step2,
    timebom = timebom,
    ["h-burst"] = h_burst,
    bubsprd = bubsprd,
    heatsprd = heatsprd,
    lifeswrd = lifeswrd,
    elemswrd = elemswrd,
    evilcut = evilcut,
    ["2xhero"] = hero_2x,
    hyperrat = hyperrat,
    evercrse = evercrse,
    gelrain = gelrain,
    poisphar = poisphar,
    bodygrd = bodygrd,
    ["500barr"] = barr_500,
    bigheart = bigheart,
    gtsshoot = gtsshoot,
    deuxhero = deuxhero,
    momquake = momquake,
    prixpowr = prixpowr,
    mstrstyl = mstrstyl,
    bassgs = bassgs,
}
