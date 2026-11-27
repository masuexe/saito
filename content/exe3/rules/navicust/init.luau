--!strict
-- EXE3 NaviCust compiler: compiles placed programs on the 5x5 grid into MegaMan's NaviStats,
-- enforces command-line / plus-part / adjacency / color-bar / style-color bug rules, and validates placement.

local board = require("@self/board")
local programs = require("@self/programs")
local styles = require("./styles")

local HUBBATC = "hubbatc"
local SIZE: number = board.SIZE
local COMMAND_LINE: number = board.COMMAND_LINE
local COLOR_BAR: number = board.COLOR_BAR
local BOARD: { string } = board.boards[1]

export type Work = { hp: number, bug_stop: boolean }
type State = { bug_counts: { [string]: number } }
type Placed = { program: NaviCustProgram, color: string, x: number, y: number, rotation: number, compressed: boolean }

local function filled(rows: { string }, x: number, y: number): boolean
    return string.sub(rows[y + 1], x + 1, x + 1) == "#"
end

local function shape(def: NaviCustProgram, rotation: number, compressed: boolean): { { boolean } }
    local rows = if compressed and def.compressed ~= nil then def.compressed else def.shape
    local out: { { boolean } } = {}
    for y = 0, SIZE - 1 do
        out[y] = {}
        for x = 0, SIZE - 1 do
            out[y][x] = false
        end
    end
    local n = SIZE - 1
    for y = 0, n do
        for x = 0, n do
            if filled(rows, x, y) then
                local r = rotation % 4
                if r == 0 then
                    out[y][x] = true
                elseif r == 1 then
                    out[x][n - y] = true
                elseif r == 2 then
                    out[n - y][n - x] = true
                else
                    out[n - x][y] = true
                end
            end
        end
    end
    return out
end

local function cells(s: { { boolean } }, x: number, y: number): { { number } }
    local center = SIZE // 2
    local out: { { number } } = {}
    for j = 0, SIZE - 1 do
        for i = 0, SIZE - 1 do
            if s[j][i] then
                local cx, cy = x - center + i, y - center + j
                if cx >= 0 and cx < SIZE and cy >= 0 and cy < SIZE and string.sub(BOARD[cy + 1], cx + 1, cx + 1) == "o" then
                    table.insert(out, { cx, cy })
                end
            end
        end
    end
    return out
end

local function clamp_add(v: number, n: number, most: number): number
    return math.min(v + n, most)
end

local function apply(stats: Navi, e: NaviCustEffect, work: Work)
    local k = e.kind
    local n = e.amount or 0
    if k == "super_armor" then
        stats.super_armor = true
    elseif k == "custom" then
        stats.custom_level = clamp_add(stats.custom_level, n, 8)
    elseif k == "mega_folder" then
        stats.mega_level = clamp_add(stats.mega_level, n, 10)
    elseif k == "giga_folder" then
        stats.giga_level = clamp_add(stats.giga_level, n, 10)
    elseif k == "first_barrier" then
        stats.first_barrier = e.barrier
    elseif k == "back_special" then
        stats.back_special_weapon = e.weapon
    elseif k == "float_shoes" then
        stats.float_shoes = true
    elseif k == "air_shoes" then
        stats.air_shoes = true
    elseif k == "undershirt" then
        stats.undershirt = true
    elseif k == "fast_gauge" then
        stats.gauge = "fast"
    elseif k == "reg_up" then
        stats.regular_memory = clamp_add(stats.regular_memory, n, 50)
    elseif k == "attack" then
        stats.attack = clamp_add(stats.attack, n, 4)
    elseif k == "rapid" then
        stats.rapid = clamp_add(stats.rapid, n, 4)
    elseif k == "charge" then
        stats.charge = clamp_add(stats.charge, n, 4)
    elseif k == "attack_max" then
        stats.attack = 4
    elseif k == "rapid_max" then
        stats.rapid = 4
    elseif k == "charge_max" then
        stats.charge = 4
    elseif k == "bug_stop" then
        work.bug_stop = true
    elseif k == "rush" or k == "beat" or k == "tango" then
        stats.rush, stats.beat, stats.tango, stats.support_bug = k == "rush", k == "beat", k == "tango", false
    elseif k == "hp" then
        work.hp += n
    elseif k == "outside" then
        -- Non-battle effect
    else
        error(string.format("unknown EXE3 NaviCust effect kind %q", tostring(k)))
    end
end

