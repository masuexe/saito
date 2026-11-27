--!strict
-- Version-owned assets for EXE3 (White US `A6BE` vs Blue US `A3XE`):
-- - Version-exclusive Giga chips (`compat/chips.toml` `version` field):
--   White owns 304..=308 (`0x130..=0x134`: NavRcycl, AlphArmΣ, Bass, Serenade, Balance);
--   Blue owns 302, 303, 309..=311 (`0x12E, 0x12F, 0x135..=0x137`: DeltaRay, FoldrBak, DarkAura, AlphArmΩ, Bass+).

local names = require("@exelib/extract/names")

local versions = {}

versions.WHITE = "white"
versions.BLUE = "blue"

-- Each version's US ROM: White (`A6BE`, base) or Blue (`A3XE`).
function versions.rom(set: Set, version: string): Rom
    return if version == versions.BLUE then set.A3XE else set.A6BE
end

-- Version-exclusive chip map (`id -> "white" | "blue"`), from compat/chips.toml.
function versions.chips(): { [number]: string }
    local chips = data("compat/chips.toml")
    local out: { [number]: string } = {}
    for _, key in names.sorted(chips) do
        local v = chips[key].version
        if v ~= nil then
            if v ~= versions.WHITE and v ~= versions.BLUE then
                error(
                    string.format("chips.toml: %s's version is %q, not %s or %s", key, v, versions.WHITE, versions.BLUE),
                    0
                )
            end
            out[chips[key].id] = v
        end
    end
    return out
end

-- The ROM from which chip `id`'s icon and picture are read: its exclusive version's US ROM, or White US (`A6BE`).
function versions.chip_source(set: Set, chip_versions: { [number]: string }, id: number): Rom
    return versions.rom(set, chip_versions[id] or versions.WHITE)
end

return versions
