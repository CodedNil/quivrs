default:
    dx serve --port 5460

build:
    dx build --release

# Open an interactive SQL shell to the local database
db-shell:
    surreal sql --endpoint surrealkv://quivrs.db --namespace quivrs --database quivrs

# Check the status of the database tables and indexes
db-info:
    @surreal sql --endpoint surrealkv://quivrs.db --namespace quivrs --database quivrs <<< "INFO FOR DB;"

# Create a new migration file (usage: just migration <name>)
migration name:
    @touch migrations/$(date +%Y%m%d)_"{{ name }}".surql
    @echo "Created migrations/$(date +%Y%m%d)_{{ name }}.surql"
