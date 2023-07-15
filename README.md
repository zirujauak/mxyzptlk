# mxyzptlk
![example branch parameter](https://github.com/zirujauak/mxyzptlk/actions/workflows/cargo-checkmate.yaml/badge.svg?branch=main)

An Infocom(tm) ZMachine virtual machine implemented in Rust.

All source code is Copyright (c) 2023 by Evan Day.  All rights reserved.

This program and source code are licensed under the GNU General Public License V3 (GPLv3). See `LICENSE.md` for the full text of the license.  Full source code is available [here](https://github.com/zirujauak/mxyzptlk).

This software is free to use and provided without any warranty or assurance of correctness or promise of support.  However, please feel free send any bug reports, feedback, or requests for features [here](https://github.com/zirujauak/mxyzptlk/issues).

This interpreter follows the [1.0 Standard](https://inform-fiction.org/zmachine/standards/z1point1/index.html) ZMachine specification, the [1.4 Quetzal](http://inform-fiction.org/zmachine/standards/quetzal/index.html) specification, and [2.0.4 Blorb](https://www.eblong.com/zarf/blorb/blorb.html) specification.

The current release is `1.0.0-beta.1`

## Installation
1. Download the appropriate release archive for your system.  Archives are named by platform:
    * `aarch64-apple-darwin` for ARM-based (M1, M2) Macs
    * `x86_64-apple-darwin` for Intel-based Macs
    * `x86_64-pc-windows-msvc` for 64-bit X86 Windows
    * ... More platforms to be added later
2. Decompress the tar/gzip archive using your favorite decompression tool:
    * Mac/Linux: `tar -xzvf mxyzptlk-{platform}-1.0-beta.1.tar.gz`
    * Windows: as above with a unix shell (gitbash, etc), or use a program like (7Zip)[https://www.7-zip.org/]

        Each archive contains 4 binaries named `mxyzptlk-{features}` as described below.
3. Choose the terminal and sound configuration binary and copy it to a local `bin/` directory (`/usr/local/bin` on most Linux and Mac installations) for ease of use.  

    There are several binaries available for each platform:
    * `-pancurses` - using the `pancurses` crate directly
    * `-easycurses` - using the `easycurses` wrapper for pancurses
    * `-{terminal}-libsndfile` - the above plus `libsndfile` to convert AIFF sound resources

    So, for example, `mxyzptlk-pancurses-libsndfile[.exe]` is a build requires `libsndfile`, while `mxyzptlk-easycurses[.exe]` is a build using `easycurses` that does not depend on `libsndfile` (and is, therefore, unable to use AIFF sound resources).

    **NOTE**: The `easycurses` builds are an artifact of early development efforts and will be removed before the final `1.0.0` release.

4. Optionally, copy the `log4rs.yml` and `config.yml` files to a `.mxyzptlk/` directory in your "home" directory.  The default configuration does not enable logging, so unless you want to change the default color scheme (white on black) or generate logs, neither file is required.

### `libsndfile`

The generally available Blorb files all have AIFF sound resources.  AIFF is an antiquated format that isn't supported by any of the Rust audio crates that I've been able to find.  To get around this limitation, `libsndfile` is used to convert the AIFF sounds to another format (FLAC or Ogg/Vorbis) that can be played.  To get around the `libsndfile` dependency, one can extract the AIFF resources from the Blorb IFF file, convert them to Ogg/Vorbis, then rebuild the Blorb file, which is probably more work than it's worth.

For the `-libsndfile` binaries, `libsndfile` must be available, obviously.
* **Linux**: many distros already include `libsndfile` in base installs, but if not you can use the package manager to install it.  Specific instructions vary by package manager
* **Mac**: Sample instructions provided for using Homebrew:
    1. Install the `libsndfile` formula
    ```
    brew install libsndfile
    ``` 

    2. Edit `~/.zshenv` and add the following line:  
    ```
    export LIBRARY_PATH=$LIBRARY_PATH:$(brew --prefix)/lib
    ```
* **Windows**: 
    [`sndfile.dll`](https://github.com/libsndfile/libsndfile/releases) needs to be in the `PATH` environment variable.  If you keep the DLL in the same directory where you run `mxzyptlk.exe`, then it should get loaded.  If you install the binary somewhere and add it to the `PATH`, then copying the DLL to the same location should work fine.  If the DLL can't be located, execution will terminate immediately with an error.

    It's worth pointing out that sound-enabled games will run with a non-libsndfile binary, you just may not hear any sounds play.  Sounds are a gimmick, really, and you don't miss much without them.  It was fun to code.

### Games

There are a couple of places to get game files:

* The [Interactive Fiction Archive](https://www.ifarchive.org/indexes/if-archive/) 

    The if-archive has quite a number of free games.  This interpreter is for "zcode" games only, generally those with names ending in ".z{version}".  Only versions 3, 4, 5, 7, and 8 are supported, though version 7 is somewhat rare and has not been tested.

    Download a zcode file \([Curses](https://www.ifarchive.org/if-archive/games/zcode/curses.z5), for example\) and try it out:
    ```
    mxyzptlk curses.z5
    ```

    Further, the if-archive has Blorb resource [files](https://www.ifarchive.org/indexes/if-archive/infocom/media/blorb/) with sounds for both `The Lurking Horror` and `Sherlock`.  It may be necessary to [patch](https://www.ifarchive.org/indexes/if-archive/infocom/patches/) your zcode file to take advantage of these resources.

    **NOTE**: The windows binary may segfault on exit.  I'm still investigating why this is, but it doesn't seem to hurt anything.

* The [Masterpieces Of Infocom](https://en.wikipedia.org/wiki/Classic_Text_Adventure_Masterpieces_of_Infocom) CD-ROM

    Published by Activision back in 1996, this release contains zcode files for every* Infocom interactive fiction game published.  If you can procure a copy, the `.DAT` files on this CD are the zcode files.

    *\* Excepting `The Hitchhiker's Guide To The Galaxy` and `James Clavell's Shogun`, which are absent due to expired licensing agreements.  `Shogun` is a V6 game and is not supported, but `THHGTTG` is a classic and is sorely missed.*

#### **A Note About Blorb Resource Files**
Certain revisions of `The Lurking Horror` and `Sherlock` support sound effects.  In order to use them, a Blorb file with the sound resources needs to be located in the same directory as the game file, with same filename and a `.blorb` or `.blb` extension in order for `mxyzptlk` to locate it. In other words, when playing `the-lurking-horror.z3`, the Blorb file should be named `the-lurking-horror.blorb` or `the-lurking-horror.blb`.

#### **A Note About Files (Saves And Transcripts)**
When saving or restoring game state, `mxyzptlk` will prompt for a filename.  When saving, the default name is `{zcode-file-minus-extension}-##.ifzs`, where `##` starts at "01" and will count upwards to the first filename not found on in the current working directory.  When restoring, the prompt defaults to the last (numerically) file found on disk.  Attempting to save to an invalid location or restore an invalid file will display an error message to the screen, but shouldn't cause the game to crash or exit. 

Transcripting (recording the game session via the `script` and `unscript` command in most games) uses the same naming as save except with a `.txt` extension.  A prompt for a filename is only shown once* during program execution and all transcripted text will be placed in the same file.

Any errors creating, opening, reading, or writing to files are reported by the interpreter and shouldn't halt game execution.  

*\*If creation of the transcript file fails, the game code may print the transcript heading, but transcripting will _not_ be enabled and not text is written to disk.  Attempting to start scripting a second time _will_ prompt for a filename again*

### Configuration

As referenced in the installation instructions, the `config.yml` as shipped contains the default configuration.  If you're happy with the default color screen (white foreground on black background) and no logging, then you don't need this file.  However, if you wish to change the default color scheme, terminal library, or enable logging, you'll need to ensure a copy of this file is either present in the same directory you'll execute `mxyzptlk` from or the `.mxyzptlk/` directory in "home" directory (which varies by platform).

### Logs

When logging is enabled, execution will dump quite a bit of output to various `.log` files in the current working directory.  Logging is disabled by default, but can be enabled via the `config.yml` file (see above) and further refined by changing the various `level` values used for different log files.  


## Building from source

### Required libraries
The following external system libraries are optional:
* libsndfile

    The `sndfile` feature controls whether `libsndfile` is used to convert AIFF sounds to another format.
    
    See the [Installation](#Installation) section above for instructions on installing the library.

### Building
```
cargo build
```
add the `--release` flag if you don't plan to debug anything:
```
cargo build --release
```

The `pancurses` feature is enabled by default.  This will yield a binary that uses the pancurses terminal library and can play FLAC or Ogg/Vorbis sound resources.

* `easycurses` - this will replace the pancurses dependency with the easycures-rs wrapper around pancurses.

    **NOTE**: The `easycurses` feature is an artifact of early development efforts and will be removed before the final `1.0.0` release.
    
* `sndfile` - include `libsndfile` for automatic AIFF sound resource conversion.

For example, for a build using the `easycurses` terminal library without a dependency on `libsndfile`:
```
cargo build --release --no-default-features --features easycurses
```

### Testing

Unit tests are currently a work in progress.  To run the tests:

```
cargo test
```

The `zcode` directory contains several freely available test files.  I did not author these files and provide no guarantee of correctness.  I do wish to thank the authors, however, because these tests were invaluable in the process of tracking down and squashing several bugs resulting from my misinterpretations of the ZMachine standard.

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

Some are interactive, like TerpEtude, while others just run a sequence of tests and output results.

## Security Advisories

\*sigh\* Full disclosure ... there are security advisories on a couple of dependencies that are rather old and probably won't get fixed upstream.  I can probably patch these locally for release packages in the future, but the actual risk is, IMHO, neglible.

* [`RUSTSEC-2019-0005`](https://rustsec.org/advisories/RUSTSEC-2019-0005) for `pancurses`, related to the `mvprintw` and `printw` functions in `ncurses`, which are not used.
* [`RUSTSEC-2019-0006`](https://rustsec.org/advisories/RUSTSEC-2019-0006) for `ncurses`, related to the above
* [`RUSTSEC-2020-0071`](https://rustsec.org/advisories/RUSTSEC-2020-0071) for `time` via `chrono` via `log4rs`
