use crate::api::wechat_miniprogram::Code2SessionResponse;
use crate::auth::graphql::context::Context;
use crate::auth::models::User as UserModel;
use crate::auth::repository as user_repository;
use crate::wechat::miniprogram::repository as miniprogram_repository;
use diesel::result::Error as DieselError;
use juniper;
use juniper::FieldResult;

const SESSION_KEY_OPENID: &str = "mp_openid";
const SESSION_KEY_UNIONID: &str = "mp_unionid";
const SESSION_KEY_SESSIONKEY: &str = "mp_session_key";

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
pub(crate) async fn login(js_code: String, context: &Context) -> FieldResult<LoginResult> {
    let miniprogram_session = context.miniprogram.code_to_session(&js_code).await?;
    login_by_wechat_miniprogram_openid(miniprogram_session, context).await
}

/// register
/// by miniprogram phoneNumber
pub(crate) async fn register(_context: &Context) -> FieldResult<bool> {
    todo!()
}

pub(crate) async fn logout(context: &Context) -> FieldResult<bool> {
    context.identity.logout().await?;
    Ok(true)
}

#[derive(juniper::GraphQLObject)]
pub(crate) struct LoginResult {
    success: bool,
    user: Option<User>,
}
impl LoginResult {
    fn success(u: User) -> Self {
        Self {
            success: true,
            user: Some(u),
        }
    }
    fn failure() -> Self {
        Self {
            success: false,
            user: None,
        }
    }
}

async fn login_by_wechat_miniprogram_openid(
    mp_session: Code2SessionResponse,
    context: &Context,
) -> FieldResult<LoginResult> {
    // 设置session_key，后续登陆、解码用到
    context
        .session
        .set(SESSION_KEY_SESSIONKEY, mp_session.session_key)
        .await?;

    // 数据库查询是否有记录
    match miniprogram_repository::find(mp_session.openid.clone(), context.pool.get()?).await {
        // 顺利登陆
        Ok(mp_user) => {
            let user = user_repository::find_user(mp_user.user_id, context.pool.get()?).await?;
            context.identity.login(user.clone()).await?;
            Ok(LoginResult::success(user.into()))
        }

        // 此情况表示小程序首次登陆
        // 记住openid/unionid，需前端补充提供手机号
        // 下一步：如果手机号登陆成功，则绑定该openid至手机号，并从session清除该openid
        Err(DieselError::NotFound) => {
            context
                .session
                .set(SESSION_KEY_OPENID, mp_session.openid)
                .await?;
            if let Some(unionid) = mp_session.unionid {
                context.session.set(SESSION_KEY_UNIONID, unionid).await?;
            }
            Ok(LoginResult::failure())
        }
        Err(e) => Err(e.into()),
    }
}

async fn register_by_wechat_miniprogram_phonenumber() {
    todo!()
}

#[cfg(test)]
mod tests {
    use crate::api::wechat_miniprogram::Code2SessionResponse;
    use crate::auth::service::TokenResponse;
    use crate::auth::tests;
    use crate::auth::tests::TestResult;

    #[tokio::test]
    async fn login_by_wechat_miniprogram_openid() -> TestResult<()> {
        let pool = tests::db_pool();
        // mock user
        let user = tests::mock_user(pool.clone()).await?;
        // mock miniprogram_user
        let mp_user = tests::mock_miniprogram_user(pool.clone()).await?;
        // mock context
        let ctx = tests::mock_context(pool.clone()).await?;
        let id = &ctx.identity;

        // success
        let mp_session = Code2SessionResponse {
            openid: mp_user.open_id,
            unionid: mp_user.union_id,
            session_key: Default::default(),
        };
        let login_success = super::login_by_wechat_miniprogram_openid(mp_session, &ctx)
            .await
            .map_err(|e| format!("{:?}", e))?;
        assert!(login_success.success);
        assert_eq!(id.is_login().await, true);
        assert_eq!(user.id, id.user_id().await.unwrap());
        assert_eq!(user, id.user().await.unwrap());
        assert!(matches!(
            id.get_response().await,
            Some(TokenResponse::Set(_, _))
        ));

        // failure
        // update: 觉得此处login failure不应该自动logout，这个应该是Gateway的事
        // 所以删掉了这部分测试代码

        Ok(())
    }
}
