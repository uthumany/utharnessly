#!/bin/sh
if [ "$1" = models ] && [ "$2" = list ] && [ "$3" = --json ]; then
  printf '{"provider":"groq","active":"groq/model-a","models":["model-b","model-a"]}\n'
  exit 0
fi
if [ "$3" = wait ]; then
  exec sleep 60
fi
if [ "$3" = route ]; then
  printf '%s/%s\n' "$UTHARNESS_PROVIDER" "$UTHARNESS_MODEL"
  exit 0
fi
if [ "$1" = chat ]; then
  if [ "$2" = fail ]; then
    echo 'Chat request failed'
    echo 'Provider rejected request (HTTP 401)' >&2
    exit 1
  fi
  printf 'Uthy · groq/test-model\nHello there.\n'
  exit 0
fi
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
