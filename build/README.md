# Test coverage scripts

These scripts are Mac-specific and require XCode command line tools.  Additionally, `rustfilt` must be installed:
```
cargo install rustfilt
```

Two scripts are supplied to evaluate unit test coverage:
* `report.sh` - outputs a summary report of test coverage
* `profile.sh` - outputs source coverage detail, file by file.

These are experimental and may not work out-of-the-box.  I suspect the `--target ...` argument may need to be updated, depending on the filenames cargo puts in `target/debug/deps`.

# Build scripts

These should be run from the repo root:

For example:
```
build/release-mac.sh 1.0.0
```
will build and create the following packages:
* `mxyzptlk-aarch64-apple-darwin-1.0.0.tar.gz`
* `mxyzptlk-x86_64-apple-darwin-1.0.0.tar.gz`

**NOTE**: `cargo clean` is run before each package is built.
    
Each release package will contain:
* `mxyzptlk-pancurses-sndfile` binary
* `mxyzptlk-pancurses` binary
* `mxyzptlk-easycurses-sndfile` binary
* `mxyzptlk-easycurses` binary
* CHANGELOG.md
* LICENSE.md
* README.md
* RELEASES.md
* config.yml
* log4rs.yml

## Mac
* `release-mac.sh {release-semver}`

    Pre-requisites: 
    
    * [Homebrew](https://brew.sh/)

        This script will build both ARM64 (M1, M2, etc) and X86_64 (Intel) packages.  In order to cross-compile, both the ARM64 and X86_64 versions of homebrew should be installed.  The author has the ARM64 version installed normally (in /opt/homebrew) and the X86_64 version in /usr/local/homebrew.  Update the `BREW` variables in `release-aarch64.sh` and `release-x86_64.sh` to reflect your configuration.

    * [libsndfile](http://www.mega-nerd.com/libsndfile/)

        Install via homebrew:

        Remember to install for _both_ ARM64 and X86_64.  Assuming a similar install as described above:
        ```
        brew install libsndfile
        ```
        ```
        /usr/local/homebrew/bin/brew install libsndfile
        ```

    This script runs `release-aarch64.sh` and `release-x86_64.sh`, building 4 configurations (pancurses + libsndfile, pancurses, easycurses + libsndfile, easycurses) and depositing the resulting `.tar.gz` package files in the repository root.

* `release-aarch64.sh {release-semver}`

    As above, but this script just builds the ARM64 release package.

* `release-x86_64.sh {release-semver}`

    As above, but this script just build the X86_64 release package.

## Linux
TBD

## Windows
TBD