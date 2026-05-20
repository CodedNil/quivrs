CREATE TABLE IF NOT EXISTS articles (
    id               TEXT PRIMARY KEY,
    title            TEXT NOT NULL,
    sources          BLOB NOT NULL,
    estimated_liked  REAL NOT NULL DEFAULT 0.0,
    entry            BLOB,
    embedding        BLOB NOT NULL,
    embedding_model  TEXT NOT NULL DEFAULT '',
    category         TEXT NOT NULL DEFAULT 'Technology',
    published        INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS published ON articles(published);

CREATE TABLE IF NOT EXISTS article_urls (
    url TEXT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS user_articles (
    article_id TEXT PRIMARY KEY,
    status     TEXT NOT NULL DEFAULT 'New',
    binned_at  INTEGER
);

CREATE TABLE IF NOT EXISTS article_ratings (
    article_id TEXT PRIMARY KEY,
    rating     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS item_ratings (
    key    TEXT PRIMARY KEY,
    rating TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS label_embeddings (
    key       TEXT PRIMARY KEY,
    hash      TEXT NOT NULL,
    embedding BLOB NOT NULL
);
