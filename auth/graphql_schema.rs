pub use super::context::Context;
use super::models::User as UserModel;
use super::repository::find_user;

use juniper;
use juniper::FieldResult;

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
    #[doc = "根据session获取当前用户信息"]
    async fn me(context: &Context) -> FieldResult<User> {
        context
            .identity
            .user()
            .await
            .map(From::from)
            .ok_or_else(|| "未登录".into())
    }
}

pub struct MutationRoot;
#[juniper::graphql_object(Context = Context)]
impl MutationRoot {
    #[doc = "登陆"]
    async fn login(context: &Context) -> FieldResult<bool> {
        let user = find_user(1, context.pool.get()?).await?;
        context.identity.login(user).await?;
        Ok(true)
    }

    #[doc = "注销登陆态"]
    async fn logout(context: &Context) -> FieldResult<bool> {
        context.identity.logout().await?;
        Ok(true)
    }
}

pub type Schema = juniper::RootNode<'static, QueryRoot, MutationRoot>;
pub fn create_schema() -> Schema {
    Schema::new(QueryRoot {}, MutationRoot {})
}
