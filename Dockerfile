# syntax=docker/dockerfile:1
FROM android-armv7-r27c-research:latest AS build

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

ENV NDK=/opt/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64 \
    PREFIX=/opt/armv7-android \
    HOST=arm-linux-androideabi \
    PATH=/opt/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

RUN apt-get update && apt-get install -y --no-install-recommends \
      autoconf automake autopoint bison bzip2 ca-certificates cmake curl flex gettext libplist-utils libtool make patch perl pkg-config python3 texinfo xxd xz-utils \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL "https://www.alsa-project.org/files/pub/lib/alsa-lib-${ALSA_VERSION}.tar.bz2" -o /tmp/alsa.tar.bz2 \
    && echo "${ALSA_SHA256}  /tmp/alsa.tar.bz2" | sha256sum -c - \
    && mkdir /src \
    && tar -xjf /tmp/alsa.tar.bz2 -C /src \
    && rm /tmp/alsa.tar.bz2

WORKDIR /src/alsa-lib-${ALSA_VERSION}
RUN export CC="$NDK/bin/armv7a-linux-androideabi24-clang" \
           AR="$NDK/bin/llvm-ar" \
           RANLIB="$NDK/bin/llvm-ranlib" \
           STRIP="$NDK/bin/llvm-strip" \
    && ac_cv_header_sys_shm_h=no ./configure --build=x86_64-pc-linux-gnu --host="$HOST" --prefix="$PREFIX" \
         --disable-shared --enable-static \
         --with-ctl-plugins=remap,ext --with-pcm-plugins=copy,linear,route,plug \
    && make -j"$(nproc)" \
    && make install

COPY tools/alsa-open-probe.c /src/alsa-open-probe.c
RUN mkdir -p /out \
    && "$NDK/bin/armv7a-linux-androideabi24-clang" -O2 -fPIE -pie \
      -I"$PREFIX/include" /src/alsa-open-probe.c -L"$PREFIX/lib" \
      -lasound -ldl -lm -o /out/alsa-open-probe \
    && "$NDK/bin/llvm-readelf" -h /out/alsa-open-probe | grep -q 'Machine:.*ARM' \
    && "$NDK/bin/llvm-readelf" -d /out/alsa-open-probe | grep -q 'Shared library: \[libc.so\]'

FROM scratch AS artifact
COPY --from=build /out/alsa-open-probe /alsa-open-probe
COPY config/echo-alsa.conf /echo-alsa.conf

FROM build AS uuid-build
RUN curl -fsSL "https://www.kernel.org/pub/linux/utils/util-linux/v${UTIL_LINUX_VERSION%.*}/util-linux-${UTIL_LINUX_VERSION}.tar.xz" -o /tmp/util-linux.tar.xz \
    && echo "${UTIL_LINUX_SHA256}  /tmp/util-linux.tar.xz" | sha256sum -c - \
    && tar -xJf /tmp/util-linux.tar.xz -C /src \
    && rm /tmp/util-linux.tar.xz
WORKDIR /src/util-linux-${UTIL_LINUX_VERSION}
RUN CC="$NDK/bin/armv7a-linux-androideabi24-clang" \
       AR="$NDK/bin/llvm-ar" \
       RANLIB="$NDK/bin/llvm-ranlib" \
       ./configure --build=x86_64-pc-linux-gnu --host="$HOST" --prefix="$PREFIX" \
         --disable-all-programs --enable-libuuid --disable-shared --enable-static --disable-year2038 \
    && make -j"$(nproc)" \
    && make install \
    && test -f "$PREFIX/lib/libuuid.a"

FROM scratch AS uuid-artifact
COPY --from=uuid-build /opt/armv7-android/lib/libuuid.a /libuuid.a

