# syntax=docker/dockerfile:1
# Fully static armv7 musl build: real pthread_cancel, no Bionic compatibility patches.
FROM rust:1.98-bookworm@sha256:93ce27a88655056a51dbdd8f5f2d7ddc071c7b0070fb288a37b5a285fc83971e AS build

ARG MUSL_TOOLCHAIN_SHA256=f49f1a15ec62364ef5e4edb4e3990c0e1d2d1a54c90153b8f3869dad63328a10

ARG ALSA_VERSION=1.2.14
ARG ALSA_SHA256=be9c88a0b3604367dd74167a2b754a35e142f670292ae47a2fdef27a2ee97a32
ARG UTIL_LINUX_VERSION=2.40.4
ARG UTIL_LINUX_SHA256=5c1daf733b04e9859afdc3bd87cc481180ee0f88b5c0946b16fdec931975fb79
ARG POPT_SHA256=6eb40d650526cb9fe63eb4415bcecdf9cf306f7556e77eff689abc5a44670060
ARG LIBCONFIG_SHA256=e95798d2992a66ecd547ce3651d7e10642ff2211427c43a7238186ff4c372627
ARG LIBSODIUM_SHA256=ebb65ef6ca439333c2bb41a0c1990587288da07f6c7fd07cb3a18cc18d30ce19
ARG LIBGPG_ERROR_SHA256=be0f1b2db6b93eed55369cdf79f19f72750c8c7c39fc20b577e724545427e6b2
ARG LIBGCRYPT_SHA256=8b0870897ac5ac67ded568dcfadf45969cfa8a6beb0fd60af2a9eadc2a3272aa
ARG LIBPLIST_SHA256=7ac42301e896b1ebe3c654634780c82baa7cb70df8554e683ff89f7c2643eb8b
ARG OPENSSL_SHA256=23c666d0edf20f14249b3d8f0368acaee9ab585b09e1de82107c66e1f3ec9533
ARG FFMPEG_SHA256=9fd092511605bbebafe095ea6d38d9e40f34d12f7386e1258372df8be0576eb7

# musl.cc's armv7l toolchain defaults to armv5te; the Echo is a Cortex-A53 (neon, vfpv4).
ENV PREFIX=/opt/armv7-musl \
    HOST=armv7l-linux-musleabihf \
    CC=armv7l-linux-musleabihf-gcc \
    CXX=armv7l-linux-musleabihf-g++ \
    AR=armv7l-linux-musleabihf-ar \
    RANLIB=armv7l-linux-musleabihf-ranlib \
    STRIP=armv7l-linux-musleabihf-strip \
    READELF=armv7l-linux-musleabihf-readelf \
    CFLAGS=-O3\ -fPIC\ -march=armv7-a\ -mfpu=neon-vfpv4\ -mtune=cortex-a53 \
    CXXFLAGS=-O3\ -fPIC\ -march=armv7-a\ -mfpu=neon-vfpv4\ -mtune=cortex-a53 \
    PKG_CONFIG_LIBDIR=/opt/armv7-musl/lib/pkgconfig \
    PKG_CONFIG_PATH=/opt/armv7-musl/lib/pkgconfig \
    CARGO_PROFILE_RELEASE_OPT_LEVEL=3 \
    CARGO_TARGET_ARMV7_UNKNOWN_LINUX_MUSLEABIHF_LINKER=armv7l-linux-musleabihf-gcc \
    PATH=/opt/armv7l-linux-musleabihf-cross/bin:/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

