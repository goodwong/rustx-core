use crate::api::wechat_miniprogram::Code2SessionResponse;
use crate::auth::graphql::context::Context;
use crate::auth::models::User as UserModel;
use crate::auth::repository as user_repository;
use crate::wechat::miniprogram::repository as miniprogram_repository;
use diesel::result::Error as DieselError;
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

/// login 登录
/// 结果：true - 登录成功; false - 登录失败，需要提供手机号码注册登录；如果发生error，例如数据库连接错误，请重试
pub(crate) async fn login(js_code: String, context: &Context) -> FieldResult<bool> {
    let Code2SessionResponse { openid, .. } = context.miniprogram.code_to_session(&js_code).await?;
    login_by_wechat_miniprogram_openid(&openid, context).await
}

pub(crate) async fn logout(context: &Context) -> FieldResult<bool> {
    context.identity.logout().await?;
    Ok(true)
}

pub enum LoginResponse {
    Success,
    Need,
}

async fn login_by_wechat_miniprogram_openid(openid: &str, context: &Context) -> FieldResult<bool> {
    match miniprogram_repository::find(openid, context.pool.get()?).await {
        Ok(mp_user) => {
            let user = user_repository::find_user(mp_user.user_id, context.pool.get()?).await?;
            context.identity.login(user).await?;
            Ok(true)
        }
        Err(DieselError::NotFound) => Ok(false),
        Err(e) => Err(e.into()),
    }
}
