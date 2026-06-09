-- URLs we have already seen, including explicitly dismissed candidates.
CREATE TABLE IF NOT EXISTS article_urls (
    url TEXT PRIMARY KEY
);

-- Candidate sources waiting to be promoted into user-facing articles.
-- `payload` stores the full Rust `PendingSource` JSON so schema churn in the
-- app model does not require a column migration for every content field.
CREATE TABLE IF NOT EXISTS pending_sources (
    url TEXT PRIMARY KEY,
    payload TEXT NOT NULL,
    published TEXT NOT NULL,
    embedding TEXT NOT NULL,
    embedding_text TEXT NOT NULL,
    embedding_model TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS pending_sources_published_idx
    ON pending_sources (published);

-- sqlite-vec KNN index for pending source similarity search.
-- Triggers below keep this table in sync with `pending_sources.embedding`.
CREATE VIRTUAL TABLE IF NOT EXISTS pending_source_embeddings USING vec0(
    embedding FLOAT[512] distance_metric=cosine,
    url TEXT PRIMARY KEY
);

CREATE TRIGGER IF NOT EXISTS sync_pending_source_embedding_after_insert
AFTER INSERT ON pending_sources
BEGIN
    INSERT INTO pending_source_embeddings (embedding, url)
    VALUES (new.embedding, new.url);
END;

CREATE TRIGGER IF NOT EXISTS sync_pending_source_embedding_after_embedding_update
AFTER UPDATE OF embedding ON pending_sources
BEGIN
    DELETE FROM pending_source_embeddings WHERE url = old.url;
    INSERT INTO pending_source_embeddings (embedding, url)
    VALUES (new.embedding, new.url);
END;

CREATE TRIGGER IF NOT EXISTS sync_pending_source_embedding_after_delete
AFTER DELETE ON pending_sources
BEGIN
    DELETE FROM pending_source_embeddings WHERE url = old.url;
END;

-- Full articles shown to the user.
CREATE TABLE IF NOT EXISTS articles (
    id TEXT PRIMARY KEY,
    payload TEXT NOT NULL,
    published TEXT NOT NULL,
    category TEXT NOT NULL,
    status TEXT NOT NULL,
    status_changed TEXT NOT NULL,
    rating TEXT,
    embedding TEXT NOT NULL,
    embedding_text TEXT NOT NULL,
    embedding_model TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS articles_published_idx
    ON articles (published);

CREATE INDEX IF NOT EXISTS articles_status_category_idx
    ON articles (status, category);

CREATE VIRTUAL TABLE IF NOT EXISTS user_article_embeddings USING vec0(
    embedding FLOAT[512] distance_metric=cosine,
    id TEXT PRIMARY KEY
);

CREATE TRIGGER IF NOT EXISTS sync_user_article_embedding_after_insert
AFTER INSERT ON articles
BEGIN
    INSERT INTO user_article_embeddings (embedding, id)
    VALUES (new.embedding, new.id);
END;

CREATE TRIGGER IF NOT EXISTS sync_user_article_embedding_after_embedding_update
AFTER UPDATE OF embedding ON articles
BEGIN
    DELETE FROM user_article_embeddings WHERE id = old.id;
    INSERT INTO user_article_embeddings (embedding, id)
    VALUES (new.embedding, new.id);
END;

CREATE TRIGGER IF NOT EXISTS sync_user_article_embedding_after_delete
AFTER DELETE ON articles
BEGIN
    DELETE FROM user_article_embeddings WHERE id = old.id;
END;

-- User preferences for non-article entities, e.g. domains/categories/regions.
CREATE TABLE IF NOT EXISTS item_ratings (
    id TEXT PRIMARY KEY,
    rating TEXT NOT NULL
);
