use chrono::prelude::*;

// 用户
#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub name: String,
    pub avatar: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// 用户认证方式，如密码、短信验证码、第三方验证等
#[derive(Queryable)]
pub struct UserIdentity {
    pub user_id: i32,
    pub provider: String,
    pub open_id: String,
    pub data: Option<String>,
}

// 用户登录时生成的token
#[derive(Queryable)]
pub struct UserToken {
    pub id: i32,
    pub user_id: i32,
    pub device: String,
    pub hash: String,
    pub issued_at: DateTime<Utc>,
    pub expired_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
