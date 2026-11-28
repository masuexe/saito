--!strict
-- EXE3 Barriers and Auras: Barrier, Barr100, Barr200, Aura, LifeAura.
-- Verified against A6BE Family 3 (0x080B117C, b11=0x02): all are instant non-dimming actions.

local barriers = require("../lib/barriers")

local barrier: Chip = {
    action = barriers.action("barrier/action", barriers.barrier_10),
    codes = { "C", "E", "L", "R", "S", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 7,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    auto_battle = { class = "when_unbarriered" },
}

local barr100: Chip = {
    action = barriers.action("barr100/action", barriers.barrier_100),
    codes = { "E", "J", "M", "R", "T", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 25,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    auto_battle = { class = "when_unbarriered" },
}

local barr200: Chip = {
    action = barriers.action("barr200/action", barriers.barrier_200),
    codes = { "E", "F", "H", "R", "U" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    auto_battle = { class = "when_unbarriered" },
}

local aura: Chip = {
    action = barriers.action("aura/action", barriers.aura),
    codes = { "F", "I", "S", "U", "Y" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 55,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    auto_battle = { class = "when_unbarriered" },
}

local lifeaura: Chip = {
    action = barriers.action("lifeaura/action", barriers.life_aura),
    codes = { "D" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 77,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "library" },
    auto_battle = { class = "when_unbarriered" },
}

return {
    barrier = barrier,
    barr100 = barr100,
    barr200 = barr200,
    aura = aura,
    lifeaura = lifeaura,
}
