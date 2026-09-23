#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "Usage: packaging/release/release-notes.sh TAG" >&2
    exit 2
fi

tag="$1"

annotation() {
    if [ "$(git cat-file -t "$tag")" = tag ]; then
        git tag -l --format='%(contents)' "$tag" | sed '/^-----BEGIN PGP SIGNATURE-----$/,$d'
    fi
}

meaningful() {
    local text
    text="$(printf '%s' "$1" | tr -d '[:space:]')"
    [ -n "$text" ] && [ "$text" != "$tag" ] && [ "$text" != "${tag#v}" ]
}

previous_tag() {
    git describe --tags --abbrev=0 --match 'v*.*.*' "$tag^" 2>/dev/null || true
}

commit_range() {
    local previous
    previous="$(previous_tag)"
    if [ -n "$previous" ]; then
        printf '%s..%s' "$previous" "$tag"
    else
        printf '%s' "$tag"
    fi
}

group_commits() {
    awk '
        function heading(type) {
            if (type == "feat") return "Features"
            if (type == "fix") return "Fixes"
            if (type == "perf") return "Performance"
            if (type == "refactor") return "Refactoring"
            if (type == "docs") return "Documentation"
            if (type ~ /^(chore|ci|build|test|style)$/) return "Maintenance"
            return "Other changes"
        }
        {
            hash = $1
            subject = substr($0, length(hash) + 2)
            type = "other"
            if (match(subject, /^[a-z]+(\([^)]*\))?!?: /)) {
                prefix = substr(subject, 1, RLENGTH - 2)
                subject = substr(subject, RLENGTH + 1)
                breaking = prefix ~ /!$/
                sub(/!$/, "", prefix)
                type = prefix
                scope = ""
                if (match(prefix, /\(.*\)/)) {
                    scope = substr(prefix, RSTART + 1, RLENGTH - 2)
                    type = substr(prefix, 1, RSTART - 1)
                }
                if (scope != "") subject = "**" scope "**: " subject
                if (breaking) subject = "**BREAKING** " subject
            }
            group = heading(type)
            lines[group] = lines[group] "- " subject " (" hash ")\n"
        }
        END {
            split("Features Fixes Performance Refactoring Documentation Maintenance", order, " ")
            order[7] = "Other changes"
            for (i = 1; i <= 7; i++) {
                if (order[i] in lines) printf "### %s\n\n%s\n", order[i], lines[order[i]]
            }
        }
    '
}

notes="$(annotation)"
if meaningful "$notes"; then
    printf '%s\n' "$notes"
else
    printf '## Changes\n\n'
    git log --no-merges --format='%h %s' "$(commit_range)" | group_commits
fi
