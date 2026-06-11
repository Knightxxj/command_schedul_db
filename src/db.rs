use sqlx::postgres::PgPool;
use crate::struct_collect::Record;

pub struct Db {
    pool: PgPool,
}

impl Db {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(database_url).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Db { pool })
    }

    pub async fn get_task_id(&self) -> Result<String, sqlx::Error> {
        let row: (String,) = sqlx::query_as(
            "SELECT last_id FROM id_counter WHERE id = 1"
        )
        .fetch_one(&self.pool).await?;
        Ok(row.0)
    }

    pub async fn list_tasks(
        &self,
        tag: Option<&str>,
        priority: Option<&str>,
    ) -> Result<Vec<Record>, sqlx::Error> {
        match (tag, priority) {
            (Some(t), Some(p)) => {
                sqlx::query_as::<_, Record>(
                    "SELECT id, content, tag, priority FROM tasks
                     WHERE tag = $1 AND priority = $2
                     ORDER BY id"
                )
                .bind(t).bind(p)
                .fetch_all(&self.pool).await
            }
            (Some(t), None) => {
                sqlx::query_as::<_, Record>(
                    "SELECT id, content, tag, priority FROM tasks
                     WHERE tag = $1 ORDER BY id"
                )
                .bind(t)
                .fetch_all(&self.pool).await
            }
            (None, Some(p)) => {
                sqlx::query_as::<_, Record>(
                    "SELECT id, content, tag, priority FROM tasks
                     WHERE priority = $1 ORDER BY id"
                )
                .bind(p)
                .fetch_all(&self.pool).await
            }
            (None, None) => {
                sqlx::query_as::<_, Record>(
                    "SELECT id, content, tag, priority FROM tasks ORDER BY id"
                )
                .fetch_all(&self.pool).await
            }
        }
    }

    pub async fn insert_task(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        record: &Record,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO tasks (id, content, tag, priority)
             VALUES ($1, $2, $3, $4)"
        )
        .bind(&record.id)
        .bind(&record.content)
        .bind(&record.tag)
        .bind(&record.priority)
        .execute(&mut **tx).await?;
        Ok(())
    }

    pub async fn mark_done(&self, id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE tasks SET tag = 'done', updated_at = NOW() WHERE id = $1"
        )
        .bind(id)
        .execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_task(&self, id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM tasks WHERE id = $1"
        )
        .bind(id)
        .execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    pub async fn add_task_with_counter(
        &self,
        record: &Record,
        new_id: &str,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("UPDATE id_counter SET last_id = $1 WHERE id = 1")
            .bind(new_id)
            .execute(&mut *tx).await?;

        Self::insert_task(&mut tx, record).await?;

        tx.commit().await?;
        Ok(())
    }
}
