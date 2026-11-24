--!strict
-- The places of a player's auto battle data (EXE4's and EXE5's: a block of 0xE0 bytes a player, 42 halfword places
-- first), which a navi in auto battle plays: what the two games' code for them shares. A player's setup states the
-- places as their save holds them, and the consoles send them as a battle starts (`send`: EXE4's 0x0800F208, EXE5's
-- 0x0802C7BE, the same routine), after which the AI reads the first (`entry`), plays it and turns it last (`turn`),
-- or changes it for another (`swap`). Each game's rules keep the lists (their setup's and their state's
-- `auto_battle_places`) and hand them here (its rules/auto_battle/block).
--
-- A place (a halfword of the block) is empty (0xFFFF: `{}`), a chip, a pattern record by its number from 1 (bit 15
-- with its index), or `zero` (the halfword 0: chip 0, which no save the games wrote holds; the send keeps it as an
-- entry). A list past its last place: empty places.

local order = require("@exelib/order")

local places = {}

-- The block's places, how many of them (the first) the send shuffles apart from the rest, and its pattern records.
places.PLACES = 42
places.FIRST = 3
places.RECORDS = 8

-- A place as a setup and a state hold it.
local PLACE: StateSchema = { chip = "chip", pattern = "u8", zero = "bool" }
places.PLACE = PLACE
export type Place = { chip: Chip?, pattern: number, zero: boolean }

