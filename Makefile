TARGET_DIR=~/apps

SHELL := /bin/bash

.PHONY: release
release:
	source /opt/codex/rm11x/3.1.2/environment-setup-cortexa7hf-neon-remarkable-linux-gnueabi && cargo build --release --target=armv7-unknown-linux-gnueabihf


.PHONY: deploy
deploy: release
	ssh rmusb "rm -f apps/rm_game_of_life"
	scp target/armv7-unknown-linux-gnueabihf/release/rm_game_of_life rmusb:$(TARGET_DIR)

.PHONY: clean
clean:
	rm -rf target
