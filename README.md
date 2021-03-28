# cls-minesweeper

**cls-minesweeper** is a simple clone of minesweeper designed to be played in the terminal. It uses [crossterm](https://github.com/crossterm-rs/crossterm) so in theory it should be compatible with most terminals. The program comes with a number of arguments that allow for control over the games generation, which can be viewed with *--help*.

### How to Play

Use the arrow keys or WASD to move the cursor around. Use Q to uncover, and E to flag. Use CTRL+Q or ESC to exit the game. All rules are otherwise the same as normal minesweeper!

### Features

- [x] Various difficulties and fine grain control
- [x] Compatibility across all modern operating systems (untested :p)
- [x] Fun colors
- [ ] Local high score record
- [ ] Leaderboards(?)
- [ ] Key remaping/configuration changing

No promises that any of this will ever get added, but no ones going to use this anyways, so it should be ok.

This is my first non-trivial project in rust, and my first project to be released to the wider world, so weird bugs are inevitable, and the code is fairly gross. Any and all feedback would be deeply appreciated.