-- The lists the games write the places in (EXE5's 0x0802C692 to 0x0802C6E6): their names, first places (from 1) and
-- sizes, the chip `class` the list accepts (`class` or `own_class` when it matches the list's name; a tool reads it to
-- offer a place the chips it can hold), `unique` where a chip can appear at most once in the list (the standard, mega,
-- giga and program advance lists hold distinct most-used chips), and `weights` for the standard list's GBA repeat
-- table (4, 4, 2, 2, then 1×12 = 24 places across 16 logical slots). `first` (places 1..3) cannot be written in normal
-- gameplay and must remain empty.
local STANDARD_WEIGHTS: { number } = { 4, 4, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1 }
local LISTS: { { name: string, start: number, len: number, class: string?, own_class: boolean?, unique: boolean?, weights: { number }? } } = {
    { name = "first", start = 1, len = 3, class = "standard" },
    { name = "standard", start = 4, len = 24, class = "standard", own_class = true, unique = true, weights = STANDARD_WEIGHTS },
    { name = "mega", start = 28, len = 5, class = "mega", own_class = true, unique = true },
    { name = "giga", start = 33, len = 1, class = "giga", own_class = true, unique = true },
    { name = "patterns", start = 34, len = 8 },
    { name = "program_advance", start = 42, len = 1, class = "program_advance", own_class = true, unique = true },
}
places.LISTS = LISTS

local function has(list: any, x: any): boolean
    return list ~= nil and table.find(list, x) ~= nil
end

local function obtainable(chip: Chip): boolean
    if chip.class == "program_advance" then
        return true
    end
    return has(chip.flags, "library") or has(chip.flags, "dark") or (chip :: any).collectible == true
end

local function list_at(i: number)
    local list = LISTS[#LISTS]
    for _, l in LISTS do
        if i >= l.start and i < l.start + l.len then
            list = l
        end
    end
    return list
end

local function empty(p: Place): boolean
    return p.chip == nil and p.pattern == 0 and not p.zero
end

-- A place copied out of a setup or a state.
local function copy(p: Place): Place
    return { chip = p.chip, pattern = p.pattern, zero = p.zero }
end

local EMPTY: Place = { chip = nil, pattern = 0, zero = false }

-- The send, once for a round: both sides' places as their setups state them (`given`, side 0's first), as their
-- consoles send them as the battle starts. Each console shuffles its own player's 42 places with the same draws, from
-- the RNG2 the link shares (the battle's RNG here), so both sides' places get the same order: the first three
-- places shuffled over three rounds and the other 39 over 39, two draws a round, 84 draws in all, after which the
-- battle's RNG goes on from where the send left it. The entries are then packed to the front and counted to the first
-- empty place. (A recording states the places as the saves hold them and the RNG2 the send starts from.)
function places.send(given: { { Place } }): { { Place } }
    local lists: { { Place } } = {}
    for side, stated in given do
        local p: { Place } = {}
        for i = 1, places.PLACES do
            p[i] = if i <= #stated then copy(stated[i]) else EMPTY
        end
        lists[side] = p
    end
    local function shuffle(from: number, n: number, swaps: number)
        local new = battle.shuffle_order(n, swaps)
        for _, p in lists do
            order.apply(p, from - 1, new)
        end
    end
    shuffle(1, places.FIRST, places.FIRST)
    shuffle(places.FIRST + 1, places.PLACES - places.FIRST, places.PLACES - places.FIRST)
    local out: { { Place } } = {}
    for side, p in lists do
        local sent: { Place } = {}
        for _, place in p do
            if not empty(place) then
                table.insert(sent, place)
            end
        end
        out[side] = sent
    end
    return out
end

-- The entry in place `i` (from 1) of sent data: its chip, its pattern's number (from 1), false for a 0, nil for an
-- empty place or one past the count.
function places.entry(sent: { Place }, i: number): (Chip | number | false)?
    if i < 1 or i > #sent then
        return nil
    end
    local p = sent[i]
    if p.chip ~= nil then
        return p.chip
    elseif p.pattern > 0 then
        return p.pattern
    elseif p.zero then
        return false
    end
    return nil
end

-- The first entry and the one in place `i` (from 1) change places.
function places.swap(sent: { Place }, i: number)
    if i > #sent then
        return
    end
    local first, other = copy(sent[1]), copy(sent[i])
    sent[1] = other
    sent[i] = first
end

-- The first entry goes last, the rest move up (the games' packing of the entries but the emptied first; the count
-- stays): the list after it. With no entries nothing of the games leads here.
function places.turn(sent: { Place }): { Place }
    local n = #sent
    if n == 0 then
        error("a navi in auto battle turns auto battle data with no entries (the original packs and counts 2^32 of them)")
    end
    local first = copy(sent[1])
    local packed: { Place } = {}
    for i = 2, n do
        local p = copy(sent[i])
        if not empty(p) then
            table.insert(packed, p)
        end
    end
    while #packed < n - 1 do
        table.insert(packed, EMPTY)
    end
    table.insert(packed, first)
    return packed
end

-- A game's `validate`'s part for a setup's places: each place one thing (a chip, a pattern, a 0, or empty), a
-- pattern one of the eight records, `first` (places 1..3) empty (cannot be written in normal gameplay), no chip among
-- the 42 places that the game's AI can't play or that can't be obtained normally, each chip list held to its required
-- chip class, weighted lists (`standard`: 4, 4, 2, 2, 1×12) uniform within each weighted block, and `unique` lists
-- (standard/mega/giga/PA) holding each chip in at most one logical slot.
function places.validate(stated: { Place }, problem: (p: SetupProblem) -> ())
    local function place_at(i: number): Place
        return if i <= #stated then stated[i] else EMPTY
    end
    local function same_place(a: Place, b: Place): boolean
        return a.chip == b.chip and a.pattern == b.pattern and a.zero == b.zero
    end

    local counts: { [any]: { [Chip]: number } } = {}
    local bad_block_slots: { [number]: boolean } = {}
    for _, l in LISTS do
        if l.unique then
            counts[l] = {}
        end
        if l.weights ~= nil then
            local offset = l.start
            for _, w in l.weights do
                local first_p = place_at(offset)
                local consistent = true
                local seen_in_block: { [Chip]: boolean } = {}
                for k = 0, w - 1 do
                    local idx = offset + k
                    local p = place_at(idx)
                    if not same_place(p, first_p) then
                        consistent = false
                    end
                    if p.chip ~= nil then
                        seen_in_block[p.chip] = true
                    end
                end
                if not consistent then
                    for k = 0, w - 1 do
                        bad_block_slots[offset + k] = true
                    end
                end
                if l.unique then
                    local list_counts = counts[l]
                    for chip in seen_in_block do
                        list_counts[chip] = (list_counts[chip] or 0) + 1
                    end
                end
                offset += w
            end
        elseif l.unique then
            local list_counts = counts[l]
            for i = l.start, l.start + l.len - 1 do
                local chip: Chip? = place_at(i).chip
                if chip ~= nil then
                    list_counts[chip] = (list_counts[chip] or 0) + 1
                end
            end
        end
    end
    for i, p in stated do
        local list = list_at(i)
        local where = string.format("place %d of the auto battle data (`%s`, entry %d)", i, list.name, i - list.start + 1)
        if list.name == "first" and not empty(p) then
            problem({
                kind = "invalid",
                field = "auto_battle_places",
                entry = i,
                text = string.format("%s cannot be set in the original game", where),
            })
        end
        local things = (if p.chip ~= nil then 1 else 0) + (if p.pattern > 0 then 1 else 0) + (if p.zero then 1 else 0)
        if things > 1 then
            problem({
                kind = "invalid",
                field = "auto_battle_places",
                entry = i,
                text = string.format("%s is a chip, a pattern or a 0, not more", where),
            })
        end
        if p.pattern > places.RECORDS then
            problem({
                kind = "range",
                field = "auto_battle_places",
                entry = i,
                value = p.pattern,
                least = 0,
                most = places.RECORDS,
                text = string.format("place %d of the auto battle data names pattern %d; it has %d pattern records", i, p.pattern, places.RECORDS),
            })
        end
        if bad_block_slots[i] then
            problem({
                kind = "invalid",
                field = "auto_battle_places",
                entry = i,
                text = string.format("%s breaks the 4x-2x-1x slot rule of `standard`", where),
            })
        end
        local chip: Chip? = p.chip
        if chip ~= nil then
            local required = list.class or (if list.own_class then list.name else nil)
            -- (A chip with no `auto_battle`, the game's rules/auto_battle/place: its positioning class is past the
            -- table.)
            if (chip :: any).auto_battle == nil then
                problem({
                    kind = "chip_not_allowed",
                    field = "auto_battle_places",
                    entry = i,
                    chip = chip,
                    text = string.format(
                        "%s holds %s: the original can't play it in auto battle (its positioning class is past the game's table of them: the game crashes)",
                        where,
                        battle.name(chip)
                    ),
                })
            elseif chip.family == "special" or not obtainable(chip) then
                problem({
                    kind = "chip_not_allowed",
                    field = "auto_battle_places",
                    entry = i,
                    chip = chip,
                    text = string.format(
                        "%s holds %s: not an obtainable chip for auto battle data",
                        where,
                        battle.name(chip)
                    ),
                })
            elseif required ~= nil and chip.class ~= required then
                problem({
                    kind = "chip_not_allowed",
                    field = "auto_battle_places",
                    entry = i,
                    chip = chip,
                    text = string.format(
                        "%s holds %s, a chip of class `%s`: the list holds `%s` chips alone",
                        where,
                        battle.name(chip),
                        chip.class or "none",
                        required
                    ),
                })
            end
            local list_counts = counts[list]
            if list_counts ~= nil and list_counts[chip] > 1 then
                problem({
                    kind = "copies",
                    field = "auto_battle_places",
                    entry = i,
                    chip = chip,
                    used = list_counts[chip],
                    limit = 1,
                    text = string.format(
                        "%s holds %s again: `%s` holds each chip at most once",
                        where,
                        battle.name(chip),
                        list.name
                    ),
                })
            end
        end
    end
end

return places
