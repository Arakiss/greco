#!/bin/sh
set -eu
printf '%s\n' "intentional failure" >&2
exit 1
