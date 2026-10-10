#!/usr/bin/env nu

# dist builds the GitHub Release body from the CHANGELOG.md section whose
# heading starts with the released version. Fail before building anything if
# the hand-written release notes for this version are missing.

const CHANGELOG = "CHANGELOG.md"

def release-version [] {
    let cargo_result = (^cargo metadata --no-deps --format-version 1 --locked | complete)

    if $cargo_result.exit_code != 0 {
        error make {
            msg: "failed to read Cargo workspace metadata"
            help: ($cargo_result.stderr | str trim)
        }
    }

    $cargo_result.stdout | from json | get packages | where name == "capsula" | first | get version
}

# A release heading is `## v<version>`, matching the tag name, optionally followed by ` - <date>`.
def has-release-section [changelog: string, version: string] {
    $changelog
    | lines
    | any {|line| $line == $"## v($version)" or ($line | str starts-with $"## v($version) - ") }
}

def main [] {
    let version = (release-version)

    if not ($CHANGELOG | path exists) {
        error make {msg: $"($CHANGELOG) does not exist"}
    }

    if not (has-release-section (open --raw $CHANGELOG) $version) {
        error make {
            msg: $"($CHANGELOG) has no release notes for ($version)"
            help: $"Add a `## v($version)` section before releasing."
        }
    }

    print $"($CHANGELOG) has release notes for ($version)."
}
