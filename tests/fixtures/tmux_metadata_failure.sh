#!/usr/bin/env bash

case "${1:-}" in
  new-window)
    printf '@99\n'
    ;;
  set-option)
    exit 1
    ;;
  kill-window)
    ;;
  *)
    exit 1
    ;;
esac