local BUG_WORK: { [string]: (stats: Navi, level: number, work: Work) -> () } = {
    move = function(stats, level)
        stats.move_bug = ({ 0xFF, 0x10, 0x20 })[level]
    end,
    hp = function(stats, level)
        stats.hp_drain = ({ 10, 6, 3 })[level]
    end,
    custom = function(stats, level)
        stats.custom_drain = ({ 6, 4, 3 })[level]
    end,
    panel = function(stats, level)
        stats.panel_trail = ({ 3, 1, 4 })[level]
    end,
    buster = function(stats, level)
        stats.buster_blanks = ({ 4, 8, 10 })[level]
    end,
    support = function(stats, _level)
        stats.rush, stats.beat, stats.tango, stats.support_bug = false, false, false, true
    end,
    hub = function(stats, _level, work)
        stats.max_hp = bit32.band(math.max(bit32.rshift(stats.max_base_hp, 1), 1) + work.hp, 0xFFFF)
        stats.hp = math.min(stats.hp, stats.max_hp)
    end,
}

local function compile(side: number)
    local setup = rules.setup_of(side)
    local parts = setup.navicust_programs :: { Placed }
    local style_type: string = setup.style_type or "normal"
    local stats = battle.navi(side)
    if stats.navi.forms == nil then
        return
    end
    local function at(grid: { [number]: number }, x: number, y: number): number?
        return grid[y * SIZE + x]
    end
    local grid: { [number]: number } = {}
    for i, p in parts do
        for _, c in cells(shape(p.program, p.rotation, p.compressed), p.x, p.y) do
            grid[c[2] * SIZE + c[1]] = i
        end
    end

    local counts: { [string]: number } = {}
    local function count(bug: string?, n: number?)
        if bug ~= nil then
            counts[bug] = (counts[bug] or 0) + (n or 1)
        end
    end

    local line_programs: { [number]: number } = {}
    local line_plus: { [number]: number } = {}
    local on_line: { [number]: boolean } = {}
    for x = SIZE - 1, 0, -1 do
        local i = at(grid, x, COMMAND_LINE)
        if i ~= nil and not on_line[i] then
            on_line[i] = true
            local def = parts[i].program
            if def.plus then
                count(def.bug)
                line_plus[x] = i
            else
                line_programs[x] = i
            end
        end
    end

    local off_plus: { [number]: boolean } = {}
    for i, p in parts do
        if not on_line[i] then
            if p.program.plus then
                off_plus[i] = true
            else
                count(p.program.bug)
            end
        end
    end

    -- Same-color adjacency bug check.
    for i, p in parts do
        local met: { number } = {}
        local s = shape(p.program, p.rotation, p.compressed)
        for _, d in { { -1, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 } } do
            for _, c in cells(s, p.x + d[1], p.y + d[2]) do
                local q = at(grid, c[1], c[2])
                if q ~= nil and q ~= i and table.find(met, q) == nil then
                    table.insert(met, q)
                end
            end
        end
        for _, q in met do
            if parts[q].color == p.color then
                count(parts[q].program.bug)
            end
        end
    end

    -- HubBatc self-bug (halves base HP) and style color mismatch bug.
    for _, p in parts do
        if p.program.id == HUBBATC then
            count(p.program.bug)
        end
        if not styles.allowed_color(style_type, p.color) then
            count(p.program.bug or "move")
        end
    end

    -- 4-color bar check.
    local colors: { string } = {}
    for _, p in parts do
        if table.find(colors, p.color) == nil then
            table.insert(colors, p.color)
        end
    end
    if #colors == COLOR_BAR + 1 then
        count("move")
        count("custom")
    elseif #colors > COLOR_BAR + 1 then
        count("move", 2)
        count("custom", 2)
    end

    local work: Work = { hp = 0, bug_stop = false }
    local function run_program(def: NaviCustProgram)
        for _, e in def.effects or {} do
            apply(stats, e, work)
        end
    end

    local ran: { string } = {}
    for x = SIZE - 1, 0, -1 do
        local i = line_programs[x]
        if i ~= nil then
            local p = parts[i]
            local part = (p.program.id :: string) .. "/" .. p.color
            if table.find(ran, part) == nil then
                table.insert(ran, part)
                run_program(p.program)
            end
        end
    end
    for i, p in parts do
        if off_plus[i] then
            run_program(p.program)
        end
    end
    for x = 0, SIZE - 1 do
        local i = line_plus[x]
        if i ~= nil then
            run_program(parts[i].program)
        end
    end
    stats.max_hp = bit32.band(stats.max_base_hp + work.hp, 0xFFFF)
    stats.hp = stats.max_hp

    if work.bug_stop then
        counts = {}
    else
        for _, bug in programs.BUGS do
            local level = math.min(counts[bug] or 0, 3)
            if level > 0 then
                BUG_WORK[bug](stats, level, work)
            end
        end
    end

    local kept = (rules.state() :: State).bug_counts
    for _, bug in programs.BUGS do
        kept[bug] = math.min(counts[bug] or 0, 0xFF)
    end
