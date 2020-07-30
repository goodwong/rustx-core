use crate::diesel_schema::{user_tokens, users};
use chrono::{DateTime, Utc};

// 用户
#[derive(Identifiable, Queryable, Clone, PartialEq, Debug)]
pub struct User {
    pub id: i32,

    pub username: String,
    pub name: String,
    pub avatar: String,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// 用户登录时生成的token
#[derive(Identifiable, Queryable, Associations, Clone)]
#[belongs_to(User)]
#[table_name = "user_tokens"]
pub struct UserToken {
    pub id: i32,
    pub user_id: i32,

    pub device: String,
    pub hash: String,

    pub created_at: DateTime<Utc>,
    pub issued_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
