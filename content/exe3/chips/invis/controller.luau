--!strict
-- EXE3 Invis dimming controller over @exelib/invisibl/controller.

local common = require("@exelib/invisibl/controller")

return common.make { id = "invis/controller", sound = asset.sound("invisible") }
