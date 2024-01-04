-- Add migration script here
CREATE TABLE feeds(
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    url TEXT UNIQUE NOT NULL,
    last_updated DATETIME NOT NULL,
    last_checked DATETIME NOT NULL
);

CREATE TABLE feed_entries(
    id TEXT NOT NULL,
    feed_id TEXT NOT NULL,
    PRIMARY KEY(id, feed_id),
    FOREIGN KEY(feed_id) REFERENCES feeds(id)
);
