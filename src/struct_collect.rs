use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct Record {
    pub id: String,
    pub content: String,
    pub tag: String,
    pub priority: String,
}
