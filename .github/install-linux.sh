#!/bin/sh

SELF=`realpath $0`
NAME=`basename $(dirname $SELF) | cut -d- -f1`
CONFIG="$HOME/.config/$NAME"

cd $(dirname $SELF)
sudo cp -vf bin/* /usr/local/bin/.
mkdir -p "$CONFIG"
cp -vfr shaders "$CONFIG/."
cp -vf config.yaml "$CONFIG/."
