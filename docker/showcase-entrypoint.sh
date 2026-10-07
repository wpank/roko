#!/usr/bin/env bash
# The showcase image's entrypoint (S11 §4.6), run by tini as root.
#
# It refuses to start without an Argon2id passphrase hash, keeps state on the volume (the
# workspace's .roko links to ROKO_STATE_ROOT), and runs `roko serve` as the unprivileged roko
# user. F1 has no live slices, so no capabilities are kept.
set -Eeuo pipefail

APP_USER="${APP_USER:-roko}"
STATE_ROOT="${ROKO_STATE_ROOT:-/data/.roko}"
WORKDIR="${ROKO_WORKDIR:-/workspace}"

log() {
  printf '[showcase] %s\n' "$*" >&2
}

# 1. Without an Argon2id passphrase hash, nobody could log in (and serve refuses to start).
case "${ROKO_SHOWCASE_PASSPHRASE_HASH:-}" in
  '$argon2id$'*) ;;
  *)
    log 'ROKO_SHOWCASE_PASSPHRASE_HASH must hold an Argon2id PHC string ($argon2id$...)'
    exit 1
    ;;
esac

# 2. State on the volume: bundles, sessions, the hold file.
mkdir -p "${STATE_ROOT}/showcase/bundles"
chown "${APP_USER}:${APP_USER}" "${STATE_ROOT}"
chown -R "${APP_USER}:${APP_USER}" "${STATE_ROOT}/showcase"
if [ ! -L "${WORKDIR}/.roko" ]; then
  rm -rf "${WORKDIR}/.roko"
  ln -s "${STATE_ROOT}" "${WORKDIR}/.roko"
fi

# The Fly environment names the public origin; the hierarchical override beats the image config.
if [ -n "${ROKO_SHOWCASE_PUBLIC_ORIGIN:-}" ]; then
  export ROKO__SHOWCASE__PUBLIC_ORIGIN="${ROKO_SHOWCASE_PUBLIC_ORIGIN}"
fi

# 3. Serve as roko. An idle exit (0) leaves the Machine stopped until the next request.
cd "${WORKDIR}"
log "starting roko serve (state in ${STATE_ROOT})"
exec gosu "${APP_USER}" roko serve
