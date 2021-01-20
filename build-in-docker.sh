#!/bin/sh

docker-compose run --rm build-gluon

mkdir -p ../builds/runtime-cache
cp target/wasm32-unknown-unknown/release/gluon_actor_signed.wasm ../builds/runtime-cache/
