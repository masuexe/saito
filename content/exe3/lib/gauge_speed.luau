--!strict
-- EXE3's FstGauge and SloGauge dimming controller over @exelib/gauge_speed.

local common = require("@exelib/gauge_speed/controller")

return common.make { id = "gauge-speed", warning = asset.sound("program-advance-part") }
