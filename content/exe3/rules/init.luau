--!strict
-- EXE3's rules definition: Style Change setup, 5x5 NaviCust, 30-chip folder, Custom Screen ADD mode,
-- and all 6 Generation-1 rule sections.

local add = require("@self/add")
local folder = require("@self/folder")
local fresh_stats = require("@self/fresh_stats")
local legality = require("@self/legality")
local legality_check = require("@exelib/legality")
local navicust = require("@self/navicust")
local save = require("@self/save")
local styles = require("@self/styles")

require("@self/builds")
require("@self/rulesets")

local rules: Rules = {
    stats = {
        buster_mult = "u8",
        move_bug = "u8",
        bug_style = "bool",
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
    state = navicust.state,
    extends = {
        chip = {
            version = { "white", "blue" },
        },
    },
    buttons = add.buttons,
    custom = {
        hand_size = add.hand_size,
    },
    hooks = {
        round_setup = function(side: number)
            save.round_setup(side)
            styles.round_setup(side)
            navicust.run(side)
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
