#!/bin/sh

if ! eval "$1"; then
  echo "$2" >&2
  exit 1
fi
