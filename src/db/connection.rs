use sqlx::postgres::PgPoolOptions;

pub async fn connect(db_url: &str) -> Result<sqlx::PgPool, sqlx::Error> {
    return PgPoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await;
}
