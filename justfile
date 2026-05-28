default:
    dx serve --port 5460

build:
    dx build --release

# Open an interactive SQL shell to the local database
db-shell:
    surreal sql --endpoint surrealkv://quivrs.db --namespace quivrs --database quivrs

# Check the status of the database tables and indexes
db-info:
    surreal sql --endpoint surrealkv://quivrs.db --namespace quivrs --database quivrs <<< "INFO FOR DB;"

# Create a new migration file (usage: just migration <name>)
migration name:
    touch migrations/$(date +%Y%m%d)_"{{ name }}".surql
    echo "Created migrations/$(date +%Y%m%d)_{{ name }}.surql"

# Fetch a rounded Material Symbol SVG from Google's repo into assets/icons/materialsymbolsrounded/<name>.svg
fetch-icon name:
    curl -L --fail "https://chromium.googlesource.com/external/github.com/google/material-design-icons/+/master/symbols/web/{{ name }}/materialsymbolsrounded/{{ name }}_wght700fill1_48px.svg?format=TEXT" | base64 -d > "assets/icons/{{ name }}.svg"
