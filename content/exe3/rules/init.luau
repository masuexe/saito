--!strict
-- EXE3's rules definition: Style Change setup, 5x5 NaviCust, 30-chip folder, Custom Screen ADD mode,
-- Bug Style per-turn rolls, Rush/Beat/Tango active supports, AntiDmg/Shadow guards, NavRcycl, and all Gen-1 sections.

local add = require("@self/add")
local api = require("@self/api")
local bug_style = require("@self/styles/bug_style")
local collision = require("@self/collision")
local folder = require("@self/folder")
local fresh_stats = require("@self/fresh_stats")
local guards = require("./navis/megaman/weapons/guards")
local legality = require("@self/legality")
local legality_check = require("@exelib/legality")
local navicust = require("@self/navicust")
local regions = require("@exelib/regions")
local save = require("@self/save")
local shuriken = require("./objects/shuriken")
local sparks = require("./lib/sparks")
local styles = require("@self/styles")
local supports = require("@self/navicust/supports")

require("@self/builds")
require("@self/rulesets")

local PUFF_FX = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0, steps = 12 }
local SWORD_FX = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0, steps = 12 }
local INVIS_SOUND = asset.sound("invisible")
local SWORD_SOUND = asset.sound("sword-swing")
local SLASH_BIT = 0x00002000

local state: StateSchema = {
    bug_counts = {
        move = "u8",
        hp = "u8",
        custom = "u8",
        panel = "u8",
        buster = "u8",
        support = "u8",
        hub = "u8",
    } :: StateSchema,
    bug_stopped = "bool",
    bug_base_atk = "u8",
    bug_base_rapid = "u8",
    bug_base_charge = "u8",
    bug_base_blanks = "u8",
    bug_base_move = "u8",
    bug_base_hp_drain = "u8",
    bug_base_custom_drain = "u8",
    bug_base_panel_trail = "u8",
    bug_hand_delta = "i8",
    rush_armed = "bool",
    beat_armed = "bool",
    tango_armed = "bool",
    rush_used = "bool",
    beat_used = "bool",
    tango_used = "bool",
    anti_dmg_stance = "u8",
    trap_kind = { "none", "anti_damage", "anti_sword", "anti_navi", "anti_recovery" },
    trap_damage = "u16",
    shadow_ticks = "u16",
    last_navi_chip = "chip",
}

local function has_flag(chip: Chip, flag: string): boolean
    local flags = chip.flags
    return flags ~= nil and table.find(flags, flag) ~= nil
end