FROM uuid-build AS shairport-deps
ENV CC=$NDK/bin/armv7a-linux-androideabi24-clang \
    CXX=$NDK/bin/armv7a-linux-androideabi24-clang++ \
    AR=$NDK/bin/llvm-ar \
    RANLIB=$NDK/bin/llvm-ranlib \
    STRIP=$NDK/bin/llvm-strip \
    CFLAGS=-O2\ -fPIC \
    PKG_CONFIG_LIBDIR=/opt/armv7-android/lib/pkgconfig \
    PKG_CONFIG_PATH=/opt/armv7-android/lib/pkgconfig
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
# Bionic advertises glob.h but does not export glob; popt's no-glob fallback is sufficient here.
RUN cd /src/popt \
    && autoreconf -fi \
    && ac_cv_header_glob_h=no ./configure --build=x86_64-pc-linux-gnu --host="$HOST" --prefix="$PREFIX" --disable-shared --enable-static \
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
    && ANDROID_NDK_ROOT=/opt/android-ndk-r27c ./Configure android-arm -D__ANDROID_API__=24 no-shared no-tests no-zlib --prefix="$PREFIX" --openssldir="$PREFIX/ssl" \
    && make -j"$(nproc)" build_libs && make install_sw
RUN cd /src/ffmpeg \
    && ./configure --prefix="$PREFIX" --target-os=android --arch=arm --cpu=armv7-a --enable-cross-compile \
         --cc="$CC" --ar="$AR" --ranlib="$RANLIB" --strip="$STRIP" \
         --disable-shared --enable-static --disable-programs --disable-doc --disable-debug --disable-everything \
         --disable-avdevice --disable-avfilter --disable-swscale --disable-network \
         --disable-jni --disable-mediacodec \
         --disable-zlib --disable-bzlib --disable-lzma --disable-iconv --disable-sdl2 \
         --enable-decoder=alac --enable-decoder=aac --enable-parser=aac --enable-protocol=file --enable-small \
         --extra-libs='-lm -ldl' \
    && make -j"$(nproc)" && make install \
    && sed -i 's/ -landroid -lmediandk//g' "$PREFIX/lib/pkgconfig/libavutil.pc" \
    && test -f "$PREFIX/lib/libavcodec.a"

FROM shairport-deps AS shairport-build
COPY third_party/shairport-sync /src/shairport-sync
COPY patches/shairport-sync /patches/shairport-sync
WORKDIR /src/shairport-sync
RUN for patch in /patches/shairport-sync/*.patch; do patch -p1 < "$patch"; done \
    && autoreconf -fi \
    && mkdir build && cd build \
    && PKG_CONFIG='pkg-config --static' \
       CPPFLAGS="-I$PREFIX/include" \
       LDFLAGS="-L$PREFIX/lib -fPIE -pie -static-libstdc++" \
       LIBS='-ldl -lm' \
       ../configure --build=x86_64-pc-linux-gnu --host="$HOST" \
         --with-airplay-2 --with-alsa --with-tinysvcmdns --with-ssl=openssl \
    && make -j"$(nproc)" \
    && "$NDK/bin/llvm-readelf" -h shairport-sync | grep -q 'Machine:.*ARM' \
    && "$NDK/bin/llvm-readelf" -d shairport-sync | grep -q 'Shared library: \[libc.so\]'

FROM scratch AS shairport-artifact
COPY --from=shairport-build /src/shairport-sync/build/shairport-sync /shairport-sync
COPY config/echo-alsa.conf /echo-alsa.conf
COPY config/echo-shairport-sync.conf /echo-shairport-sync.conf
COPY scripts/echo-airplay.sh /echo-airplay

FROM build AS nqptp-build
COPY third_party/nqptp /src/nqptp
COPY patches/nqptp /patches/nqptp
WORKDIR /src/nqptp
RUN for patch in /patches/nqptp/*.patch; do patch -p1 < "$patch"; done \
    && autoreconf -fi \
    && CC="$NDK/bin/armv7a-linux-androideabi24-clang" \
       AR="$NDK/bin/llvm-ar" \
       RANLIB="$NDK/bin/llvm-ranlib" \
       STRIP="$NDK/bin/llvm-strip" \
       ac_cv_func_malloc_0_nonnull=yes \
       ./configure --build=x86_64-pc-linux-gnu --host="$HOST" \
    && make -j"$(nproc)" \
    && "$NDK/bin/llvm-readelf" -h nqptp | grep -q 'Machine:.*ARM' \
    && "$NDK/bin/llvm-readelf" -d nqptp | grep -q 'Shared library: \[libc.so\]'

FROM scratch AS nqptp-artifact
COPY --from=nqptp-build /src/nqptp/nqptp /nqptp
