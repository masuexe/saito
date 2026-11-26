--!strict
-- EXE5DS Sol Cross MegaMan (Boktai 2 Slot-2 double-slot bonus: 0x0205B14E, 0x02098B54):
-- - Sets `hub_style = 3`, `max_hp += 200` (`hp = max_hp`), `super_armor = true`,
--   `mega_level = math.max(mega_level - 2, 1)`, and `charge_shot_weapon = sol_gundelsol.weapon` (0x6C).
-- - Spawns the corner `sol-sun` object at round start when fighting in sunlight (`stats.sun`).
-- - Requires MegaMan as the main navi and is mutually exclusive with the Bass-Cross MegaMan patch card.

local sol_gundelsol = require("../navis/megaman/weapons/sol_gundelsol")
local sol_sun = require("../objects/sol_sun")

type Setup = {
    navi: NaviDef?,
    sol_cross: boolean,
    patch_cards: { InstalledPatchCard },
}

local sol_cross = {}

-- 0x0205B14E: apply Sol Cross stats to MegaMan after NaviCust and patch cards.
function sol_cross.round_setup(side: number)
    local setup = rules.setup_of(side) :: Setup
    if not setup.sol_cross then
        return
    end
    local stats = battle.navi(side)
    if stats.navi.forms == nil then
        return
    end
    stats.hub_style = 3
    stats.max_hp = math.min(stats.max_hp + 200, 0xFFFF)
    stats.hp = stats.max_hp
    stats.super_armor = true
    stats.mega_level = math.max(stats.mega_level - 2, 1)
    stats.charge_shot_weapon = sol_gundelsol.weapon
end

-- 0x02098B54: spawn the Sol Cross sun object at round start when fighting in sunlight.
function sol_cross.round_start(side: number)
    local stats = battle.navi(side)
    if stats.hub_style == 3 and stats.sun then
        sol_sun.spawn(side)
    end
end

-- `validate`: Sol Cross is MegaMan-only and mutually exclusive with Bass-Cross MegaMan.
function sol_cross.validate(side: number, problem: (p: SetupProblem) -> ())
    local setup = rules.setup_of(side) :: Setup
    if not setup.sol_cross then
        return
    end
    local navi = setup.navi
    if navi == nil then
        return
    end
    if navi.forms == nil then
        problem({
            kind = "not_taken",
            field = "sol_cross",
            text = string.format("sol_cross: %s cannot use Sol Cross (MegaMan only)", battle.name(navi)),
        })
        return
    end
    for _, installed in setup.patch_cards do
        if installed.card and installed.card.id == "bass-cross-megaman" then
            problem({
                kind = "invalid",
                field = "sol_cross",
                text = "sol_cross: Sol Cross cannot be combined with Bass-Cross MegaMan",
            })
            break
        end
    end
end

return sol_cross
