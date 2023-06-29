# mxyzptlk

An inform (zmachine) virtual machine implemented in Rust.

## Usage

I'm pretty sure a curses library (ncurses for Linux/MacOS, pdcurses for windows) needs to be installed. Will add instructions.  Short version: MacOS - use homebrew to install the `ncurses` formula;  Linux - depends on your package manager, look for `ncursesw` or `ncurses`;  Windows - google `pdcurses windows` and good luck!

### Building

```
$ cargo build
```
add the `--release` flag if you don't plan to debug anything:
```
$ cargo build --release
```

### Testing

The `zcode` directory contains several freely available test files.  I did not author these files and provide no guarantee of correctness.  I do wish to thank the authors, however, because these tests helped track down and squash several bugs resulting from my misinterpretation of the ZMachine standard.

* [TerpEtude](https://www.ifarchive.org/if-archive/infocom/interpreters/tools/etude.tar.Z) by Andrew Plotkin
    * Also available [here](https://github.com/townba/etude) with source code.  The `etude.z5` and `gntests.z5` files included are from this repo.
* [Czech](https://www.ifarchive.org/if-archive/infocom/interpreters/tools/czech_0_8.zip) by Amir Karger
* [Praxix](https://www.ifarchive.org/if-archive/infocom/interpreters/tools/praxix.zip) by Zarf & Dannii
* [Strict](https://www.ifarchive.org/if-archive/infocom/interpreters/tools/strictz.z5) by Torbjorn Andersson

These files have been included in this repo without express permission and will be removed upon request by the author.

To run a test from the repo root:
```
$ cargo run -- zcode/etude.z5
```

Some are interactive, like TerpEtude, and others just run a sequence of tests and output results.

### Running a game

From a curses-supported terminal window measuring 80 columns by 24 rows or larger.  Smaller should work, mostly.

* Running the compiled binary directly, assuming it's in the $PATH:
```
$ mxyzptlk filename.z5
```
* Via `cargo`:
```
$ cargo run -- filename.z5
```

Speaking of games, the [Interactive Fiction Archive](https://www.ifarchive.org/indexes/if-archive/) has quite a number of free games.  This interpreter is for "zcode" games only, generally those with names ending in ".z{version}".  Only `version`s 3, 4, 5, 7, and 8 are supported, though version 7 is somewhat rare and I haven't tried running one yet.

### Configuration

Add notes about config.yml

### Logs

Execution will dump quite a bit of output to various `.log` files.  This can be mostly disabled by changing the `level` values in the `log4rs.yml` file to `warn`. Or just ignore them.

# History
## June 28, 2023

More restructuring code and implementing better rust development practices.  The sound player is now behind a trait, which should make it easier to change if necessary.  The old "state" has been rechristened "zmachine" and decomposed into separate files for runtime state, screen i/o, and sound.  Files for instruction, object, and text were pulled out of the zmachine module to keep internals private.

### Fixed
* `PRINT_TABLE` no longer prints padding that overwrites other text inappropriately, which was very evident in Beyond Zork
* Stream 3 converts \n (0x0d) to \r (0x0a), per spec.  This was responsible for issues with the layout in Beyond Zork.

### Backlog
* Logging is a mess
* Refactor read/sound interrupt handling to a consistent implementation.
* Input streams
* SAVE and RESTORE data (V5+)
* Enable interpreter commands:
    * `!undo` to undo a move in games that don't support `SAVE_UNDO` and `RESTORE_UNDO`
    * up/down arrow keys to cycle through input history (if up/down aren't terminator characters for the `READ` instruction)
    * `!again` or `!g` to repeat last input, also for games that don't have a native `again` verb.
* Update curses terminals to gracefully(?) handle resizing the terminal window.
* Implement error handling as suggested by spec.

---
---

## June 24, 2023

Refactored a lot of the code to make it more readable and manageable.  I also rewrote the terminal implementation to use either easycurses or pancurses, which are mostly the same except pancurses exposes mouse click and location info.  Easycurses should be able to do this by accessing the underlying pancurses lib.

### Working
* Curses-based terminal interpreter (two!) with working zmachine screen model including color and font styling, though italic characters are underlined due to limitations in curses.  Mouse input is also working correctly in Beyond Zork.
* Output pauses when a full screen of text has been printed without input from the player.  Hitting `Enter` (or `Return`) will print just one more line, any other key prints up to a full page.
* Multiple undo states as with previous update
* Save/restore game state to/from Quetzal IFF files using compressed memory
* Transcripting
* Passes czech.z5 and praxis.z5 tests
* Everything works in etude.z5
* Suggest filenames `{story-file-basename}-{##}.{ext}` for save, restore, and script
* V3 sound (The Lurking Horror)
* V5 sound (Sherlock) ... provisionally.  The clock chime sounds 6 times as 6AM and the interrupt routine runs (and does nothing of consequence), but I haven't played far enough to trigger an interrupt that does anything interesting.
* STATUS_LINE will truncate the location name if the screen is too narrow to display full text.  For those who wax longingly for a Commodore VIC-20, maybe?

### Fixed!
* AREAD opcode correctly sets the text buffer positions of words, which fixed problems with setting the table style and handling puzzle pieces in jigsaw.z8

### Backlog
* Refactor read/sound interrupt handling so it less ... hacky
* Input streams
* SAVE and RESTORE data (V5+)
* Enable interpreter commands:
    * `!undo` to undo a move in games that don't support `SAVE_UNDO` and `RESTORE_UNDO`
    * up/down arrow keys to cycle through input history (if up/down aren't terminator characters for the `READ` instruction)
    * `!again` or `!g` to repeat last input, also for games that don't have a native `again` verb.
* Update curses terminals to gracefully(?) handle resizing the terminal window.

---
---

## March 11, 2023

### Working

* Curses-based terminal interpreter with working zmachine screen model support including color and font styling.  Color, however is inconsistent in different shells and on different platforms due to differences in the underlying curses libraries (I think).  "True color" is even more inconsistent, for good measure.
* Multiple undo states keeping the most recent 10 stored undo states.
* Save and restore Quetzal files using compressed memory format.  The interpreter will suggest an autonumbered save file (lurking-horror-01.ifzs), and on restore suggest the most recent (highest numbered) file it finds.
* Transcripting to auto-incrementing file names.
* Passes czech.z5, etude.z5, praxis.z5 test suites.
* Mouse input in Beyond Zork.
* Sound effects in The Lurking Horror with a suitable blorb resource file containing OGG sound data.

### Not working
* AIFF sound playback ... need to find a Rust lib for this.
* Infocom V6 games (Zork 0, Shogun, Journey, Arthur) - most V6 instructions will panic.
* Infocom V5 sound interrupts (Sherlock is the only V5 game with sound?)
* Input streams (reading commands from a file instead of keyboard)
* V5+ save/restore data ... not extensively used

### Bugs
* Beyond Zork windowed display has minor glitches where spaces appear where they shouldn't be.
* Jigsaw won't show the puzzle frame.
* Small screen sizes can cause problems with status line display

### Backlog
* V5 sound interrupts
* Fix handling of small (< 80 column) screens
* Debug Beyond Zork display glitches
* Debug Jigsaw weirdness
* Input streams
* Save/restore data

### Future
* Browser-based interpreter
* Native interpreters for MacOS/Linux/Windows

---
---

## Feb 19, 2023

### Why?

I want to learn Rust.  The zmachine is a well-documented virtual machine with implementations dating back to 1980 on platforms from the Apple ][ to the (Sinclar) ZX.  I've also worked on other implementations, first porting the [Frotz](https://www.ifwiki.org/Frotz) interpreter to the Apple //gs in the mid 90s and later authoring (unreleased) Java and Clojure implementations.  It seems to be my go-to when I don't have any better ideas.

### What

The general idea is to separate the virtual machine that executes code from the interpreter, which provides the user interface.  I have wild and unexplored ideas of native UX implementations, web implementations, etc.

It's going to be sloppy - I'm still acclimating to the peculiarities of Rust and figuring out what I can get from a library vs. what I have to write myself.

---
---

## History
0.0.1 - ... work in progress  
0.0.2 - 20230623 - refactor

