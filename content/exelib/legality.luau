--!strict
-- Shared build legality checks for items that exist in ROM data tables (so replays and `validate` still understand
-- them) but cannot be obtained in normal play (`unobtainable` in each game's `rules/legality.luau`).

export type LegalityGroup = {
    chip_codes: { [string]: { string } }?,
    navicust_colors: { [string]: { string } }?,
    navicust_programs: { string }?,
    facts: { string }?,
}

export type LegalitySpec = {
    unobtainable: LegalityGroup?,
    ds_only: LegalityGroup?,
}

local legality = {}

local function has(list: { string }?, x: string): boolean
    return list ~= nil and table.find(list, x) ~= nil
end

function legality.build_check(spec: LegalitySpec, setup: any, problem: (p: SetupProblem) -> ())
    local unobtainable = spec.unobtainable
    if unobtainable == nil then
        return
    end
    local chip_codes = unobtainable.chip_codes
    if chip_codes ~= nil and setup.folder ~= nil then
        for i, e in setup.folder do
            local chip: Chip?, code: string? = e.chip, e.code
            if chip ~= nil and code ~= nil then
                local id = chip.id :: string
                if has(chip_codes[id], code) then
                    problem({
                        kind = "unobtainable",
                        field = "folder",
                        entry = i,
                        chip = chip,
                        code = code,
                        text = string.format("folder entry %d: %s %s cannot be obtained in normal play", i - 1, id, code),
                    })
                end
            end
        end
    end
    local navicust_colors = unobtainable.navicust_colors
    local navicust_programs = unobtainable.navicust_programs
    if (navicust_colors ~= nil or navicust_programs ~= nil) and setup.navicust_programs ~= nil then
        for i, p in setup.navicust_programs do
            local def = p.program
            if def ~= nil then
                local id = def.id :: string
                local bad_prog = has(navicust_programs, id)
                local bad_color = navicust_colors ~= nil and has(navicust_colors[id], p.color)
                if bad_prog or bad_color then
                    problem({
                        kind = "unobtainable",
                        field = "navicust_programs",
                        entry = i,
                        def = def,
                        color = p.color,
                        text = string.format("navicust program %d: %s (%s) cannot be obtained in normal play", i, id, p.color),
                    })
                end
            end
        end
    end
end

return legality
