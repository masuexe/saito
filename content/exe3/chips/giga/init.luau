--!strict
-- EXE3 Version-Exclusive Giga Chips (10 chips):
--   White (`version = "white"`): navrcycl, bass, serenade, balance, alpharmsg (AlphArmΣ)
--   Blue  (`version = "blue"`):  foldrbak, deltaray, darkaura, alpharmom (AlphArmΩ), bass-137 (Bass+)

local api = require("../rules/api")
local barrier_visual = require("../objects/barrier_visual")
local barriers = require("../lib/barriers")
local collision = require("../rules/collision")
local common_visual = require("@exelib/barriers/visual")
local dimming_chips = require("@exelib/dimming")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")

local BLAST_FX = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0, steps = 20 }
local BIG_SLASH_FX = new.effect { sprite = asset.sprite("big-slash"), anim = 0, palette = 0, steps = 15 }
local BURST_FX = new.effect { sprite = asset.sprite("burst"), anim = 0, palette = 0, steps = 14 }
local HEAL_FX = new.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0, steps = 16 }
local FLASH_FX = new.effect { sprite = asset.sprite("pink-flash"), anim = 1, palette = 0, steps = 10 }

local APPEAR_SOUND = asset.sound("appear")
local BLAST_SOUND = asset.sound("hit-bomb")
local CANNON_SOUND = asset.sound("cannon")
local SWING_SOUND = asset.sound("big-sword-swing")
local RECOVER_SOUND = asset.sound("sfx-recover")
local BARRIER_SOUND = asset.sound("barrier")
local CRACK_SOUND = asset.sound("panel-crack")

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
    return if side == 0 then 5 else 2, 2
end

-- =============================================================================
-- WHITE GIGA 1: NavRcycl (0x130) - replays last Navi/Mega chip (via rules chip_substitute);
-- if none used yet, plays a brief dimming fizzle cleanly.
-- =============================================================================
local NAVRCYCL_KIND = new.kind {
    id = "giga/navrcycl-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "NavRcycl",
        effect = function(me: Object)
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 16
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

-- =============================================================================
-- WHITE GIGA 2: Bass (0x132) - dim machine-gun barrage across opponent area!
-- =============================================================================
type VolleyState = { shots: number }

local BASS_KIND = new.kind {
    id = "giga/bass-controller",
    pool = "effect",
    state = { shots = "u8" },
    update = dimming_chips.phases {
        name = "Bass",
        effect = function(me: Object)
            local s = me.state :: VolleyState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                s.shots = 0
                me.phase_init = 4
            end
            if me.timer % 4 == 0 and s.shots < 10 then
                s.shots += 1
                local region = regions.other_area[me.alliance]
                local rx, ry = regions.random_panel(me.panel_x, me.panel_y, region, me.facing, 0, 0x8000)
                if rx == nil or ry == nil then
                    rx, ry = enemy_target(me.alliance)
                end
                local tx, ty = rx :: number, ry :: number
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.single,
                    hit_effect = sparks.blast,
                    target = collision.hits_navis,
                    self_type = collision.attack,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if s.shots == 10 then 3 else 1,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), FLASH_FX)
                battle.play_sound(CANNON_SOUND)
            end
            me.timer += 1
            if me.timer >= 52 then
                dimming_chips.done(me)
            end
        end,
    },
}

-- =============================================================================
-- WHITE GIGA 3: Serenade (0x133) - dim HolyShock column wave blast across enemy area!
-- =============================================================================
local SERENADE_KIND = new.kind {
    id = "giga/serenade-controller",
    pool = "effect",
    state = { shots = "u8" },
    update = dimming_chips.phases {
        name = "Serenade",
        effect = function(me: Object)
            local s = me.state :: VolleyState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                battle.palette_flash(1, 14, true, true)
                me.timer = 0
                s.shots = 0
                me.phase_init = 4
            end
            if me.timer % 8 == 4 and s.shots < 4 then
                s.shots += 1
                local region = regions.other_area[me.alliance]
                dim_hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = 0,
                    z = 0x10_0000,
                    region = region,
                    hit_effect = sparks.heavy,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if s.shots == 4 then 3 else 1,
                })
                battle.region_effects(me.panel_x, me.panel_y, region, me.alliance, BURST_FX, 0x10_0000)
                battle.play_sound(BLAST_SOUND)
                battle.shake_camera(2, 10)
            end
            me.timer += 1
            if me.timer >= 44 then
                dimming_chips.done(me)
            end
        end,
    },
}

