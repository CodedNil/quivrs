CREATE TABLE IF NOT EXISTS pending_sources (
    url             TEXT PRIMARY KEY,

    domain          TEXT NOT NULL,
    title           TEXT NOT NULL,
    summary         TEXT,
    content         TEXT,
    tags            TEXT,
    images          TEXT,
    published       INTEGER NOT NULL,
    category        TEXT NOT NULL,

    embedding       BLOB NOT NULL,
    embedding_text  TEXT NOT NULL,
    embedding_model TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS article_urls (
    url TEXT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS user_articles (
    id      TEXT PRIMARY KEY,

    sources         TEXT NOT NULL,
    title           TEXT NOT NULL,
    description     TEXT NOT NULL,
    content         TEXT NOT NULL,
    sidebar         TEXT NOT NULL,
    thumbnail       TEXT NOT NULL,
    published       INTEGER NOT NULL,
    category        TEXT NOT NULL,

    status          TEXT NOT NULL,
    binned_at       INTEGER,
    rating          TEXT,

    embedding       BLOB NOT NULL,
    embedding_text  TEXT NOT NULL,
    embedding_model TEXT NOT NULL
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
