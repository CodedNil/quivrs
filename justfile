default:
    dx serve --port 5460

build:
    dx build --release

# Create and initialise a fresh local database
db:
    DATABASE_URL=sqlite:quivrs.db cargo sqlx database create
    DATABASE_URL=sqlite:quivrs.db cargo sqlx migrate run

# Regenerate the sqlx offline query cache after changing SQL queries or schema
prepare:
    DATABASE_URL=sqlite:quivrs.db cargo sqlx prepare -- --features server

# Apply pending migrations against the local database
migrate:
    DATABASE_URL=sqlite:quivrs.db cargo sqlx migrate run

# Create a new migration file (usage: just migration <name>)
migration name:
    cargo sqlx migrate add {{ name }}
