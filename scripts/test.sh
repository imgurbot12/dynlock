#!/bin/sh

set -e

export SELF=`realpath $0`
export LOCAL=`dirname $SELF`

run_sway() {
  export SHADERLOCK=$(pwd)
  sway -c "$LOCAL/sway.config"
}

case "$1" in
  "run")
    cd $LOCAL/..
    $LOCAL/../target/debug/dynlock $FLAGS
    swaymsg exit
    ;;
  *)
    export FLAGS="$@"
    cargo build
    run_sway
    ;;
esac
