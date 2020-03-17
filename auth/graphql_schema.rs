use super::models::User as UserModel;
use super::repository::all_users;
use crate::db_connection::{PgPool, PgPooledConnection};
use juniper;
use juniper::FieldResult;

pub struct Context {
    pub pool: PgPool,
}
impl juniper::Context for Context {}
impl Context {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(juniper::GraphQLObject)]
#[graphql(description = "用户类型")]
struct User {
    #[graphql(description = "用户ID")]
    id: i32,

    #[graphql(description = "用户名")]
    username: String,

    #[graphql(description = "用户昵称")]
    name: String,

    #[graphql(description = "用户头像")]
    avatar: String,

    #[graphql(description = "用户创建时间")]
    created_at: String,

    #[graphql(description = "用户更新时间")]
    updated_at: String,
}
impl From<UserModel> for User {
    fn from(m: UserModel) -> Self {
        Self {
            id: m.id,
            username: m.username,
            name: m.name,
            avatar: m.avatar,
            created_at: m.created_at.to_rfc3339(),
            updated_at: m.updated_at.to_rfc3339(),
        }
    }
}

pub struct QueryRoot;
#[juniper::graphql_object(Context = Context)]
impl QueryRoot {
    #[doc = "获取所有的用户名单"]
    async fn all_users(context: &Context) -> FieldResult<Vec<User>> {
        let conn: PgPooledConnection = context.pool.get().expect("获取数据库连接失败");
        Ok(all_users(conn)
            .await?
            .into_iter()
            .map(|u| User::from(u))
            .collect())
    }
}

pub struct MutationRoot;
#[juniper::graphql_object(Context = Context)]
impl MutationRoot {
    #[graphql(description = "注销登陆态")]
    async fn logout() -> bool {
        // 占位
        false
    }
}

pub type Schema = juniper::RootNode<'static, QueryRoot, MutationRoot>;
pub fn create_schema() -> Schema {
    Schema::new(QueryRoot {}, MutationRoot {})
}
