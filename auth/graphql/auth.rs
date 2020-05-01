use super::super::repository::find_user;
use super::context::Context;
use crate::auth::models::User as UserModel;
use juniper;
use juniper::FieldResult;

#[derive(juniper::GraphQLObject)]
#[graphql(description = "用户类型")]
pub struct User {
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

pub(crate) async fn query_me(context: &Context) -> FieldResult<User> {
    context
        .identity
        .user()
        .await
        .map(From::from)
        .ok_or_else(|| "未登录".into())
}

pub(crate) async fn login(context: &Context) -> FieldResult<bool> {
    let user = find_user(1, context.pool.get()?).await?;
    context.identity.login(user).await?;
    Ok(true)
}

pub(crate) async fn logout(context: &Context) -> FieldResult<bool> {
    context.identity.logout().await?;
    Ok(true)
}
