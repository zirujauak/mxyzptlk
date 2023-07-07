base=mxyzptlk-$1-$2

if [ -d $base ]; then
    rm -r $base
fi

mkdir mxyzptlk-$1-$2

if [ -f ${base}.tar.gz ]; then
    rm mxyzptlk-$1-$2.tar.gz
fi

cargo clean
cargo build --quiet --release --target $1 --features pancurses,sndfile
cp target/$1/release/mxyzptlk $base/mxyzptlk-pancurses-libsndfile

cargo build --quiet --release --target $1 --features pancurses
cp target/$1/release/mxyzptlk $base/mxyzptlk-pancurses

cargo build --quiet --release --target $1 --features easycurses,sndfile
cp target/$1/release/mxyzptlk $base/mxyzptlk-easycurses-libsndfile

cargo build --quiet --release --target $1 --features easycurses
cp target/$1/release/mxyzptlk $base/mxyzptlk-easycurses

cp CHANGELOG.md $base
cp LICENSE.md $base
cp README.md $base
cp RELEASES.md $base
cp config.yml $base
cp log4rs.yml $base

tar cvfz ${base}.tar.gz $base
rm -r $base
