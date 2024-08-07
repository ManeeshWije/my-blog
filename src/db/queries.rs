use crate::models::article::Article;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn get_blog_by_id(pool: &PgPool, id: Uuid) -> Result<Article, sqlx::Error> {
    let _article = sqlx::query_as!(
        Article,
        "
        SELECT * FROM blogs
        WHERE id = $1
        ",
        id
    )
    .fetch_one(pool)
    .await?;

    let updated_views_article = update_views(pool, id).await?;

    Ok(updated_views_article)
}

pub async fn get_blogs(pool: &PgPool) -> Result<Vec<Article>, sqlx::Error> {
    let articles = sqlx::query_as!(
        Article,
        "
        SELECT * FROM blogs
        ",
    )
    .fetch_all(pool)
    .await?;

    Ok(articles)
}

pub async fn create_blog(pool: &PgPool, article: Article) -> Result<Article, sqlx::Error> {
    let article = sqlx::query_as!(
        Article,
        "
        INSERT INTO blogs(id, filename, title, author, content, views, created_at)
        VALUES($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        ",
        article.id,
        article.filename,
        article.title,
        article.author,
        article.content,
        article.views,
        article.created_at
    )
    .fetch_one(pool)
    .await?;

    Ok(article)
}

pub async fn update_views(pool: &PgPool, id: Uuid) -> Result<Article, sqlx::Error> {
    let article = sqlx::query_as!(
        Article,
        "
        UPDATE blogs
        SET views = views + 1
        WHERE id = $1
        RETURNING *
        ",
        id,
    )
    .fetch_one(pool)
    .await?;

    Ok(article)
}

pub async fn update_article_content(
    pool: &PgPool,
    id: Uuid,
    content: String,
) -> Result<Article, sqlx::Error> {
    let article = sqlx::query_as!(
        Article,
        "
        UPDATE blogs
        SET content = $1
        WHERE id = $2
        RETURNING *
        ",
        content,
        id
    )
    .fetch_one(pool)
    .await?;

    Ok(article)
}

pub async fn delete_blog(pool: &PgPool, id: Uuid) -> Result<Article, sqlx::Error> {
    let article = sqlx::query_as!(
        Article,
        "
        DELETE FROM blogs
        WHERE id = $1
        RETURNING *
        ",
        id
    )
    .fetch_one(pool)
    .await?;

    Ok(article)
}

pub async fn search_blogs(pool: &PgPool, search_text: String) -> Result<Vec<Article>, sqlx::Error> {
    let articles = sqlx::query_as!(
        Article,
        "
        SELECT * FROM blogs
        WHERE blogs.title ILIKE $1
        ",
        format!("%{}%", search_text)
    )
    .fetch_all(pool)
    .await?;

    Ok(articles)
}
