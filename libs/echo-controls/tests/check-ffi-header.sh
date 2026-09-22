#!/bin/sh
set -eu

cd "$(dirname "$0")/.."
cc -std=c11 -Wall -Wextra -Werror -fsyntax-only tests/echo_controls_ffi_header.c
