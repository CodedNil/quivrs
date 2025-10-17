# Quivrs

Quivrs is a Rust-based service that ingests RSS-style feeds, applies configurable filtering and summaries via LLM prompts, cleaning up the content and titles (especially useful for twitter feeds). It declaratively configures your Miniflux feeds, making it easy to keep your reader in sync with curated sources.

## Features

- Refreshes multiple feeds defined in `feeds.json`, storing normalised data in a Redb database.
- Supports website, YouTube, Twitter, and Reddit sources with source-specific handling.
- Allows per feed filtering in plain english, to filter out unwanted content.
- Can condense feeds to timed summaries instead of individual articles.
- Uses OpenRouter so backend LLM can be configured.

Configuration defaults to `feeds.json`, and the local database path can be set with the `DATABASE_URL` environment variable.
