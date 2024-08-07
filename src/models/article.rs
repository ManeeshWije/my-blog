use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Article {
    pub id: Uuid,
    pub filename: String,
    pub title: String,
    pub author: String,
    pub content: String,
    pub created_at: String,
    pub views: BigDecimal,
}