-- =============================================================================
-- WHITE GIGA 4: Balance (0x134) - halves BOTH Navis' current HP (min 1)!
-- =============================================================================
local BALANCE_KIND = new.kind {
    id = "giga/balance-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "Balance",
        effect = function(me: Object)
            if me.phase_init == 0 then
                battle.palette_flash(0, 16, true, true)
                battle.play_sound(BLAST_SOUND)
                for side = 0, 1 do
                    local navi = battle.player(side)
                    if navi and not navi:status("dead") and navi.hp > 1 then
                        navi.hp = math.max(1, math.floor(navi.hp / 2))
                        battle.effect(navi.pos, BURST_FX)
                    end
                end
                me.timer = 28
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

-- =============================================================================
-- WHITE GIGA 5: AlphArmΣ (`alpharmsg`, 0x131) - dim sweeping multi-hit beam across enemy area!
-- =============================================================================
local ALPHARMSG_KIND = new.kind {
    id = "giga/alpharmsg-controller",
    pool = "effect",
    state = { shots = "u8" },
    update = dimming_chips.phases {
        name = "AlphArmΣ",
        effect = function(me: Object)
            local s = me.state :: VolleyState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                s.shots = 0
                me.phase_init = 4
            end
            if me.timer % 5 == 2 and s.shots < 8 then
                s.shots += 1
                local region = regions.other_area[me.alliance]
                dim_hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = 0,
                    z = 0x10_0000,
                    region = region,
                    hit_effect = sparks.blast,
                    target = collision.hits_navis,
                    self_type = collision.piercing,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = if s.shots == 8 then 3 else 1,
                })
                battle.region_effects(me.panel_x, me.panel_y, region, me.alliance, BLAST_FX, 0x10_0000)
                battle.play_sound(CANNON_SOUND)
                battle.shake_camera(2, 6)
            end
            me.timer += 1
            if me.timer >= 52 then
                dimming_chips.done(me)
            end
        end,
    },
}

-- =============================================================================
-- BLUE GIGA 1: FoldrBak (0x12F) - restores ALL 30 chips into battle folder + heals!
-- =============================================================================
local FOLDRBAK_KIND = new.kind {
    id = "giga/foldrbak-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "FoldrBak",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local side = me.alliance
                api.restore_folder(side)
                local user = me.related1 or battle.player(side)
                if user and not user:status("dead") then
                    user.hp = math.min(user.max_hp, user.hp + 200)
                    battle.effect(user.pos, HEAL_FX)
                end
                battle.palette_flash(1, 14, true, true)
                battle.play_sound(RECOVER_SOUND)
                me.timer = 28
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

-- =============================================================================
-- BLUE GIGA 2: DeltaRay (0x12E) - dim triple DeltaRayEdge slash around enemy!
-- =============================================================================
local DELTARAY_KIND = new.kind {
    id = "giga/deltaray-controller",
    pool = "effect",
    state = { shots = "u8" },
    update = dimming_chips.phases {
        name = "DeltaRay",
        effect = function(me: Object)
            local s = me.state :: VolleyState
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                s.shots = 0
                me.phase_init = 4
            end
            if me.timer % 12 == 6 and s.shots < 3 then
                s.shots += 1
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
                    hit_mod = if s.shots == 3 then 3 else 1,
                })
                local cx, cy = field.center(tx, ty)
                battle.effect(Vec3.new(cx, cy, 0x10_0000), BIG_SLASH_FX, me.alliance)
                battle.play_sound(SWING_SOUND)
                battle.palette_flash(1, 6, true, true)
            end
            me.timer += 1
            if me.timer >= 48 then
                dimming_chips.done(me)
            end
        end,
    },
}

-- =============================================================================
-- BLUE GIGA 3: DarkAura (0x135) - raises 300-threshold DarkAura barrier (ID 10)!
-- =============================================================================
local DARKAURA_KIND = new.kind {
    id = "giga/darkaura-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "DarkAura",
        effect = function(me: Object)
            if me.phase_init == 0 then
                local user = me.related1 or battle.player(me.alliance)
                if user then
                    user:raise_barrier {
                        id = 10,
                        behavior = "plain",
                        hp = 1,
                        threshold = 300,
                        timer = 1800,
                        weak_element = 0,
                    }
                    local old = user.barrier_visual
                    if old then
                        common_visual.finish(old)
                    end
                    barrier_visual.spawn(user, barriers.life_aura)
                    battle.play_sound(BARRIER_SOUND)
                end
                me.timer = 24
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

-- =============================================================================
-- BLUE GIGA 4: AlphArmΩ (`alpharmom`, 0x136) - dim massive rocket/claw barrage (500 dmg)!
-- =============================================================================
local ALPHARMOM_KIND = new.kind {
    id = "giga/alpharmom-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "AlphArmΩ",
        effect = function(me: Object)
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                me.phase_init = 4
            end
            if me.timer == 14 then
                local tx, ty = enemy_target(me.alliance)
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.square,
                    hit_effect = sparks.blast,
                    target = collision.hits_navis,
                    self_type = collision.breaking,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 5,
                })
                battle.region_effects(tx, ty, regions.square, me.alliance, BLAST_FX, 0x10_0000)
                battle.play_sound(BLAST_SOUND)
                battle.shake_camera(3, 20)
                battle.palette_flash(0, 12, true, true)
            end
            me.timer += 1
            if me.timer >= 42 then
                dimming_chips.done(me)
            end
        end,
    },
}