RUN apt-get update && apt-get install -y --no-install-recommends \
      autoconf automake autopoint bison bzip2 ca-certificates cmake curl flex gettext libplist-utils libtool make patch perl pkg-config python3 texinfo xxd xz-utils \
    && rm -rf /var/lib/apt/lists/* \
    && curl -fsSL https://musl.cc/armv7l-linux-musleabihf-cross.tgz -o /tmp/musl.tgz \
    && echo "${MUSL_TOOLCHAIN_SHA256}  /tmp/musl.tgz" | sha256sum -c - \
    && tar -xzf /tmp/musl.tgz -C /opt \
    && rm /tmp/musl.tgz \
    && rustup target add armv7-unknown-linux-musleabihf

# Static binaries only: a NEEDED entry would mean a dependency on Android's dynamic libraries.
RUN printf '#!/bin/sh\nset -e\nfor f; do "$READELF" -h "$f" | grep -q "Machine:.*ARM"; ! "$READELF" -d "$f" | grep -q NEEDED; done\n' \
      >/usr/local/bin/check-static && chmod 755 /usr/local/bin/check-static

RUN curl -fsSL "https://www.alsa-project.org/files/pub/lib/alsa-lib-${ALSA_VERSION}.tar.bz2" -o /tmp/alsa.tar.bz2 \
    && echo "${ALSA_SHA256}  /tmp/alsa.tar.bz2" | sha256sum -c - \
    && mkdir /src \
    && tar -xjf /tmp/alsa.tar.bz2 -C /src \
    && rm /tmp/alsa.tar.bz2

WORKDIR /src/alsa-lib-${ALSA_VERSION}
RUN ./configure --build=x86_64-pc-linux-gnu --host="$HOST" --prefix="$PREFIX" \
         --disable-shared --enable-static \
         --with-ctl-plugins=remap,ext --with-pcm-plugins=copy,linear,route,plug \
    && make -j"$(nproc)" \
    && make install

COPY tools/alsa-open-probe.c /src/alsa-open-probe.c
RUN mkdir -p /out \
    && "$CC" -O3 -static -I"$PREFIX/include" /src/alsa-open-probe.c -L"$PREFIX/lib" \
      -lasound -lm -o /out/alsa-open-probe \
    && check-static /out/alsa-open-probe

FROM scratch AS artifact
COPY --from=build /out/alsa-open-probe /alsa-open-probe
COPY config/echo-alsa.conf /echo-alsa.conf

FROM build AS controls-build
COPY libs/echo-controls /src/echo-controls
WORKDIR /src/echo-controls
RUN cargo build --locked --release --target armv7-unknown-linux-musleabihf \
    && "$CC" -O3 -static -Iinclude tests/echo_controls_ffi_link.c \
      target/armv7-unknown-linux-musleabihf/release/libecho_controls.a \
      -lm -o /tmp/echo-controls-ffi-link \
    && check-static /tmp/echo-controls-ffi-link

FROM scratch AS controls-artifact
COPY --from=controls-build /src/echo-controls/target/armv7-unknown-linux-musleabihf/release/libecho_controls.a /libecho_controls.a
COPY libs/echo-controls/include/echo_controls.h /include/echo_controls.h

FROM build AS uuid-build
RUN curl -fsSL "https://www.kernel.org/pub/linux/utils/util-linux/v${UTIL_LINUX_VERSION%.*}/util-linux-${UTIL_LINUX_VERSION}.tar.xz" -o /tmp/util-linux.tar.xz \
    && echo "${UTIL_LINUX_SHA256}  /tmp/util-linux.tar.xz" | sha256sum -c - \
    && tar -xJf /tmp/util-linux.tar.xz -C /src \
    && rm /tmp/util-linux.tar.xz
WORKDIR /src/util-linux-${UTIL_LINUX_VERSION}
RUN ./configure --build=x86_64-pc-linux-gnu --host="$HOST" --prefix="$PREFIX" \
         --disable-all-programs --enable-libuuid --disable-shared --enable-static --disable-year2038 \
    && make -j"$(nproc)" \
    && make install \
    && test -f "$PREFIX/lib/libuuid.a"

FROM scratch AS uuid-artifact
COPY --from=uuid-build /opt/armv7-musl/lib/libuuid.a /libuuid.a

FROM uuid-build AS shairport-deps
RUN mkdir -p /src/popt /src/libconfig /src/libsodium /src/libgpg-error /src/libgcrypt /src/libplist /src/openssl /src/ffmpeg \
    && curl -fsSL https://github.com/rpm-software-management/popt/archive/refs/tags/popt-1.19-release.tar.gz -o /tmp/popt.tar.gz \
    && echo "$POPT_SHA256  /tmp/popt.tar.gz" | sha256sum -c - \
    && tar -xzf /tmp/popt.tar.gz -C /src/popt --strip-components=1 \
    && curl -fsSL https://github.com/hyperrealm/libconfig/archive/refs/tags/v1.8.1.tar.gz -o /tmp/libconfig.tar.gz \
    && echo "$LIBCONFIG_SHA256  /tmp/libconfig.tar.gz" | sha256sum -c - \
    && tar -xzf /tmp/libconfig.tar.gz -C /src/libconfig --strip-components=1 \
    && curl -fsSL https://download.libsodium.org/libsodium/releases/libsodium-1.0.20.tar.gz -o /tmp/libsodium.tar.gz \
    && echo "$LIBSODIUM_SHA256  /tmp/libsodium.tar.gz" | sha256sum -c - \
    && tar -xzf /tmp/libsodium.tar.gz -C /src/libsodium --strip-components=1 \
    && curl -fsSL https://gnupg.org/ftp/gcrypt/libgpg-error/libgpg-error-1.51.tar.bz2 -o /tmp/libgpg-error.tar.bz2 \
    && echo "$LIBGPG_ERROR_SHA256  /tmp/libgpg-error.tar.bz2" | sha256sum -c - \
    && tar -xjf /tmp/libgpg-error.tar.bz2 -C /src/libgpg-error --strip-components=1 \
    && curl -fsSL https://gnupg.org/ftp/gcrypt/libgcrypt/libgcrypt-1.10.3.tar.bz2 -o /tmp/libgcrypt.tar.bz2 \
    && echo "$LIBGCRYPT_SHA256  /tmp/libgcrypt.tar.bz2" | sha256sum -c - \
    && tar -xjf /tmp/libgcrypt.tar.bz2 -C /src/libgcrypt --strip-components=1 \
    && curl -fsSL https://github.com/libimobiledevice/libplist/releases/download/2.7.0/libplist-2.7.0.tar.bz2 -o /tmp/libplist.tar.bz2 \
    && echo "$LIBPLIST_SHA256  /tmp/libplist.tar.bz2" | sha256sum -c - \
    && tar -xjf /tmp/libplist.tar.bz2 -C /src/libplist --strip-components=1 \
    && curl -fsSL https://www.openssl.org/source/openssl-3.0.15.tar.gz -o /tmp/openssl.tar.gz \
    && echo "$OPENSSL_SHA256  /tmp/openssl.tar.gz" | sha256sum -c - \
    && tar -xzf /tmp/openssl.tar.gz -C /src/openssl --strip-components=1 \
    && curl -fsSL https://github.com/FFmpeg/FFmpeg/archive/refs/tags/n8.1.2.tar.gz -o /tmp/ffmpeg.tar.gz \
    && echo "$FFMPEG_SHA256  /tmp/ffmpeg.tar.gz" | sha256sum -c - \
    && tar -xzf /tmp/ffmpeg.tar.gz -C /src/ffmpeg --strip-components=1 \
    && rm -f /tmp/*.tar.*
RUN cd /src/popt \
    && autoreconf -fi \
    && ./configure --build=x86_64-pc-linux-gnu --host="$HOST" --prefix="$PREFIX" --disable-shared --enable-static \
    && make -j"$(nproc)" && make install
RUN cd /src/libconfig \
    && autoreconf -fi \
    && ./configure --build=x86_64-pc-linux-gnu --host="$HOST" --prefix="$PREFIX" --disable-cxx --disable-shared --enable-static \
    && make -j"$(nproc)" && make install
RUN cd /src/libsodium \
    && ./configure --build=x86_64-pc-linux-gnu --host="$HOST" --prefix="$PREFIX" --disable-shared --enable-static \
    && make -j"$(nproc)" && make install
RUN cd /src/libgpg-error \
    && ./configure --build=x86_64-pc-linux-gnu --host="$HOST" --prefix="$PREFIX" --disable-shared --enable-static --disable-nls --disable-doc \
    && make -j"$(nproc)" && make install
RUN cd /src/libgcrypt \
    && PATH="$PREFIX/bin:$PATH" ./configure --build=x86_64-pc-linux-gnu --host="$HOST" --prefix="$PREFIX" --disable-shared --enable-static --disable-doc --disable-tests --disable-asm \
    && make -j"$(nproc)" && make install
RUN cd /src/libplist \
    && ./configure --build=x86_64-pc-linux-gnu --host="$HOST" --prefix="$PREFIX" --disable-shared --enable-static --without-cython \
    && make -j"$(nproc)" && make install
RUN cd /src/openssl \
    && ./Configure linux-armv4 no-shared no-tests no-zlib --prefix="$PREFIX" --openssldir="$PREFIX/ssl" \
    && make -j"$(nproc)" build_libs && make install_sw
RUN cd /src/ffmpeg \
    && ./configure --prefix="$PREFIX" --target-os=linux --arch=arm --enable-cross-compile \
         --cc="$CC" --ar="$AR" --ranlib="$RANLIB" --strip="$STRIP" \
         --disable-shared --enable-static --disable-programs --disable-doc --disable-debug --disable-everything \
         --disable-avdevice --disable-avfilter --disable-swscale --disable-network \
         --disable-zlib --disable-bzlib --disable-lzma --disable-iconv --disable-sdl2 \
         --enable-decoder=alac --enable-decoder=aac --enable-parser=aac --enable-protocol=file \
         --extra-libs='-lm' \
    && make -j"$(nproc)" && make install \
    && test -f "$PREFIX/lib/libavcodec.a"

FROM build AS echo-alsa-build
COPY libs/echo-alsa /src/echo-alsa
WORKDIR /src/echo-alsa
RUN cargo build --locked --release --target armv7-unknown-linux-musleabihf \
    && "$CC" -c tests/armv7_alsa_layout.c -o /tmp/armv7-alsa-layout.o \
    && "$CC" -Iinclude -c tests/echo_alsa_ffi_header.c -o /tmp/echo-alsa-ffi-header.o \
    && test -f target/armv7-unknown-linux-musleabihf/release/libecho_alsa.a

FROM build AS volume-control-build
COPY libs/echo-alsa /src/echo-alsa
COPY libs/echo-controls /src/echo-controls
COPY libs/echo-volume-control /src/echo-volume-control
WORKDIR /src/echo-volume-control
RUN cargo test --locked \
    && cargo build --locked --release --target armv7-unknown-linux-musleabihf \
    && "$STRIP" target/armv7-unknown-linux-musleabihf/release/echo-volume-control \
    && check-static target/armv7-unknown-linux-musleabihf/release/echo-volume-control

FROM shairport-deps AS shairport-build
COPY --from=echo-alsa-build /src/echo-alsa/target/armv7-unknown-linux-musleabihf/release/libecho_alsa.a /opt/armv7-musl/lib/libecho_alsa.a
COPY libs/echo-alsa/include/echo_alsa.h /opt/armv7-musl/include/echo_alsa.h
COPY third_party/shairport-sync /src/shairport-sync
COPY patches/shairport-sync /patches/shairport-sync
WORKDIR /src/shairport-sync
RUN for patch in /patches/shairport-sync/*.patch; do patch -p1 < "$patch"; done \
    && autoreconf -fi \
    && mkdir build && cd build \
    && PKG_CONFIG='pkg-config --static' \
       CPPFLAGS="-I$PREFIX/include" \
       LDFLAGS="-L$PREFIX/lib -static" \
       LIBS='-lm' \
       ../configure --build=x86_64-pc-linux-gnu --host="$HOST" \
         --with-airplay-2 --with-alsa --with-echo-alsa --with-metadata --with-metadata-multicast --with-tinysvcmdns --with-ssl=openssl \
    && make -j"$(nproc)" \
    && "$STRIP" shairport-sync && check-static shairport-sync

FROM scratch AS shairport-artifact
COPY --from=shairport-build /src/shairport-sync/build/shairport-sync /shairport-sync
COPY config/echo-alsa.conf /echo-alsa.conf
COPY config/echo-shairport-sync.conf /echo-shairport-sync.conf
COPY config/echo-shairport-sync-nosync.conf /echo-shairport-sync-nosync.conf
COPY config/echo-shairport-sync-echo.conf /echo-shairport-sync-echo.conf
COPY scripts/echo-airplay.sh /echo-airplay
COPY scripts/echo-route.sh /echo-route

FROM build AS nqptp-build
COPY third_party/nqptp /src/nqptp
WORKDIR /src/nqptp
RUN autoreconf -fi \
    && LDFLAGS=-static ac_cv_func_malloc_0_nonnull=yes ./configure --build=x86_64-pc-linux-gnu --host="$HOST" \
    && make -j"$(nproc)" \
    && "$STRIP" nqptp && check-static nqptp

FROM scratch AS nqptp-artifact
COPY --from=nqptp-build /src/nqptp/nqptp /nqptp

# Host round-trip check (no CAP_SYS_ADMIN here, so it must still rotate the seed), then the target build.
FROM build AS seed-build
COPY tools/entropy-seed.c /src/entropy-seed.c
RUN gcc -O2 -Wall -Wextra -Werror /src/entropy-seed.c -o /tmp/entropy-seed-host \
    && /tmp/entropy-seed-host /tmp/seed && a=$(sha256sum /tmp/seed) \
    && /tmp/entropy-seed-host /tmp/seed && [ "$a" != "$(sha256sum /tmp/seed)" ] && [ "$(stat -c %s /tmp/seed)" = 512 ] \
    && "$CC" -O2 -Wall -Wextra -Werror -static /src/entropy-seed.c -o /entropy-seed \
    && "$STRIP" /entropy-seed && check-static /entropy-seed

FROM scratch AS twrp-artifact
COPY --from=shairport-build /src/shairport-sync/build/shairport-sync /payload/system/lib/shairport-echo/shairport-sync
COPY --from=nqptp-build /src/nqptp/nqptp /payload/system/lib/shairport-echo/nqptp
COPY --from=seed-build /entropy-seed /payload/system/lib/shairport-echo/entropy-seed
COPY --from=volume-control-build /src/echo-volume-control/target/armv7-unknown-linux-musleabihf/release/echo-volume-control /payload/system/lib/shairport-echo/echo-volume-control
COPY --chmod=755 scripts/ledcontroller.sh /payload/system/bin/ledcontroller
COPY config/echo-alsa.conf /payload/system/lib/shairport-echo/echo-alsa.conf
COPY config/shairport-sync.conf /payload/system/lib/shairport-echo/shairport-sync.conf
