use anyhow::Error;
use sqlx::sqlite::SqlitePool;

pub use sqlx::types::chrono::{NaiveDateTime as DateTime, Utc};

pub struct Store {
    pool: SqlitePool,
}

#[derive(sqlx::FromRow)]
pub struct Feed {
    pub id: String,
    pub title: String,
    pub url: String,
    pub last_updated: DateTime,
    pub last_checked: DateTime,
}

#[derive(sqlx::FromRow)]
pub struct Entry {
    pub id: String,
    pub feed_id: String,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        return self.id == other.id && self.feed_id == other.feed_id;
    }
}

impl Store {
    pub async fn new(connection_string: &str) -> Result<Store, Error> {
        Ok(Store {
            pool: SqlitePool::connect(connection_string).await?,
        })
    }

    pub async fn list_feeds(&self) -> Result<Vec<Feed>, Error> {
        let feeds = sqlx::query_as!(
            Feed,
            r#"
                select * from feeds
            "#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(feeds)
    }

    pub async fn store_feed(&self, feed: &Feed, entries: &Vec<Entry>) -> Result<(), Error> {
        let id = sqlx::query!(
            r#"
            INSERT INTO feeds (id, title, url, last_updated, last_checked) VALUES (
                ?1, ?2, ?3, ?4, ?5
            ) ON CONFLICT(id) DO UPDATE SET
                title=excluded.title, 
                url=excluded.url,
                last_updated=excluded.last_updated,
                last_checked=excluded.last_checked
        "#,
            feed.id,
            feed.title,
            feed.url,
            feed.last_updated,
            feed.last_checked,
        )
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        log::debug!("feed {} ({}) inserted at {}", feed.title, feed.id, id);

        let existing_entries = sqlx::query_as!(
            Entry,
            "SELECT id, feed_id FROM feed_entries WHERE feed_id = ?1",
            feed.id,
        )
        .fetch_all(&self.pool)
        .await?;
        for entry in entries {
            if existing_entries.iter().any(|e| e == entry) {
                continue;
            }
            let _id = sqlx::query!(
                r#"
                INSERT INTO feed_entries (id, feed_id) VALUES (
                    ?1, ?2
                )
                "#,
                entry.id,
                feed.id,
            )
            .execute(&self.pool)
            .await?
            .last_insert_rowid();
            log::debug!("added entry {} for feed {}", entry.id, feed.id);
        }

        Ok(())
    }
}
