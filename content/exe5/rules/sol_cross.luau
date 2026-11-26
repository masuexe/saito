--!strict
-- EXE5DS Sol Cross MegaMan (Boktai 2/3 Slot-2 bonus, modeled as the 70 MB 5DS-only patch card `sol-cross-megaman`):
-- - Patch card `sol-cross-megaman` sets `hub_style = 3`, `max_hp += 200`, `super_armor = true`,
--   `mega_level = math.max(mega_level - 2, 1)`, and `charge_shot_weapon = sol_gundelsol.weapon` (0x6C).
-- - Spawns the corner `sol-sun` object at round start when fighting in sunlight (`stats.sun`).
-- - Mutually exclusive with `bass-cross-megaman` on the same build (even if custom rulesets raise the MB limit).

local sol_sun = require("../objects/sol_sun")

type Setup = {
    patch_cards: { InstalledPatchCard },
}

local sol_cross = {}

-- 0x02098B54: spawn the Sol Cross sun object at round start when fighting in sunlight.
function sol_cross.round_start(side: number)
    local stats = battle.navi(side)
    if stats.hub_style == 3 and stats.sun then
        sol_sun.spawn(side)
    end
end

-- `validate`: Sol-Cross MegaMan cannot be combined with Bass-Cross MegaMan on the same build.
function sol_cross.validate(side: number, problem: (p: SetupProblem) -> ())
    local setup = rules.setup_of(side) :: Setup
    local sol_entry: number? = nil
    local has_bass = false
    for i, installed in setup.patch_cards do
        if installed.card then
            if installed.card.id == "sol-cross-megaman" then
                sol_entry = i
            elseif installed.card.id == "bass-cross-megaman" then
                has_bass = true
            end
        end
    end
    if sol_entry ~= nil and has_bass then
        problem({
            kind = "invalid",
            field = "patch_cards",
            entry = sol_entry,
            text = "patch_cards: Sol-Cross MegaMan cannot be combined with Bass-Cross MegaMan",
        })
    end
end

return sol_cross
