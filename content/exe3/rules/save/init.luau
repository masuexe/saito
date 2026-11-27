--!strict
-- What an EXE3 save brings to MegaMan's stats: base HP (HP Memories: 100 + 20 * count, max 1000) and Regular Memory
-- (RegUps, max 50 MB).

type Setup = { hp: number, reg_up: number }

local MAX_HP = 1000
local MAX_REG_UP = 50

local function round_setup(side: number)
    local stats = battle.navi(side)
    local setup = rules.setup() :: Setup
    stats.regular_memory = setup.reg_up
    stats.max_base_hp = setup.hp
    stats.max_hp = setup.hp
    stats.hp = setup.hp
end

local function validate(side: number, problem: (p: SetupProblem) -> ())
    local setup = rules.setup_of(side)
    if setup.hp < 1 or setup.hp > MAX_HP then
        problem({
            kind = "range",
            field = "hp",
            value = setup.hp,
            least = 1,
            most = MAX_HP,
            text = string.format("hp %d: MegaMan's base HP is 1 to %d", setup.hp, MAX_HP),
        })
    end
    if setup.reg_up > MAX_REG_UP then
        problem({
            kind = "range",
            field = "reg_up",
            value = setup.reg_up,
            least = 0,
            most = MAX_REG_UP,
            text = string.format("reg_up %d: the Regular memory is %d at most", setup.reg_up, MAX_REG_UP),
        })
    end
end

return {
    round_setup = round_setup,
    validate = validate,
    MAX_HP = MAX_HP,
    MAX_REG_UP = MAX_REG_UP,
}
