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
    source_rowid INTEGER PRIMARY KEY,
    embedding FLOAT[512] distance_metric=cosine
);

CREATE TRIGGER IF NOT EXISTS sync_pending_source_embedding_after_insert
AFTER INSERT ON pending_sources
BEGIN
    INSERT INTO pending_source_embeddings (source_rowid, embedding)
    VALUES (new.rowid, new.embedding);
END;

CREATE TRIGGER IF NOT EXISTS sync_pending_source_embedding_after_embedding_update
AFTER UPDATE OF embedding ON pending_sources
BEGIN
    DELETE FROM pending_source_embeddings WHERE source_rowid = old.rowid;
    INSERT INTO pending_source_embeddings (source_rowid, embedding)
    VALUES (new.rowid, new.embedding);
END;

CREATE TRIGGER IF NOT EXISTS sync_pending_source_embedding_after_delete
AFTER DELETE ON pending_sources
BEGIN
    DELETE FROM pending_source_embeddings WHERE source_rowid = old.rowid;
END;

-- Promoted articles shown to the user.
-- `payload` stores the full Rust `Article` JSON; typed columns are duplicated
-- only where SQLite needs to filter, sort, or vector-search efficiently.
CREATE TABLE IF NOT EXISTS user_articles (
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

CREATE INDEX IF NOT EXISTS user_articles_published_idx
    ON user_articles (published);

CREATE INDEX IF NOT EXISTS user_articles_status_category_idx
    ON user_articles (status, category);

-- sqlite-vec KNN index for article preference scoring.
-- Triggers below keep this table in sync with `user_articles.embedding`.
CREATE VIRTUAL TABLE IF NOT EXISTS user_article_embeddings USING vec0(
    article_rowid INTEGER PRIMARY KEY,
    embedding FLOAT[512] distance_metric=cosine
);

CREATE TRIGGER IF NOT EXISTS sync_user_article_embedding_after_insert
AFTER INSERT ON user_articles
BEGIN
    INSERT INTO user_article_embeddings (article_rowid, embedding)
    VALUES (new.rowid, new.embedding);
END;

CREATE TRIGGER IF NOT EXISTS sync_user_article_embedding_after_embedding_update
AFTER UPDATE OF embedding ON user_articles
BEGIN
    DELETE FROM user_article_embeddings WHERE article_rowid = old.rowid;
    INSERT INTO user_article_embeddings (article_rowid, embedding)
    VALUES (new.rowid, new.embedding);
END;

CREATE TRIGGER IF NOT EXISTS sync_user_article_embedding_after_delete
AFTER DELETE ON user_articles
BEGIN
    DELETE FROM user_article_embeddings WHERE article_rowid = old.rowid;
END;

-- User preferences for non-article entities, e.g. domains/categories/regions.
CREATE TABLE IF NOT EXISTS item_ratings (
    id TEXT PRIMARY KEY,
    rating TEXT NOT NULL
);

-- Static label embeddings used to classify incoming content.
CREATE TABLE IF NOT EXISTS label_embeddings (
    id TEXT PRIMARY KEY,
    label_group TEXT NOT NULL,
    label_value TEXT NOT NULL,
    hash TEXT NOT NULL,
    text TEXT NOT NULL,
    embedding TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS label_embeddings_group_idx
    ON label_embeddings (label_group);

-- sqlite-vec KNN index for category/region/sentiment/importance labels.
-- Triggers below keep this table in sync with `label_embeddings.embedding`.
CREATE VIRTUAL TABLE IF NOT EXISTS label_embedding_vectors USING vec0(
    label_rowid INTEGER PRIMARY KEY,
    embedding FLOAT[512] distance_metric=cosine
);

CREATE TRIGGER IF NOT EXISTS sync_label_embedding_vector_after_insert
AFTER INSERT ON label_embeddings
BEGIN
    INSERT INTO label_embedding_vectors (label_rowid, embedding)
    VALUES (new.rowid, new.embedding);
END;

CREATE TRIGGER IF NOT EXISTS sync_label_embedding_vector_after_embedding_update
AFTER UPDATE OF embedding ON label_embeddings
BEGIN
    DELETE FROM label_embedding_vectors WHERE label_rowid = old.rowid;
    INSERT INTO label_embedding_vectors (label_rowid, embedding)
    VALUES (new.rowid, new.embedding);
END;

CREATE TRIGGER IF NOT EXISTS sync_label_embedding_vector_after_delete
AFTER DELETE ON label_embeddings
BEGIN
    DELETE FROM label_embedding_vectors WHERE label_rowid = old.rowid;
END;
