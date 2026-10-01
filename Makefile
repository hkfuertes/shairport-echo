VERSION ?= 0.2.1
ECHO_LIBS := third_party/echo-libs

.PHONY: all build zip test verify update-speakerd clean

all build: zip

zip:
	VERSION=$(VERSION) ./twrp/build.sh

test: zip
	VERSION=$(VERSION) ./tests/twrp_zip.sh

verify: test

# Maintenance only: rebuild speakerd from the pinned echo-libs submodule, then record it in libs/README.md.
update-speakerd:
	$(MAKE) -C $(ECHO_LIBS) speakerd
	cp $(ECHO_LIBS)/dist/armv7-unknown-linux-musleabihf/speakerd libs/
	@git -C $(ECHO_LIBS) log --oneline -1; sha256sum libs/speakerd

clean:
	rm -rf out
