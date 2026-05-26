DEFINE TABLE pending_sources SCHEMALESS;
DEFINE INDEX source_url ON TABLE pending_sources FIELDS url UNIQUE;
DEFINE INDEX pending_embedding ON TABLE pending_sources FIELDS embedding HNSW DIMENSION 768 DIST COSINE;

DEFINE TABLE article_urls SCHEMALESS;

DEFINE TABLE user_articles SCHEMALESS;
DEFINE INDEX article_published ON TABLE user_articles FIELDS published;
DEFINE INDEX article_embedding ON TABLE user_articles FIELDS embedding HNSW DIMENSION 768 DIST COSINE;

DEFINE TABLE item_ratings SCHEMALESS;
DEFINE TABLE label_embeddings SCHEMALESS;

DEFINE INDEX label_embedding_group ON TABLE label_embeddings FIELDS label_group;
