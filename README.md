# Quivrs

Quivrs is a small Rust service that keeps Miniflux categories and feeds in sync with `feeds.json`.

## Features

- Syncs feeds defined in `feeds.json` once, then exits.
- Supports direct RSS feeds with `name/feed-url` entries.
- Resolves YouTube handles to channel RSS URLs.
- Resolves Twitter and Bluesky handles to FxEmbed Atom feeds.
- Keeps Miniflux declarative, deleting feeds and categories that are no longer configured.

## Configuration

Each top-level key is a Miniflux category. Each feed is a string split on the first slash:

```json
{
  "Test": [
    "bbc_tech/https://feeds.bbci.co.uk/news/technology/rss.xml",
    "youtube/veritasium",
    "twitter/ArtificialAnlys",
    "bluesky/user.bsky.social"
  ]
}
```

`youtube`, `twitter`, and `bluesky` are source keywords. `twitter` and `bluesky` use FxEmbed Atom feeds. Any other first segment is treated as a direct-feed label; it is only used for logging.

Configuration defaults to `feeds.json`. Set `MINIFLUX_URL` and `MINIFLUX_KEY` for the Miniflux API.
