TARGET_DIR=~/apps

.PHONY: release
release:
	cargo build --release --target=armv7-unknown-linux-gnueabihf


.PHONY: deploy
deploy: release
	ssh rmusb "rm -f apps/rm_game_of_life"
	scp target/armv7-unknown-linux-gnueabihf/release/rm_game_of_life rmusb:$(TARGET_DIR)

.PHONY: clean
clean:
	rm -rf target
