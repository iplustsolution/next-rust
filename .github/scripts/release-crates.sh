#!/usr/bin/env bash
# The crates released to crates.io, in dependency order: every crate comes
# after the crates it uses. The one list CI and the Release workflow share.
#
#   release-crates.sh list    print the crates, one per line, in publish order
#   release-crates.sh check   fail unless the list is exactly the publishable
#                             workspace crates, in a valid order, sharing one
#                             version, each with the metadata crates.io needs
set -euo pipefail

crates=(
  next-rust-assets
  next-rust-core
  next-rust-router
  next-rust-view
  next-rust-icons
  next-rust-ui
  next-rust-cache
  next-rust-server
  next-rust-macros
  next-rust-jobs
  next-rust-auth
  next-rust-build
  next-rust
  next-rust-cli
  # The browser side. `next-rust-wasm` is the host ABI and depends on
  # nothing; `next-rust-ui-wasm` is the components' behaviour and depends on
  # it. `next-rust-runtime` is deliberately absent: it sets `publish = false`
  # because it is checked in pre-built as `wasm_runtime/runtime.wasm`.
  next-rust-wasm
  next-rust-ui-wasm
)

error() {
  if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::error::$*"; else echo "error: $*" >&2; fi
  failed=1
}

case "${1:-list}" in
  list)
    printf '%s\n' "${crates[@]}"
    ;;
  check)
    failed=0
    metadata=$(cargo metadata --no-deps --format-version 1)

    publishable=$(echo "$metadata" | jq -r '.packages[] | select(.publish != []) | .name' | sort | xargs)
    listed=$(printf '%s\n' "${crates[@]}" | sort | xargs)
    if [ "$publishable" != "$listed" ]; then
      error "the publishable crates ($publishable) don't match the list in .github/scripts/release-crates.sh ($listed)"
    fi

    versions=$(echo "$metadata" | jq -r '.packages[] | select(.publish != []) | .version' | sort -u | xargs)
    if [ "$(echo "$versions" | wc -w)" -ne 1 ]; then
      error "all crates must share one version (use version.workspace = true); found: $versions"
    fi

    # A crate is published after every workspace crate it depends on (dev-dependencies
    # are not needed on crates.io first), and each of those is required by version,
    # not by path alone: `cargo publish` rejects a path-only dependency.
    seen=" "
    for crate in "${crates[@]}"; do
      while IFS=$'\t' read -r dep kind req; do
        [ -n "$dep" ] || continue
        if [ "$req" = "*" ]; then
          error "$crate depends on $dep by path only; add version = \"…\" (see [workspace.dependencies])"
        fi
        if [ "$kind" != "dev" ] && [[ "$seen" != *" $dep "* ]]; then
          error "$crate is listed before its dependency $dep"
        fi
      done < <(echo "$metadata" | jq -r --arg c "$crate" '
        .packages[] | select(.name == $c) | .dependencies[]
        | select(.path != null) | [.name, (.kind // "normal"), .req] | @tsv')
      seen="$seen$crate "
    done

    while IFS=$'\t' read -r name missing; do
      [ -n "$name" ] || continue
      error "$name is missing package metadata: $missing"
    done < <(echo "$metadata" | jq -r '
      .packages[] | select(.publish != [])
      | . as $p
      | [ (select(($p.description // "") == "") | "description"),
          (select(($p.license // "") == "" and ($p.license_file // "") == "") | "license"),
          (select(($p.repository // "") == "") | "repository"),
          (select(($p.readme // "") == "") | "readme"),
          (select(($p.keywords | length) > 5) | "at most 5 keywords"),
          (select(($p.categories | length) > 5) | "at most 5 categories") ]
      | select(length > 0) | [$p.name, join(", ")] | @tsv')

    [ "$failed" -eq 0 ] || exit 1
    echo "release list ok: ${#crates[@]} crates at version $versions"
    ;;
  *)
    echo "usage: $0 [list|check]" >&2
    exit 2
    ;;
esac