-- =============================================================================
-- BLUE GIGA 5: Bass+ (`bass-137`, 0x137) - dim EarthBreaker cracking center + 550 massive hit!
-- =============================================================================
local BASS_PLUS_KIND = new.kind {
    id = "giga/bass-plus-controller",
    pool = "effect",
    update = dimming_chips.phases {
        name = "Bass+",
        effect = function(me: Object)
            if me.phase_init == 0 then
                battle.play_sound(APPEAR_SOUND)
                me.timer = 0
                me.phase_init = 4
            end
            if me.timer == 16 then
                local tx, ty = enemy_target(me.alliance)
                dim_hitbox(me, {
                    panel_x = tx,
                    panel_y = ty,
                    element = 0,
                    z = 0x10_0000,
                    region = regions.plus,
                    hit_effect = sparks.blast,
                    target = collision.hits_navis,
                    self_type = collision.breaking,
                    damage = me.damage,
                    stamina = me.stamina,
                    hit_mod = 5,
                })
                -- EarthBreaker shatters center & surrounding panels!
                for _, d in { { 0, 0 }, { -1, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 } } do
                    local px, py = tx + d[1], ty + d[2]
                    if field.valid(px, py) then
                        field.break_panel(px, py)
                    end
                end
                battle.region_effects(tx, ty, regions.plus, me.alliance, BLAST_FX, 0x10_0000)
                battle.play_sound(BLAST_SOUND)
                battle.play_sound(CRACK_SOUND)
                battle.shake_camera(3, 22)
            end
            me.timer += 1
            if me.timer >= 44 then
                dimming_chips.done(me)
            end
        end,
    },
}

local function dim_hook(kind: Kind): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(kind, user, spec, true)
    end
end

-- White Version Giga Chips
local navrcycl: Chip = {
    version = "white",
    codes = { "*" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 50,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    dimming = dim_hook(NAVRCYCL_KIND),
}

local bass: Chip = {
    version = "white",
    codes = { "X" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 98,
    damage = 90,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = dim_hook(BASS_KIND),
}

local serenade: Chip = {
    version = "white",
    codes = { "S" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 97,
    damage = 100,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = dim_hook(SERENADE_KIND),
}

local balance: Chip = {
    version = "white",
    codes = { "Y" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 60,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    dimming = dim_hook(BALANCE_KIND),
}

local alpharmsg: Chip = {
    version = "white",
    codes = { "V" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 97,
    damage = 50,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = dim_hook(ALPHARMSG_KIND),
}

-- Blue Version Giga Chips
local foldrbak: Chip = {
    version = "blue",
    codes = { "*" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 99,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    dimming = dim_hook(FOLDRBAK_KIND),
}

local deltaray: Chip = {
    version = "blue",
    codes = { "Z" },
    element = "null",
    family = "sword",
    class = "giga",
    rarity = 4,
    mb = 92,
    damage = 220,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = dim_hook(DELTARAY_KIND),
}

local darkaura: Chip = {
    version = "blue",
    codes = { "A" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 55,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    dimming = dim_hook(DARKAURA_KIND),
}

local alpharmom: Chip = {
    version = "blue",
    codes = { "V" },
    element = "null",
    family = "break",
    class = "giga",
    rarity = 4,
    mb = 97,
    damage = 500,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = dim_hook(ALPHARMOM_KIND),
}

local bass_plus: Chip = {
    version = "blue",
    codes = { "X" },
    element = "null",
    family = "break",
    class = "giga",
    rarity = 4,
    mb = 98,
    damage = 550,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    dimming = dim_hook(BASS_PLUS_KIND),
}

return {
    navrcycl = navrcycl,
    bass = bass,
    serenade = serenade,
    balance = balance,
    alpharmsg = alpharmsg,
    foldrbak = foldrbak,
    deltaray = deltaray,
    darkaura = darkaura,
    alpharmom = alpharmom,
    ["bass-137"] = bass_plus,
}
