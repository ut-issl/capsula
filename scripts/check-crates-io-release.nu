#!/usr/bin/env nu

# crates.io's public API does not expose Trusted Publishing configurations to
# anonymous callers. `trustpub_only` is the closest public preflight signal: it
# proves that publishing is restricted to Trusted Publishing, but not that this
# particular workflow still has a matching configuration.
const CRATES_IO_API = "https://crates.io/api/v1/crates"

const REQUEST_HEADERS = {
    User-Agent: "capsula-release-precondition (https://github.com/ut-issl/capsula)"
    Accept: "application/json"
}

# Match `cargo publish --workspace` while targeting the default crates.io
# registry: workspace members with `publish = false` (represented as `[]` by
# `cargo metadata`) or a registry allowlist that excludes crates.io are skipped.
def crates-to-publish [] {
    let cargo_result = (^cargo metadata --no-deps --format-version 1 --locked | complete)

    if $cargo_result.exit_code != 0 {
        error make {
            msg: "failed to read Cargo workspace metadata"
            help: ($cargo_result.stderr | str trim)
        }
    }

    let metadata = $cargo_result.stdout | from json

    $metadata.packages
    | where {|package|
        ($package.id in $metadata.workspace_members) and (
            ($package.publish == null) or ("crates-io" in $package.publish)
        )
    }
    | select name version
    | sort-by name
}

def check-crate [package: record] {
    let url = $"($CRATES_IO_API)/($package.name)"
    let response = try {
        http get --allow-errors --full --max-time 30sec --headers $REQUEST_HEADERS $url
    } catch {|error| error make {
        msg: $"failed to query crates.io for `($package.name)`"
        help: $error.msg
    } }

    match $response.status {
        200 => {
            {
                crate: $package.name
                version: $package.version
                exists: true
                trustpub_only: ($response.body.crate.trustpub_only | default false)
            }
        }
        404 => {
            {
                crate: $package.name
                version: $package.version
                exists: false
                trustpub_only: false
            }
        }
        _ => {
            let detail = $response.body.errors?.detail? | default [] | str join "; "
            error make {
                msg: $"crates.io returned HTTP ($response.status) for `($package.name)`"
                help: $detail
            }
        }
    }
}

def main [] {
    let packages = (crates-to-publish)

    if ($packages | is-empty) {
        error make {msg: "cargo publish --workspace would not select any crates for crates.io"}
    }

    # crates.io's Data Access Policy limits API clients to one request/second.
    let results = ($packages | enumerate | each {|item|
        if $item.index > 0 {
            sleep 1sec
        }
        check-crate $item.item
    })

    print ($results | table)

    let missing = $results | where exists == false | get crate
    if not ($missing | is-empty) {
        error make {
            msg: "some crates selected by cargo publish --workspace do not exist on crates.io"
            help: $"Publish these crates manually first, then configure Trusted Publishing: ($missing | str join ', ')"
        }
    }

    let not_enforced = $results | where trustpub_only == false | get crate
    if not ($not_enforced | is-empty) {
        error make {
            msg: "some release crates do not require Trusted Publishing"
            help: $"Enable 'Require Trusted Publishing' in crates.io settings for: ($not_enforced | str join ', ')"
        }
    }

    print "All release crates exist on crates.io and require Trusted Publishing."
    print "Note: crates.io does not expose publisher configurations anonymously; token exchange in the publish job validates the matching workflow configuration."
}
