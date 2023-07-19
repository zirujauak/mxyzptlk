Version 1.0.0.beta-2 (2023-07-16)
==========================
- Second beta release
    * Several minor bugs fixed:
        * Better support for platform-specific "home" directories for finding configuration files
        * Adjust sound volume scaling by platform for more consistent playback volume
        * Look for blorbs in the directory where the game file is located
        * Fix minor cosmetic issues with V3 status line
        * Correct handling of terminating characters when reading input
        * Fix predictable random number generation

    * Full unit testing; test coverage reporting

\**V7 support is untested.  Version 7 was transitory and rarely used.*Version 1.0.0.beta-1 (2023-07-02)
==========================
- First beta release

    Includes full support for V3, V4, V5, V7* and V8 ZMachine versions, including:
    
    * Color text with a color terminal (V5+) 
    * Sound (for `The Lurking Horror` and `Sherlock` when the associated blorb resource files are available) when the `-libsndfile` build is used.
    * Mouse support (for `Beyond Zork`)

\**V7 support is untested.  Version 7 was transitory and rarely used.*

