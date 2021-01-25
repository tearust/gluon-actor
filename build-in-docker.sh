#!/bin/sh

DEV_MODE=$1
: ${DEV_MODE:="0"}

if [ $DEV_MODE -ne "0" ]; then
  docker-compose run --rm build-gluon-dev
else
  docker-compose run --rm build-gluon
fi

mkdir -p ../builds/runtime-cache
cp target/wasm32-unknown-unknown/release/gluon_actor_signed.wasm ../builds/runtime-cache/
