#!/bin/sh

NAME=`basename $(pwd)`
PACKAGE="$NAME-${GITHUB_REF_NAME:-latest}"

RELEASE="target/release/$NAME"

# build binaries
cargo build --all --release
strip "$RELEASE"

# build project structure
mkdir -p "$PACKAGE/bin"
cp README.md "$PACKAGE/."
cp LICENSE "$PACKAGE/."
cp -r shaders "$PACKAGE/."
cp default-config.yaml "$PACKAGE/config.yaml"
mv $RELEASE "$PACKAGE/bin/."
cp .github/install-linux.sh "$PACKAGE/install.sh"

# tar items together
tar czf "linux-amd64.tar.gz" "$PACKAGE"

