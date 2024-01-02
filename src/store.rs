use anyhow::Error;
use sqlx::sqlite::SqlitePool;

pub use sqlx::types::chrono::{DateTime, Utc};

pub struct Store {
    pool: SqlitePool,
}

#[derive(sqlx::FromRow)]
pub struct Feed {
    pub id: String,
    pub title: String,
    pub last_updated: DateTime<Utc>,
    pub last_checked: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
pub struct Entry {
    id: String,
    feed_id: String,
}

impl Store {
    pub async fn new(connection_string: &str) -> Result<Store, Error> {
        Ok(Store {
            pool: SqlitePool::connect(connection_string).await?,
        })
    }

    pub async fn add_feed(&self, feed: Feed) -> Result<(), Error> {
        let id = sqlx::query!(
            r#"
            INSERT INTO feeds (id, title, last_updated) VALUES (
                ?1, ?2, ?3
            ) ON CONFLICT(id) DO UPDATE SET
                title=excluded.title, 
                last_updated=excluded.last_updated
        "#,
            feed.id,
            feed.title,
            feed.last_updated
        )
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        log::debug!("feed {} ({}) inserted at {}", feed.title, feed.id, id);
        Ok(())
    }
}
