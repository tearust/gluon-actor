#!/bin/sh

docker-compose run --rm build-gluon

mkdir -p ../builds/actors
cp target/wasm32-unknown-unknown/release/gluon_actor_signed.wasm ../builds/actors/
