# mxyzptlk

An inform (zmachine) virtual machine implemented in Rust.

## Usage

I'm pretty sure a curses library (ncurses of Linux/MacOS, pdcurses for windows) needs to be installed. Will add instructions.  Short version: MacOS - use homebrew to install the `ncurses` formula;  Linux - depends on your package manager, look for `ncursesw` or `ncurses`;  Windows - google `pdcurses windows` and good luck!

### Building

```
$ cargo build
```
add the `--release` flag if you don't plan to debug anything:
```
$ cargo build --release
```

### Testing

TBD!

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

# June 23, 2023

Refactored a lot of the code to make it more readable and manageable.  I also rewrote the terminal implementation to use either easycurses or pancurses, which are mostly the same except pancurses exposes mouse click and location info.  Easycurses should be able to do this by accessing the underlying pancurses lib.

## Working
* Curses-based terminal interpreter (two!) with working zmachine screen model including color and font styling, though italic characters are underlined due to limitations in curses.  Mouse input is also working correctly in Beyond Zork
* Multiple undo states as with previous update
* Save/restore game state to/from Quetzal IFF files using compressed memory
* Transcripting
* Passes czech.z5 and praxis.z5 tests
* Everything works in etude.z5

## Fixed!
* AREAD opcode correctly sets the text buffer positions of words, which fixed problems with jigsaw.v8

## Backlog
* Restore sound support after refactoring it out
* Input streams
* SAVE and RESTORE data (V5+)
* Modify STATUS_LINE to handle narrow screens by eliding text

---
---

# March 11, 2023

## Working

* Curses-based terminal interpreter with working zmachine screen model support including color and font styling.  Color, however is inconsistent in different shells and on different platforms due to differences in the underlying curses libraries (I think).  "True color" is even more inconsistent, for good measure.
* Multiple undo states keeping the most recent 10 stored undo states.
* Save and restore Quetzal files using compressed memory format.  The interpreter will suggest an autonumbered save file (lurking-horror-01.ifzs), and on restore suggest the most recent (highest numbered) file it finds.
* Transcripting to auto-incrementing file names.
* Passes czech.z5, etude.z5, praxis.z5 test suites.
* Mouse input in Beyond Zork.
* Sound effects in The Lurking Horror with a suitable blorb resource file containing OGG sound data.

## Not working
* AIFF sound playback ... need to find a Rust lib for this.
* Infocom V6 games (Zork 0, Shogun, Journey, Arthur) - most V6 instructions will panic.
* Infocom V5 sound interrupts (Sherlock is the only V5 game with sound?)
* Input streams (reading commands from a file instead of keyboard)
* V5+ save/restore data ... not extensively used

## Bugs
* Beyond Zork windowed display has minor glitches where spaces appear where they shouldn't be.
* Jigsaw won't show the puzzle frame.
* Small screen sizes can cause problems with status line display

## Backlog
* V5 sound interrupts
* Fix handling of small (< 80 column) screens
* Debug Beyond Zork display glitches
* Debug Jigsaw weirdness
* Input streams
* Save/restore data

## Future
* Browser-based interpreter
* Native interpreters for MacOS/Linux/Windows

---
---

# Feb 19, 2023

## Why?

I want to learn Rust.  The zmachine is a well-documented virtual machine with implementations dating back to 1980 on platforms from the Apple ][ to the (Sinclar) ZX.  I've also worked on other implementations, first porting the [Frotz](https://www.ifwiki.org/Frotz) interpreter to the Apple //gs in the mid 90s and later authoring (unreleased) Java and Clojure implementations.  It seems to be my go-to when I don't have any better ideas.

## What

The general idea is to separate the virtual machine that executes code from the interpreter, which provides the user interface.  I have wild and unexplored ideas of native UX implementations, web implementations, etc.

It's going to be sloppy - I'm still acclimating to the peculiarities of Rust and figuring out what I can get from a library vs. what I have to write myself.

---
---

## History
0.0.1 - ... work in progress


