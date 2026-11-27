--!strict
-- EXE3 collision types (self and target flag words per side).

local collision = {}

collision.nothing = new.collision { side0 = 0x00000000, side1 = 0x00000000, row_offset = 0x0 }
collision.navi = new.collision { side0 = 0x08410000, side1 = 0x04210000, row_offset = 0x8 }
collision.navi_target = new.collision { side0 = 0x55800200, side1 = 0xaa800200, row_offset = 0x10 }
collision.attack = new.collision { side0 = 0x80000000, side1 = 0x40000000, row_offset = 0x20 }
collision.hits_navis = new.collision { side0 = 0x15800000, side1 = 0x2a800000, row_offset = 0x28 }
collision.breaking = new.collision { side0 = 0x80000002, side1 = 0x40000002, row_offset = 0x30 }
collision.slash = new.collision { side0 = 0x80002000, side1 = 0x40002000, row_offset = 0x38 }
collision.thrown = new.collision { side0 = 0x80000008, side1 = 0x40000008, row_offset = 0x50 }
collision.piercing = new.collision { side0 = 0x80000004, side1 = 0x40000004, row_offset = 0x58 }
collision.floating_navi = new.collision { side0 = 0x08510000, side1 = 0x04310000, row_offset = 0x80 }
collision.thrown_break = new.collision { side0 = 0x8000000a, side1 = 0x4000000a, row_offset = 0xa8 }
collision.piercing_break = new.collision { side0 = 0x80000006, side1 = 0x40000006, row_offset = 0xb0 }
collision.thrown_slash = new.collision { side0 = 0x80002008, side1 = 0x40002008, row_offset = 0xe0 }

return collision