end

local function validate(side: number, problem: (p: SetupProblem) -> ())
    local parts = rules.setup_of(side).navicust_programs :: { Placed }
    if #parts > 0 and battle.navi(side).navi.forms == nil then
        problem({
            kind = "not_taken",
            field = "navicust_programs",
            text = "a NaviCust, but only MegaMan compiles NaviCust programs",
        })
        return
    end
    local grid: { [number]: number } = {}
    for i, p in parts do
        local def = p.program
        local what = string.format("navicust program %d (%s)", i, battle.name(def))
        if table.find(def.colors, p.color) == nil then
            problem({ kind = "invalid", field = "navicust_programs", entry = i, text = string.format("%s: invalid color %s", what, p.color) })
        end
        if p.rotation > 3 then
            problem({
                kind = "range",
                field = "navicust_programs",
                entry = i,
                value = p.rotation,
                least = 0,
                most = 3,
                text = string.format("%s: rotation %d out of 0..3", what, p.rotation),
            })
        end
        local s = shape(def, p.rotation, p.compressed)
        local center = SIZE // 2
        local covered = 0
        for j = 0, SIZE - 1 do
            for k = 0, SIZE - 1 do
                if s[j][k] then
                    covered += 1
                end
            end
        end
        local on = cells(s, p.x, p.y)
        if #on ~= covered then
            problem({
                kind = "off_board",
                field = "navicust_programs",
                entry = i,
                text = string.format("%s at (%d, %d): off the board", what, p.x - center + 2, p.y - center + 2),
            })
        end
        for _, c in on do
            local at = c[2] * SIZE + c[1]
            if grid[at] ~= nil then
                problem({
                    kind = "overlap",
                    field = "navicust_programs",
                    entry = i,
                    other = grid[at],
                    text = string.format("%s overlaps program %d", what, grid[at]),
                })
                break
            end
            grid[at] = i
        end
    end
end

local NUMBERS: { [string]: boolean } = {
    hp = true,
    attack = true,
    rapid = true,
    charge = true,
    attack_max = true,
    rapid_max = true,
    charge_max = true,
    custom = true,
    mega_folder = true,
    giga_folder = true,
    reg_up = true,
    outside = true,
}

local function overview(side: number, out: SetupOverview)
    if battle.navi(side).navi.forms == nil then
        return
    end
    local parts = rules.setup_of(side).navicust_programs :: { Placed }
    local grid: { [number]: number } = {}
    for i, p in parts do
        for _, c in cells(shape(p.program, p.rotation, p.compressed), p.x, p.y) do
            grid[c[2] * SIZE + c[1]] = i
        end
    end
    local on_line: { [number]: boolean } = {}
    local at_work: { [number]: boolean } = {}
    local ran: { string } = {}
    for x = SIZE - 1, 0, -1 do
        local i = grid[COMMAND_LINE * SIZE + x]
        if i ~= nil and not on_line[i] then
            on_line[i] = true
            local p = parts[i]
            local part = (p.program.id :: string) .. "/" .. p.color
            if p.program.plus then
                at_work[i] = true
            elseif table.find(ran, part) == nil then
                table.insert(ran, part)
                at_work[i] = true
            end
        end
    end
    for i, p in parts do
        if at_work[i] or (not on_line[i] and p.program.plus) then
            for _, e in p.program.effects or {} do
                if not NUMBERS[e.kind] then
                    local source: SetupSource = { entry = p.program }
                    table.insert(out.abilities, source)
                    break
                end
            end
        end
    end
    local kept = (rules.state_of(side) :: State).bug_counts
    for _, bug in programs.BUGS do
        local level = math.min(kept[bug], 3)
        if level > 0 then
            local brought: SetupBug = { name = bug, level = level }
            table.insert(out.bugs, brought)
        end
    end
end

return {
    run = compile,
    validate = validate,
    overview = overview,
    state = {
        bug_counts = {
            move = "u8",
            hp = "u8",
            custom = "u8",
            panel = "u8",
            buster = "u8",
            support = "u8",
            hub = "u8",
        },
    } :: StateSchema,
}
