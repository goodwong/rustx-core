use super::token::{NONCE_LENGTH, REFRESH_TOKEN_LIFE_DAYS};
use crate::diesel_schema::{user_tokens, users};
use bcrypt;
use chrono::{DateTime, Duration, Utc};

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
#[derive(Identifiable, Queryable, Associations, Insertable, Clone)]
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
impl UserToken {
    pub fn is_valid(&self, nonce: &[u8; NONCE_LENGTH]) -> bool {
        !self.is_expired() && bcrypt::verify(nonce, &self.hash).unwrap_or(false)
    }
    fn is_expired(&self) -> bool {
        self.deleted_at.is_none()
            && self.issued_at + Duration::hours(REFRESH_TOKEN_LIFE_DAYS) < Utc::now()
    }
}
