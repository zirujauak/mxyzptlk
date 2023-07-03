# Point this at your homebrew install, which is usually /opt/homebrew/bin/brew
# However, the author cross-compiles from a M1 and the x86_64 homebrew is 
# installed to /usr/local/homebrew/bin/brew
#BREW=/opt/homebrew/bin/brew
BREW=/usr/local/homebrew/bin/brew

LIBRARY_PATH=$(BREW --prefix)/lib ./build-release.sh x86_64-apple-darwin $1