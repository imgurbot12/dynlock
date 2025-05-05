#!/bin/sh

set -e

export SELF=`realpath $0`
export LOCAL=`dirname $SELF`

run_sway() {
  export SHADERLOCK=$(pwd)
  sway -c "$LOCAL/sway.config"
}

run_hyprland() {
  export SHADERLOCK=$(pwd)
  Hyprland -c "$LOCAL/hyprland.config"
}

command="$1"
shift

case "$command" in
  "run")
    cd $LOCAL/..
    RUST_BACKTRACE=1 $LOCAL/../target/debug/dynlock $FLAGS
    swaymsg exit || hyprctl dispatch exit
    ;;
  "sway")
    export FLAGS="$@"
    cargo build
    run_sway
    ;;
  "hyprland")
    export FLAGS="$@"
    cargo build
    run_hyprland
    ;;
  *)
    echo "usage: $(dirname $0) <sway/hyprland>"
    exit 1
    ;;
esac
