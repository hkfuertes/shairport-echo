VERSION ?= 0.2.1

.PHONY: all build zip test verify update-speakerd clean

all build: zip

zip:
	VERSION=$(VERSION) ./twrp/build.sh

test: zip
	VERSION=$(VERSION) ./tests/twrp_zip.sh

verify: test

# Maintenance only: rebuild the sibling project's speakerd and copy the binary, never source.
update-speakerd:
	$(MAKE) -C ../echo-libs speakerd
	cp ../echo-libs/dist/armv7-unknown-linux-musleabihf/speakerd libs/
	@echo "record this echo-libs commit in libs/README.md:"; git -C ../echo-libs log --oneline -1; sha256sum libs/speakerd

clean:
	rm -rf out