local rules: Rules = {
    stats = {
        buster_mult = "u8",
        move_bug = "u8",
        bug_style = "bool",
        brake_charge = "bool",
        brake_buster = "bool",
        shadow_shoes = "bool",
        weapon_level = "u8",
        stage_panel = "u8",
    },
    setup = {
        navi = "navi",
        folder = schema.list({ chip = "chip", code = "code" }, 30),
        regular_chip = "u8?",
        hp = "u16",
        reg_up = "u8",
        style_type = { "normal", "guts", "custom", "team", "shield", "ground", "shadow", "bug" },
        style_element = { "null", "elec", "heat", "aqua", "wood" },
        navicust_programs = schema.list({
            program = "navicust_programs",
            color = { "white", "pink", "yellow", "red", "blue", "green", "orange", "purple", "dark" },
            x = "u8",
            y = "u8",
            rotation = "u8",
            compressed = "bool",
        }, 25),
    },
    setup_defaults = {
        hp = save.MAX_HP,
        reg_up = save.MAX_REG_UP,
        style_type = "normal",
        style_element = "null",
    },
    state = state,
    extends = {
        chip = {
            version = { "white", "blue" },
        },
    },
    buttons = add.buttons,
    custom = {
        hand_size = function(side: number): number?
            local base = add.hand_size(side) or 5
            local delta = bug_style.hand_delta(side)
            return math.clamp(base + delta, 3, 10)
        end,
    },
    hooks = {
        round_setup = function(side: number)
            save.round_setup(side)
            styles.round_setup(side)
            navicust.run(side)
        end,
        round_start = function(side: number)
            navicust.round_start(side)
            bug_style.roll_turn(side)
        end,
        custom_result = function(side: number)
            bug_style.roll_turn(side)
        end,
        fight_tick = function(side: number)
            local s = api.state(side)
            if s.anti_dmg_stance > 0 then
                s.anti_dmg_stance -= 1
            end
            if s.shadow_ticks > 0 then
                s.shadow_ticks -= 1
            end
            supports.fight_tick(side)
            -- ShdwShoe: never cracks or breaks panels beneath MegaMan.
            local stats = battle.navi(side)
            if stats.shadow_shoes then
                local navi = battle.player(side)
                if navi then
                    local p = field.panel(navi.panel_x, navi.panel_y)
                    if p and p.kind == "cracked" then
                        field.set_type(navi.panel_x, navi.panel_y, "normal")
                    end
                end
            end
        end,
        opponent_chip = function(side: number, opp_navi: Object): boolean
            if supports.opponent_chip(side, opp_navi) then
                return true
            end
            -- AntiNavi / AntiRecv trap checks!
            local s = api.state(side)
            local chip = opp_navi.attack_chip
            if chip ~= nil and s.trap_kind ~= "none" then
                if s.trap_kind == "anti_navi" and has_flag(chip, "navi") then
                    api.clear_trap(side)
                    battle.play_sound(asset.sound("trap-mark"))
                    battle.effect(opp_navi.pos, PUFF_FX)
                    return true
                elseif s.trap_kind == "anti_recovery" and chip.family == "recovery" then
                    api.clear_trap(side)
                    battle.play_sound(asset.sound("hit-bomb"))
                    local owner = battle.player(side)
                    if owner then
                        battle.hitbox(owner, {
                            panel_x = opp_navi.panel_x,
                            panel_y = opp_navi.panel_y,
                            element = 0,
                            z = 0x10_0000,
                            region = regions.single,
                            hit_effect = sparks.blast,
                            target = collision.hits_navis,
                            self_type = collision.piercing,
                            damage = 150,
                            hit_mod = 3,
                        })
                    end
                    return true
                end
            end
            return false
        end,
        low_hp = function(side: number, navi: Object): boolean
            return supports.low_hp(side, navi)
        end,
        hit_guard = function(side: number, navi: Object): boolean
            local c = navi.collision
            local dmg = c.final_damage
            if dmg <= 0 then
                return false
            end
            local s = api.state(side)

            -- 1. Shadow chip: immune to all non-sword hits!
            if s.shadow_ticks > 0 then
                local is_sword = bit32.btest(c.hit_flags, SLASH_BIT) or bit32.btest(c.damage_elements, 0x80)
                if not is_sword then
                    return true
                end
            end

            -- 2. AntiDmg B+Left stance (Shadow Style / AntiDmg NCP): swallow hit, turn invisible, throw shuriken!
            if s.anti_dmg_stance > 0 then
                guards.trigger_anti_dmg_counter(navi, 100)
                return true
            end

            -- 3. Armed AntiDmg / AntiSwrd defensive trap chip (from EXE3 rules state).
            if s.trap_kind == "anti_damage" then
                local counter_dmg = math.max(100, s.trap_damage)
                api.clear_trap(side)
                navi:set_status_timer("flash", 90)
                navi:set_status("invisible", true)
                battle.effect(navi.pos, PUFF_FX)
                battle.play_sound(INVIS_SOUND)
                local tx, ty = shuriken.target_enemy(navi)
                if tx and ty then
                    shuriken.spawn(navi, tx, ty, 0, 3, counter_dmg)
                end
                return true
            elseif s.trap_kind == "anti_sword" and (bit32.btest(c.hit_flags, SLASH_BIT) or bit32.btest(c.damage_elements, 0x80)) then
                local counter_dmg = math.max(100, s.trap_damage)
                api.clear_trap(side)
                battle.play_sound(SWORD_SOUND)
                for i = 0, 2 do
                    local tx, ty = shuriken.target_enemy(navi)
                    if tx and ty then
                        battle.hitbox(navi, {
                            panel_x = tx,
                            panel_y = ty,
                            element = 0,
                            z = 0x10_0000,
                            region = regions.wide,
                            hit_effect = sparks.heavy,
                            target = collision.hits_navis,
                            self_type = collision.slash,
                            damage = counter_dmg,
                            hit_mod = if i == 2 then 3 else 1,
                        })
                        local cx, cy = field.center(tx, ty)
                        battle.effect(Vec3.new(cx, cy, 0x10_0000), SWORD_FX, navi.alliance)
                    end
                end
                return true
            end

            return false
        end,
        navi_palette = styles.navi_palette,
        chip_substitute = function(side: number, _navi: Object, chip: Chip): Chip?
            if chip.id == "navrcycl" then
                local last = api.last_navi_chip(side)
                if last ~= nil then
                    return last
                end
            end
            return nil
        end,
        chip_used = function(side: number, _navi: Object, chip: Chip, _weapon: Weapon?)
            local cid = chip.id or ""
            if cid ~= "" and cid ~= "navrcycl" and (chip.class == "mega" or has_flag(chip, "navi")) then
                api.set_last_navi_chip(side, chip)
            end
        end,
        validate = function(side: number): { SetupProblem }
            local out: { SetupProblem } = {}
            local function problem(p: SetupProblem)
                table.insert(out, p)
            end
            save.validate(side, problem)
            navicust.validate(side, problem)
            folder.validate(side, problem)
            return out
        end,
        build_check = function(side: number): { SetupProblem }
            local out: { SetupProblem } = {}
            local function problem(p: SetupProblem)
                table.insert(out, p)
            end
            legality_check.build_check(legality, rules.setup_of(side), problem)
            return out
        end,
        folder_check = folder.check,
        folder_view = folder.view,
        overview = function(side: number): SetupOverview?
            local out: SetupOverview = { abilities = {}, bugs = {}, lists = {} }
            navicust.overview(side, out)
            return out
        end,
    },
    banners = require("@self/banners"),
    buster = require("@self/buster"),
    chip_use = require("@self/chip_use"),
    custom_screen = require("@self/custom_screen"),
    effects = require("@self/effects"),
    elements = require("@self/elements"),
    flow = require("@self/flow"),
    fresh_stats = fresh_stats,
    link_pick = require("@self/link_pick"),
    math = require("@self/math"),
    panels = require("@self/panels"),
    pools = require("@self/pools"),
    reactions = require("@self/reactions"),
    roles = require("@self/roles"),
    status = require("@self/status").rules,
}

return rules
