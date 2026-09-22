#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
"${CC:-cc}" -std=c11 -Wall -Wextra -Werror -fsyntax-only \
  -I"$root/include" "$root/tests/echo_alsa_ffi_header.c"
