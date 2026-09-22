# syntax=docker/dockerfile:1
FROM android-armv7-r27c-research:latest AS build

ARG ALSA_VERSION=1.2.14
ARG ALSA_SHA256=be9c88a0b3604367dd74167a2b754a35e142f670292ae47a2fdef27a2ee97a32

ENV NDK=/opt/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64 \
    PREFIX=/opt/armv7-android \
    HOST=arm-linux-androideabi \
    PATH=/opt/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

RUN apt-get update && apt-get install -y --no-install-recommends \
      autoconf automake bzip2 ca-certificates curl libtool make patch pkg-config \
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
