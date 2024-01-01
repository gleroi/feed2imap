-- Add migration script here
CREATE TABLE feeds(
    id TEXT PRIMARY KEY,
    title TEXT,
    last_updated INTEGER,
    last_checked INTEGER
);

CREATE TABLE feed_entries(
    id TEXT PRIMARY KEY,
    feed_id TEXT,
    FOREIGN KEY(feed_id) REFERENCES feeds(id)
);
