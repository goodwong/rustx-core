use crate::auth::models::User;
use crate::diesel_schema::wechat_miniprogram_users;

pub(super) type AnyResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Identifiable, Queryable, Associations, Insertable, AsChangeset, Clone, Debug, Default)]
#[belongs_to(User)]
#[primary_key(open_id)]
#[table_name = "wechat_miniprogram_users"]
pub struct MiniprogramUser {
    pub open_id: String,
    pub union_id: Option<String>,
    pub nick_name: Option<String>,
    pub gender: Option<i16>, // 0未知 1男性 2女性
    pub language: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub country: Option<String>,
    pub avatar_url: Option<String>,
    pub user_id: i32, // 关联users表
}
