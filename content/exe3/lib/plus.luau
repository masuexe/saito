--!strict
-- EXE3's plus chips (Atk+10, Atk+30) sparkle when used standalone.

local rising_bubble = require("../objects/rising_bubble")
local sparkle = require("@exelib/instant/sparkle")

local plus = {}

plus.sparkle = sparkle.make(rising_bubble.kind, asset.sound("panel-poison"))

function plus.spawner(_bonus: number): (user: Object, element: number, damage: number) -> ()
    return function(user: Object, _element: number, _damage: number)
        plus.sparkle(user)
    end
end

return plus
