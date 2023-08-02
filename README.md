# mxyzptlk
![example branch parameter](https://github.com/zirujauak/mxyzptlk/actions/workflows/cargo-checkmate.yaml/badge.svg?branch=main)

An Infocom(tm) ZMachine virtual machine implemented in Rust.

All source code is Copyright (c) 2023 by Evan Day.  All rights reserved.

This program and source code are licensed under the GNU General Public License V3 (GPLv3). See `LICENSE.md` for the full text of the license.  Full source code is available [here](https://github.com/zirujauak/mxyzptlk).

This software is free to use and provided without any warranty, assurance of correctness or promise of support.  However, submit bug reports, feedback, or requests for features [here](https://github.com/zirujauak/mxyzptlk/issues).

This interpreter adheres the [standard 1.0](https://inform-fiction.org/zmachine/standards/z1point1/index.html) ZMachine specification, and uses the [Quetzal 1.4](http://inform-fiction.org/zmachine/standards/quetzal/index.html) save file specification, and supports [Blorb 2.04](https://www.eblong.com/zarf/blorb/blorb.html) sound resource files.

The current release is `1.0.0-beta.3`

This software dynamically links the [libsndfile](https://github.com/libsndfile/libsndfile) library in certain configurations, licensed under the [GNU LGPL 2.1](https://github.com/libsndfile/libsndfile/blob/master/COPYING) license.  No source or executable code from this library is intentionally included in any of the release packages hosted in this repository.

## Installation
1. Download the appropriate release tarball for your system.  Archives are named by platform:
    * `aarch64-apple-darwin` for ARM-based (M1, M2) Macs
    * `aarch64-unknown-linux-gnu` for ARM-based (M1, M2) Macs running Asahi Linux
    * `x86_64-apple-darwin` for 64-bit Intel-based Macs
    * `x86_64-pc-windows-msvc` for 64-bit Intel Windows
    * `x86_64-unknown-linux-gnu` for 64-bit Intel Linux
    * ...
2. Decompress the tar/gzip archive using your favorite decompression tool:
    * Mac/Linux: `tar -xzvf mxyzptlk-{platform}-1.0-beta.1.tar.gz`
    * Windows: [7Zip](https://www.7-zip.org/) or another archiver that supports gzipped tar files.

        Each archive should contain 4 binaries (named `mxyzptlk-{features}` as described below), default configuration files, and assorted documentation.
3. Pick a terminal and sound configuration binary and copy it to a local `bin/` directory (`/usr/local/bin` on most Linux and Mac installations) for ease of use.  

    The available binaries are named according to supported features:
    * `-libsndfile` - uses the `libsndfile` to support AIFF sound resources.

    For example, `mxyzptlk-libsndfile[.exe]` requires `libsndfile`, while `mxyzptlk[.exe]` does not (and is, therefore, unable to utilize AIFF sound resources).

4. Optionally, copy the `log4rs.yml` and `config.yml` files to a `.mxyzptlk/` directory in your "home" directory (varies by platform, `/home/{username}` on Linux/MacOS, generally `C:\Users\{username}` on Windows).  The default configuration does not enable logging, so unless you want to change the default color scheme (white on black) or enable logging, these files are not required.

#### `libsndfile`

The generally available Blorb files all have AIFF sound resources.  AIFF sounds aren't supported by any of the Rust audio crates that I've been able to find.  To get around this limitation, `libsndfile` is used to convert the AIFF sounds to another format (currently FLAC) that can be played by [`rodio`](https://docs.rs/rodio/latest/rodio/).

It is possible to extract the AIFF sounds from a blorb, convert them to Ogg/Vorbis (The blorb specification only lists AIFF and OggV sounds) using any number of software packages or online tools and then reassemble the blorb.  The specifics are left as an exercise for the reader.

For the `-libsndfile` binaries, the `libsndfile` library must be installed, obviously:
* **Linux**: many distros include `libsndfile` in base installs, but if not you can use the package manager to install it.  Specific instructions vary by package manager.
* **MacOS**: Sample instructions provided using [homebrew](https://brew.sh/):
    1. Install the `libsndfile` formula
    ```
    brew install libsndfile
    ``` 

    2. Edit `~/.zshenv` or `~/.zprofile` and add the following line:  
    ```
    export LIBRARY_PATH=$LIBRARY_PATH:$(brew --prefix)/lib
    ```
* **Windows**: 
    The [`sndfile.dll`](https://github.com/libsndfile/libsndfile/releases) needs to be in the `PATH` environment variable.  If you keep the DLL in the same directory where you put `mxyzptlk.exe`, then it should load correctly.  If you install `mxyzptlk.exe` somewhere and add it to the `PATH`, then copying the DLL to that same location should work fine.  If the DLL can't be located, execution will terminate immediately with an error.

It's worth pointing out that sound-enabled games will run normally with a non-libsndfile binary.  It's not even necessary to have the blorb file containing the sounds.  You just won't not hear any sounds play.  Sound effects are atmospheric and you don't miss much without them.  It was fun to code, however.

### Games

#### File types
`mxyzptlk` supports both "raw" zcode files (typically files that end in `.z#`) and Blorb files (`.blb` or `.blorb`, usually) with an `Exec` entry in the `RIdx` chunk that points at the start of a `ZCOD` chunk.  Attempting to run a Blorb file without an `Exec` index will result in an error.  Wrapping code into a Blorb is convenient for file management, but not strictly necessary.

#### Where do I get games?
There are many places to get game files (legally or not), but I've listed my two favorite _legal_ sources:

* The [Interactive Fiction Archive](https://www.ifarchive.org/indexes/if-archive/) 

    The if-archive has a large number of free games.  This interpreter is for "zcode" games only, generally those with names ending in ".z{version}".  Only versions 3, 4, 5, 7, and 8 are supported.  Note that version 7 is somewhat rare and has not been tested yet.

    Download a zcode file from the archive ([Curses](https://www.ifarchive.org/if-archive/games/zcode/curses.z5), for example\) and try it out:
    ```
    mxyzptlk curses.z5
    ```

    Additionally, the if-archive has Blorb resource [files](https://www.ifarchive.org/indexes/if-archive/infocom/media/blorb/) with sounds for both `The Lurking Horror` and `Sherlock`.  It may be necessary to [patch](https://www.ifarchive.org/indexes/if-archive/infocom/patches/) the relevante zcode file(s) in order to take advantage of these resources.

* The [Masterpieces Of Infocom](https://en.wikipedia.org/wiki/Classic_Text_Adventure_Masterpieces_of_Infocom) CD-ROM

    Published by Activision back in 1996, this release contains zcode files for every* Infocom interactive fiction game published.  If you can procure a copy, the `.DAT` files on this CD are the zcode files.

    *\* Excepting `The Hitchhiker's Guide To The Galaxy` and `James Clavell's Shogun`, which are absent due to expired licensing agreements.  `Shogun` is a V6 game and is not supported, but `THHGTTG` is a classic and is sorely missed.*

#### **A Note About Blorb Resource Files**
Certain revisions of `The Lurking Horror` and `Sherlock` support sound effects.  In order to use them, a Blorb file with the sound resources needs to be located in the same directory as the game file, with same filename and a `.blorb` or `.blb` extension in order for `mxyzptlk` to locate it. In other words, when playing `the-lurking-horror.z3`, the Blorb file should be in the same directory as the game file and named `the-lurking-horror.blorb` or `the-lurking-horror.blb`.

#### **A Note About Files (Saves And Transcripts)**
When saving or restoring game state, `mxyzptlk` will prompt for a filename.  When saving, the default name is `{zcode-file-minus-extension}-##.ifzs`, where `##` starts at "01" and will count upwards to the first filename not found on in the current working directory.  When restoring, the prompt defaults to the last (numerically) file found on disk.  Attempting to save to an invalid location or restore an invalid file will display an error message to the screen, but shouldn't cause the game to crash or exit. 

Transcripting (recording the game session via the `script` and `unscript` command in most games) uses the same naming as save except with a `.txt` extension.  A prompt for a filename is only shown once* during program execution and all transcripted text will be placed in the same file.

File names ending in `.z#`, `.blorb`, or `.blb` are not permitted, nor will existing files be overwritten.

Any errors creating, opening, reading, or writing to files are reported by the interpreter and shouldn't halt game execution.  

*\*If creation of the transcript file fails, the game code may print the transcript heading, but transcripting will _not_ be enabled and no text is written to disk.  In this case, toggling scripting off and on again _will_ prompt for a filename again*

#### **A Note About Error Handling**
Errors may occur during gameplay.  Some games (Infocom's `Sherlock`, in particular) may unintentionally do things that aren't allowd (like manipulate an invalid attribute - look at you, `Sherlock`).  There may be bugs in games (or `mxyzptlk` itself) that result in more serious errors, such as a stack underflow or divide by zero.

There are two "types" of errors: error which _might_ be recoverable and those which are definitely not.  

For "recoverable" errors, behavior is defined by the `error_handling` setting in the configuration:
* `continue_warn_always` - display a notice every time an error occurs and prompt the user to continue or quit.
* `continue_warn_once` - display a notice the first time a particular error code is encountered and prompt the user to continue or quit.  If continue is chosen, any further occurrences of the error code are ignored.
* `ignore` - silently ignore any recoverable errors and continue.
* `abort` - treat recoverable errors as fatal error.

The default configuration will `ignore` recoverable errors, which is what most users will want to happen.  Game developers, however, will probably want to continue or abort on any error.  Error messaging includes the instruction counter, which may be cross-referenced with logs (which developers will probably want to enable) that may be used to diagnose and hopefully correct the problem.

"Recovering" from an error is implemente by running the next instruction in the program.  Except for the ART_SHIFT and LOG_SHIFT instructions, no store or branch is followed which may leave the program in an unpredictable or unplayable state.  Caveat actor.

### Configuration

As referenced in the installation instructions, the `config.yml` as shipped contains the default configuration.  If you're happy with the default color screen (white foreground on black background) and don't need logs for debugging a zcode file or fixing bugs in the interpreter, then you probably don't need this file.  However, if you wish to change the default color scheme, terminal library, or enable logging, you'll need to ensure a copy of this file is either present in the `.mxyzptlk/` directory in the "home" directory (which varies by platform) or the current working directory where `mxyzptlk` is launched from, with the current working directory taking precedence.

### Logs

When logging is enabled, execution will dump quite a bit of output to various `.log` files in the current working directory.  Logging is disabled by default, but can be enabled via the `config.yml` file (see above) and further refined by changing the various `level` values in `log4rs.yml` for different log files.  As with `config.yml`, `log4rs.yml` should be located in the `.mxyzptlk/` directory in the home directory or the current working directory, with any copy in the current working directory taking precedence.

Each log message includes the instruction counter, making is relatively easy to cross-reference data from different log files.  

The logs are split across several files as follows:
* `instruction.log` - instruction execution
* `resource.log` - resource file
* `screen.log` - user-input (keyboard/mouse) and screen output
* `sound.log` - sound conversion and playback
* `state.log` - runtime state
* `stream.log` - input and output streams
* `mxyzptlk.log` - all of the above, all at once.

## Building from source

### Required libraries
The following external libraries are optional:
* `libsndfile` (`libsndfile1`, `libsndfile-dev`, or `libsndfile1-dev` in some cases)

    The `sndfile` feature controls whether `libsndfile` is used to convert AIFF sounds to another format.
    
    See the [Installation](#Installation) section above for instructions on installing the library.

* Linux platforms may require additional libraries when the `sndfile` feature is enabled.  If compilation fails, the error output will list the missing libraries.  Below are libraries that I've seen required on various Linux platforms during development:
    * ALSA development libraries (`libasound2-dev`)
    * UDEV development libraries (`libudev-dev`)

    Compilation
### Building
```
cargo build
```
add the `--release` flag if you don't plan to debug anything:
```
cargo build --release
```

#### Features
* `sndfile` - include `libsndfile` for automatic AIFF sound resource conversion.

To build with `libsndfile`:
```
cargo build --release --features sndfile
```

### Testing

The source code includes extensive unit tests.  To run these tests:

```
cargo test
```

Unit tests silently create and (usually) delete several files named `test-...`.  Avoid saving games or transcripts in the source resposiblty with similar names to avoid potential data loss.

### Profiling

See the `README` in the `build/` directory for information about generating test coverage reports.

### Integration testing
The `zcode/` directory contains several freely available test programs that can be used to verify interpreter behavior.  I did not author these programs and provide no guarantee of correctness.  I do wish to extend my deepest gratitude to the authors.  These programs were invaluable in the process of tracking down and squashing several bugs resulting from my many misinterpretations of the ZMachine standard documentation.

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

Some of thes programs are interactive, such as TerpEtude, while others run a sequence of functional tests and output results.

## Security Advisories

\*sigh\* Full disclosure ... there are security advisories on a couple of dependencies that are rather old and probably won't get fixed upstream.  I can probably patch these locally for release packages in the future, but the actual risk is, in my opinion, neglible.

* [`RUSTSEC-2019-0005`](https://rustsec.org/advisories/RUSTSEC-2019-0005) for `pancurses`, related to the `mvprintw` and `printw` functions in `ncurses`, which are not used.
* [`RUSTSEC-2019-0006`](https://rustsec.org/advisories/RUSTSEC-2019-0006) for `ncurses`, related to the above
* [`RUSTSEC-2020-0071`](https://rustsec.org/advisories/RUSTSEC-2020-0071) for `time` via `chrono` via `log4rs`
