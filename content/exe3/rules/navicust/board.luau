--!strict
-- EXE3's NaviCust 5x5 board (centered inside the engine's 7x7 grid), command line row 3, and 4-color bar.

local board = {}

board.SIZE = 7
board.COMMAND_LINE = 3
board.COLOR_BAR = 4

board.boards = {
    {
        ".......",
        ".ooooo.",
        ".ooooo.",
        ".ooooo.",
        ".ooooo.",
        ".ooooo.",
        ".......",
    },
}

return board
