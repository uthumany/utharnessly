#!/bin/sh
if [ "$1" != agents ] || [ "$2" != run ]; then
  echo 'Wrong route: workspace tasks require agents run' >&2
  exit 2
fi
if [ "$3" = fail ]; then
  echo 'Partial output is not success'
  echo 'Provider rejected request (HTTP 401)' >&2
  exit 1
fi
printf '01 list_directory · Allow\n   README.md\n   Cargo.toml\nCompleted 1 approved read-only step(s).\n'
