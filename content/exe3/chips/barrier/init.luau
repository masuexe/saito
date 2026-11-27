--!strict
-- EXE3 Barriers and Auras: Barrier, Barr100, Barr200, Aura, LifeAura.

local barriers = require("../lib/barriers")

local barrier: Chip = {
    codes = { "C", "E", "L", "R", "S", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 7,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = barriers.controller.hook(barriers.barrier_10),
    auto_battle = { class = "when_unbarriered" },
}

local barr100: Chip = {
    codes = { "E", "J", "M", "R", "T", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 25,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = barriers.controller.hook(barriers.barrier_100),
    auto_battle = { class = "when_unbarriered" },
}

local barr200: Chip = {
    codes = { "E", "F", "H", "R", "U" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = barriers.controller.hook(barriers.barrier_200),
    auto_battle = { class = "when_unbarriered" },
}

local aura: Chip = {
    codes = { "F", "I", "S", "U", "Y" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 55,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = barriers.controller.hook(barriers.aura),
    auto_battle = { class = "when_unbarriered" },
}

local lifeaura: Chip = {
    codes = { "D" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 77,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    dimming = barriers.controller.hook(barriers.life_aura),
    auto_battle = { class = "when_unbarriered" },
}

return {
    barrier = barrier,
    barr100 = barr100,
    barr200 = barr200,
    aura = aura,
    lifeaura = lifeaura,
}
