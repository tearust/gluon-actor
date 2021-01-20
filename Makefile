# Copyright 2015-2019 Capital One Services, LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
# http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

COLOR ?= always # Valid COLOR options: {always, auto, never}
CARGO = cargo --color $(COLOR)
TARGET = target/wasm32-unknown-unknown
DEBUG = $(TARGET)/debug
RELEASE = $(TARGET)/release
KEYDIR ?= keys

.PHONY: all bench build check clean doc test update keys keys-account keys-module

all: build

bench:
	@$(CARGO) bench

build:
	@$(CARGO) build --target wasm32-unknown-unknown
	wascap sign $(DEBUG)/gluon_actor.wasm $(DEBUG)/gluon_actor_signed.wasm -i $(KEYDIR)/account.nk -u $(KEYDIR)/module.nk -l -g -z -c tea:keyvalue -c tea:layer1 -c tea:env -c tea:ipfs -c tea:tpm -c tea:crypto -c tea:intercom -n "TEA project: Gluon Wallet Actor"

check:
	@$(CARGO) check

clean:
	@$(CARGO) clean

doc:
	@$(CARGO) doc

test: build
	@$(CARGO) test

update:
	@$(CARGO) update

dev:
	@$(CARGO) build --release --target wasm32-unknown-unknown --features "dev"
	wascap sign $(RELEASE)/gluon_actor.wasm $(RELEASE)/gluon_actor_signed.wasm -i $(KEYDIR)/account.nk -u $(KEYDIR)/module.nk -l -g -z -c tea:keyvalue -c tea:layer1 -c tea:env -c tea:ipfs -c tea:tpm -c tea:crypto -c tea:intercom -n "TEA project: Gluon Wallet Actor"

release:
	@$(CARGO) build --release --target wasm32-unknown-unknown
	wascap sign $(RELEASE)/gluon_actor.wasm $(RELEASE)/gluon_actor_signed.wasm -i $(KEYDIR)/account.nk -u $(KEYDIR)/module.nk -l -g -z -c tea:keyvalue -c tea:layer1 -c tea:env -c tea:ipfs -c tea:tpm -c tea:crypto -c tea:intercom -n "TEA project: Gluon Wallet Actor"
	
keys: keys-account
keys: keys-module

keys-account:
	@mkdir -p $(KEYDIR)
	nk gen account > $(KEYDIR)/account.txt
	awk '/Seed/{ print $$2 }' $(KEYDIR)/account.txt > $(KEYDIR)/account.nk

keys-module:
	@mkdir -p $(KEYDIR)
	nk gen module > $(KEYDIR)/module.txt
	awk '/Seed/{ print $$2 }' $(KEYDIR)/module.txt > $(KEYDIR)/module.nk

docker:
	cp -r $(DEBUG)/gluon_actor_signed.wasm ../docker-output/debug/
