--!strict
-- EXE3 folder rules: 30 chips, flat 4-copy Standard chip limit, 1 copy per Mega/Giga chip,
-- Mega/Giga folder limits from NaviStats (5/1 base + Team Style / MegFldr / GigFldr), Regular chip MB check, no tags.

local folder_rules = require("@exelib/folder_rules")

local COPIES: { [string]: number } = { standard = 4, mega = 1, giga = 1 }

local function has(list: any, x: any): boolean
    return list ~= nil and table.find(list, x) ~= nil
end

local RULES: FolderRules = {
    size = 30,
    in_pack = function(c: Chip): boolean
        return COPIES[c.class or ""] ~= nil
            and c.codes ~= nil
            and #c.codes > 0
            and has(c.flags, "library")
    end,
    copies = function(c: Chip): number
        return COPIES[c.class or ""] or 0
    end,
}

return {
    check = function(side: number): { SetupProblem }?
        return folder_rules.check(RULES, side, rules.setup_of(side))
    end,
    view = function(side: number): FolderView?
        return folder_rules.checked_view(RULES, side, rules.setup_of(side))
    end,
    validate = function(side: number, problem: (p: SetupProblem) -> ())
        folder_rules.validate(RULES, side, rules.setup_of(side), problem)
    end,
}
