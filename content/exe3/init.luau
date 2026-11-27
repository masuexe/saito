--!strict
-- EXE3's top module: exposes chips, navis, forms, stages, navicust_programs, rules, and extract.
-- EXE3 has no patch_cards (e-Reader Mod Cards were introduced in EXE4).

return {
    chips = require("@self/chips"),
    navis = require("@self/navis"),
    forms = require("@self/navis/megaman/forms"),
    stages = require("@self/stages"),
    navicust_programs = require("@self/navicust"),
    rules = require("@self/rules"),
    extract = require("@self/extract"),
}
