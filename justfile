set shell := ["bash", "-o", "pipefail", "-eu", "-c"]

default:
    dx serve --port 5460

build:
    dx build --release

# Open an interactive SQL shell to the local database
db-shell:
    db="${DATABASE_URL:-sqlite://quivrs.db}"; sqlite3 "${db#sqlite://}"

# Check the status of the database tables and indexes
db-info:
    db="${DATABASE_URL:-sqlite://quivrs.db}"; sqlite3 "${db#sqlite://}" ".tables"

# Create a new migration file (usage: just migration <name>)
migration name:
    touch migrations/$(date +%Y%m%d)_"{{ name }}".sql
    echo "Created migrations/$(date +%Y%m%d)_{{ name }}.sql"

# Fetch a rounded Material Symbol SVG from Google's repo into assets/icons/materialsymbolsrounded/<name>.svg
fetch-icon name:
    curl -L --fail "https://chromium.googlesource.com/external/github.com/google/material-design-icons/+/master/symbols/web/{{ name }}/materialsymbolsrounded/{{ name }}_wght700fill1_48px.svg?format=TEXT" | base64 -d > "assets/icons/{{ name }}.svg"

# Pull Kagi Kite feeds, strip display names, and apply feed/category blacklists
pull_kagi:
    tmp="$(mktemp)" && curl -L --fail "https://raw.githubusercontent.com/kagisearch/kite-public/main/kite_feeds.json" | jq --slurpfile config scripts/kagi_filters.json -f scripts/kagi_feeds.jq > "$tmp" && chmod 0644 "$tmp" && mv "$tmp" kite_feeds.json
