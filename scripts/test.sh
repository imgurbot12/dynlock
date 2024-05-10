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
    $LOCAL/../target/debug/shaderlock
    swaymsg exit
    ;;
  *)
    cargo build
    run_sway
    ;;
esac
