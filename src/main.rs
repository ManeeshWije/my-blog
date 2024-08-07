mod db;
mod models;
use bigdecimal::BigDecimal;
use chrono::prelude::*;
use db::queries::{create_blog, delete_blog, get_blogs, update_article_content};
use dotenv::dotenv;
use handlebars::Handlebars;
use models::article::Article;
use serde::Deserialize;
use serde_json::json;
use serde_urlencoded::from_str;
use sqlx::PgPool;
use std::{env, ffi::OsString, fs, io, str::FromStr, sync::Arc};
use uuid::Uuid;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};

use tower_http::cors::{Any, CorsLayer};
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Clone)]
struct AppState {
    handlebars: Arc<Handlebars<'static>>,
    pool: PgPool,
}

#[derive(Debug, Deserialize)]
struct SearchParams {
    search: String,
}

async fn insert_files(app_state: AppState) -> Result<(), anyhow::Error> {
    let entries = fs::read_dir("src/markdown")?
        .map(|res| res.map(|e| e.file_name()))
        .collect::<Result<Vec<_>, io::Error>>()?;

    let articles = get_blogs(&app_state.pool).await?;

    let files_to_delete: Vec<_> = articles
        .iter()
        .filter(|article| !entries.contains(&OsString::from(&article.filename)))
        .collect();

    for file in files_to_delete {
        delete_blog(&app_state.pool, file.id).await?;
    }

    for entry in entries.iter() {
        let mut current_title = "Untitled".to_string();
        let entry_str = entry.to_string_lossy();
        let file_content = fs::read_to_string(format!("src/markdown/{}", entry_str))?;
        for line in file_content.lines() {
            if line.starts_with('#') {
                current_title = line
                    .to_string()
                    .strip_prefix("#")
                    .expect("ERROR: Could not strip # to get blog title")
                    .trim()
                    .to_string();
                break;
            }
        }

        let post_content = markdown::to_html_with_options(
            &file_content,
            &markdown::Options {
                compile: markdown::CompileOptions {
                    allow_dangerous_html: true,
                    allow_dangerous_protocol: true,
                    ..markdown::CompileOptions::default()
                },
                ..markdown::Options::gfm()
            },
        )
        .unwrap();
        let existing_article = articles.iter().find(|doc| doc.title == current_title);

        match existing_article {
            Some(article) => {
                println!("INFO: updating contents for {}", article.title);
                update_article_content(&app_state.pool, article.id, post_content).await?;
            }
            None => {
                let insert_article = Article {
                    id: Uuid::new_v4(),
                    filename: entry.to_string_lossy().into_owned(),
                    title: current_title,
                    author: "Maneesh Wijewardhana".to_string(),
                    content: post_content,
                    created_at: Utc::now().to_rfc3339(),
                    views: BigDecimal::from_str("0")
                        .expect("ERROR: Could not assign views property to 0"),
                };
                println!("INFO: inserting new post {}", insert_article.title);
                create_blog(&app_state.pool, insert_article).await?;
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let pool = db::connection::connect(env::var("DATABASE_URL").unwrap().as_str())
        .await
        .unwrap_or_else(|err| {
            eprintln!("ERROR: Could not connect to postgres database: {}", err);
            std::process::exit(1);
        });

    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_file("articles", "./src/views/partials/articles.hbs")
        .expect("ERROR: could not register template file - articles");
    handlebars
        .register_template_file("content", "./src/views/partials/content.hbs")
        .expect("ERROR: could not register template file - content");
    handlebars
        .register_template_file("footer", "./src/views/partials/footer.hbs")
        .expect("ERROR: could not register template file - footer");
    handlebars
        .register_template_file("head", "./src/views/partials/head.hbs")
        .expect("ERROR: could not register template file - head");
    handlebars
        .register_template_file("nav", "./src/views/partials/nav.hbs")
        .expect("ERROR: could not register template file - nav");
    handlebars
        .register_template_file("index", "./src/views/index.hbs")
        .expect("ERROR: could not register template file - index");

    let handlebars = Arc::new(handlebars);

    let app_state = AppState { handlebars, pool };

    let cors = CorsLayer::new().allow_methods(Any).allow_origin(Any);

    let insert_files_state = app_state.clone();
    tokio::spawn(async move {
        if let Err(e) = insert_files(insert_files_state).await {
            eprintln!("ERROR: insert_files failed: {}", e);
        }
    });

    let app = Router::new()
        .nest_service("/client", ServeDir::new("client"))
        .nest_service("/public", ServeDir::new("public"))
        .route("/", get(all_articles))
        .route("/articles/:title", get(article))
        .route("/search", post(search))
        .with_state(app_state)
        .layer(cors)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn all_articles(State(app_state): State<AppState>) -> impl IntoResponse {
    let handlebars = &app_state.handlebars;
    let pool = &app_state.pool;
    let blogs = db::queries::get_blogs(pool)
        .await
        .expect("ERROR: Could not fetch all blogs");

    let data = json!({ "articles": blogs });

    let index = handlebars
        .render("index", &data)
        .expect("ERROR: Could not render index template during fetch all articles");
    (StatusCode::OK, Html(index))
}

async fn article(State(app_state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let handlebars = &app_state.handlebars;
    let pool = &app_state.pool;
    let blog = db::queries::get_blog_by_id(pool, id)
        .await
        .expect("ERROR: Could not get blog by id");

    let human_readable_date = match DateTime::parse_from_rfc3339(&blog.created_at) {
        Ok(parsed_date) => {
            let utc_date: DateTime<Utc> = parsed_date.with_timezone(&Utc);
            utc_date.format("%Y-%m-%d %H:%M:%S").to_string()
        }
        Err(e) => {
            eprintln!("Failed to parse date: {}", e);
            String::from("Invalid date")
        }
    };

    let data = json!({ "title": blog.title, "views": blog.views, "author": blog.author, "date": human_readable_date, "content": blog.content });

    let index = handlebars
        .render("index", &data)
        .expect("ERROR: Could not render index template during fetch article");
    (StatusCode::OK, Html(index))
}

async fn search(State(app_state): State<AppState>, body: String) -> impl IntoResponse {
    let handlebars = &app_state.handlebars;
    let pool = &app_state.pool;
    let body = from_str::<SearchParams>(&body).expect("ERROR: Could not parse search parameters");
    let found_blog = db::queries::search_blogs(pool, body.search)
        .await
        .expect("ERROR: Could not search blogs");

    let data = json!({ "articles": found_blog });

    let index = handlebars
        .render("articles", &data)
        .expect("ERROR: Could not render article template during search");
    (StatusCode::OK, Html(index))
}
