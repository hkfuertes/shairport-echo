VERSION ?= 0.1.0

.PHONY: all build zip test verify clean

all build: zip

zip:
	VERSION=$(VERSION) ./twrp/build.sh

test: zip
	VERSION=$(VERSION) ./tests/twrp_zip.sh

verify: test

clean:
	rm -rf out
