#!/bin/zsh
# Point this at your homebrew install, which is usually /opt/homebrew/bin/brew
BREW=/opt/homebrew/bin/brew
LIBRARY_PATH=$(BREW --prefix)/lib ./build-release.sh aarch64-apple-darwin $1