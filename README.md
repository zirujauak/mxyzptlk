# mxyzptlk

An Infocom(tm) ZMachine virtual machine implemented in Rust.

All source code is Copyright (c) 2023 by Evan Day.  All rights reserved.

This program and source code are licensed under the Creative Commmons Attribution-ShareAlike 4.0 International license. See `LICENSE.md` for the full text of the license.  Source code is available [here](https://github.com/zirujauak/mxyzptlk).

This software is free to use and provided without any warranty or assurance of correctness or promise of support.  However, please feel free send any bug reports, feedback, or requests for features [here](https://github.com/zirujauak/mxyzptlk/issues).

This interpreter follows the [1.0 Standard](https://inform-fiction.org/zmachine/standards/z1point1/index.html) ZMachine specification, the [1.4 Quetzal](http://inform-fiction.org/zmachine/standards/quetzal/index.html) specification, and [2.0.4 Blorb](https://www.eblong.com/zarf/blorb/blorb.html) specification.

The current release is `1.0.0-beta.1`

## Installation
1. Download the appropriate release binary for your system.  There are several binaries available for each platform:
    * `-pancurses` - using the `pancurses` crate directly
    * `-easycurses` - using the `easycurses` wrapper for pancurses
    * `-libsndfile` - using `libsndfile` to convert AIFF sound resources

        So, `aarch64-apple-darwin-pancurses-libsndfile` is a build for Apple silicon (M1/M2) Macs that requires `libsndfile`, while `aarch64-apple-darwin-easycurses` is a build for Apple silicon using `easycurses` that does not depend on `libsndfile` (and is unable to use AIFF sound resources).


    Platforms:
    * `aarch64-apple-darwin` for ARM-based (M1, M2) Macs
    * `x86_64-apple-darwin` for Intel-based Macs
    * ... More platforms to be added later
2. Decompress the tar/gzip archive using your favorite decompression tool:
    * `tar -xzvf mxyzptlk-1.0-beta.1-aarch64-apple-darwin.tar.gz` for Mac/Linux
    * 7Zip, etc. on Windows

        Each archive contains 4 binaries named `mxyzptlk-...` as described above.
3. Choose the terminal and sound configuration binary and copy it to a local `bin/` directory (`/usr/local/bin` on most Linux and Mac installations) for ease of use.  
4. Optionally, copy the `log4rs.yml` and `config.yml` files to a `.mxyzptlk/` directory in your "home" directory.  The default configuration does not enable logging, so unless you want to change the default color scheme (white on black) or generate logs, neither file is required.

### `libsndfile`

For `-libsndfile` binaries, `libsndfile` must be available.  
* Linux: many distros already include `libsndfile` in base installs, but if not you can use the package manager to install it.  Specific instructions vary by package manager
* Mac: Sample instructions provided for using Homebrew:
    1. Install the `libsndfile` formula
    ```
    brew install libsndfile
    ``` 

    2. Edit `~/.zshenv` and add the following line:  
    ```
    export LIBRARY_PATH=$LIBRARY_PATH:$(brew --prefix)/lib```
* Windows: There are 32- and 64-bit Windows [installers](http://www.mega-nerd.com/libsndfile/#Download) available, but I haven't (yet) tested them.

### Games

There are a couple of places to get game files:

* The [Interactive Fiction Archive](https://www.ifarchive.org/indexes/if-archive/) 

    The if-archive has quite a number of free games.  This interpreter is for "zcode" games only, generally those with names ending in ".z{version}".  Only `version`s 3, 4, 5, 7, and 8 are supported, though version 7 is somewhat rare and I haven't tried running one yet.

    Download a zcode file ([Curses](https://www.ifarchive.org/if-archive/games/zcode/curses.z5), for example) and try it out:
    ```
    mxyzptlk curses.z5
    ```

    Further, the if-archive has Blorb resource [files](https://www.ifarchive.org/indexes/if-archive/infocom/media/blorb/) with sounds for both `The Lurking Horror` and `Sherlock`.  It may be necessary to [patch](https://www.ifarchive.org/indexes/if-archive/infocom/patches/) your zcode file to take advantage of these resources.

* The [Masterpieces Of Infocom](https://en.wikipedia.org/wiki/Classic_Text_Adventure_Masterpieces_of_Infocom) CD-ROM

    Published by Activision back in 1996, contains zcode files for (almost*) every Infocom game ever published.  If you can procure a copy, the `.DAT` files on this CD are the zcode files.

    *\* Except `The Hitchhiker's Guide To The Galaxy` and `James Clavell's Shogun` due to licensing issues.  `Shogun` is a graphical V6 game that won't run anyway, but `HHGTTG` is a classic that is sorely missed.*

#### **A Note About Files (Saves And Transcripts)**
When saving or restoring game state, `mxyzptlk` will prompt for a filename.  When saving, the default name is `{zcode-file-minus-extension}-##.ifzs`, where `##` starts at "01" and will count upwards to the first filename not present on disk.  When restoring, the prompt defaults to the last (numerically) file found on disk.  

Transcripting (recording the game session via the `script` and `unscript` command in most games) uses the same naming as save/restore except with a `.txt` extension.

Any errors creating, opening, reading, or writing to files are reported by the interpreter and shouldn't halt game execution.  If transcripting fails, the game will print the transcripting header, but transcripting will not be enabled.

### Configuration

As referenced in the installation instructions, the `config.yml` as shipped contains the default configuration.  If you're happy with the default color screen (white foreground on black background) and no logging, then you don't need this file.  However, if you wish to change the default color scheme, terminal library, or enable logging, you'll need to ensure a copy of this file is either present in the same directory you'll execute `mxyzptlk` from or the `.mxyzptlk/` directory in "home" directory (which varies by platform).

### Logs

When logging is enabled, execution will dump quite a bit of output to various `.log` files in the current working directory.  Logging is disabled by default, but can be enabled via the `config.yml` file (see above) and further refined by changing the various `level` values used for different log files.  


## Building from source

### Required libraries
The following libraries are required to build from source:
* libsndfile

    The `sndfile` feature (enabled by default) controls whether libsndfile is used to convert AIFF sounds to another format.  To disable `sndfile`, pass the `--no-default-features` flag to cargo when you build or run. See the [Installation](#Installation) section above for instructions on installing the library.

### Building
```
cargo build
```
add the `--release` flag if you don't plan to debug anything:
```
cargo build --release
```

The default build uses `pancurses` and `libsndfile`.  Use the `--no-default-features` and `--features` flags on `cargo` to change the build.

For example, to build using `easycurses` without `libsndfile`:
```
cargo build --release --no-default-features --features easycurses
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

## Security Advisories

\*sigh\* Full disclosure ... there are security advisories on a couple of dependencies that are rather old and probably won't get fixed upstream.  I can probably patch these locally for release packages in the future, but the actual risk is, IMHO, neglible.

* [`RUSTSEC-2019-0005`](https://rustsec.org/advisories/RUSTSEC-2019-0005) for `pancurses`, related to the `mvprintw` and `printw` functions in `ncurses`, which are not used.
* ['RUSTSEC-2019-0006'](https://rustsec.org/advisories/RUSTSEC-2019-0006) for `ncurses`, related to the above
* ['RUSTSEC-2020-0071`](https://rustsec.org/advisories/RUSTSEC-2020-0071) for `time` via `chrono` via `log4rs`