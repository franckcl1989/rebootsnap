#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

git -C "$ROOT" config core.hooksPath .githooks

printf 'Configured git core.hooksPath=.githooks\n'
printf 'Run scripts/verify.sh before committing.\n'
