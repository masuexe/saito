--!strict
-- EXE3's 5 elements (null, heat/fire, aqua, elec, wood) and weakness cycle:
-- Heat > Wood > Elec > Aqua > Heat.

return {
    weakness = {
        null = { 0, 0, 0, 0, 0, 0 },
        fire = { 0, 0, 1, 0, 0, 0 },
        aqua = { 0, 0, 0, 1, 0, 0 },
        elec = { 0, 0, 0, 0, 1, 0 },
        wood = { 0, 1, 0, 0, 0, 0 },
        drain = { 0, 0, 0, 0, 0, 0 },
    },
    families = {
        fire = 0,
        aqua = 1,
        elec = 2,
        wood = 3,
        recovery = 4,
        plus = 5,
        sword = 6,
        invisible = 7,
        ["break"] = 8,
        summon = 9,
        wind = 10,
        metal = 11,
        null = 12,
        program_advance = 14,
        special = 15,
    },
    non_elemental = "null",
}
