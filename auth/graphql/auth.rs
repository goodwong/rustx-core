use crate::api::wechat_miniprogram::{Code2SessionResponse, Miniprogram};
use crate::auth::graphql::context::Context;
use crate::auth::models::User as UserModel;
use crate::auth::repository as user_repository;
use crate::wechat::miniprogram::models::MiniprogramUser;
use crate::wechat::miniprogram::repository as mp_repository;
use diesel::result::Error as DieselError;
use juniper;
use juniper::FieldResult;
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Debug, juniper::GraphQLInputObject)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegisterInput {
    iv: String,
    encrypted_data: String,
}

/// register
/// by miniprogram phoneNumber
pub(crate) async fn register(args: RegisterInput, context: &Context) -> FieldResult<LoginResult> {
    let phone_number = {
        let session_key = context
            .session
            .get::<String>(SESSION_KEY_SESSIONKEY)
            .await?
            .ok_or("session_key不存在session里")?;
        let iv = args.iv;
        let data = args.encrypted_data;
        Miniprogram::get_phone_number(&session_key, &iv, &data)?.phone_number
    };
    let open_id = context
        .session
        .get::<String>(SESSION_KEY_OPENID)
        .await?
        .ok_or("open_id不存在session里")?;
    let union_id = context.session.get::<String>(SESSION_KEY_UNIONID).await?;

    register_by_wechat_miniprogram_phonenumber(phone_number, open_id, union_id, context).await
}

pub(crate) async fn logout(context: &Context) -> FieldResult<bool> {
    if context.identity.is_login().await {
        context.identity.logout().await?;
        context.session.purge().await;
        Ok(true)
    } else {
        Ok(false)
    }
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

// 分离代码，方便测试
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
    match mp_repository::find(mp_session.openid.clone(), context.pool.get()?).await {
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

async fn register_by_wechat_miniprogram_phonenumber(
    phone_number: String,
    open_id: String,
    union_id: Option<String>,
    context: &Context,
) -> FieldResult<LoginResult> {
    // 根据电话查找exist_user
    // todo: fallback - 根据union_id查找exist_user，暂时不做
    match user_repository::find_user_by_username(phone_number, context.pool.get()?).await {
        Ok(exist_user) => {
            // 关联exist_user与miniprogram_user
            let mp_user =
                mp_repository::create(open_id, exist_user.id, context.pool.get()?).await?;
            if let Some(_) = union_id {
                let update = MiniprogramUser {
                    union_id,
                    ..mp_user
                };
                mp_repository::update(update, context.pool.get()?).await?;
            }

            // 清理session的openid/unionid
            context.session.remove(SESSION_KEY_OPENID).await;
            context.session.remove(SESSION_KEY_UNIONID).await;

            // 设置identity为登陆态
            context.identity.login(exist_user.clone()).await?;

            Ok(LoginResult::success(exist_user.into()))
        }
        Err(DieselError::NotFound) => {
            // update: 管理员登记号码后，仍然可以再次注册，所以session不要清空
            // context.session.purge().await;
            Ok(LoginResult::failure())
        }
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::{SESSION_KEY_OPENID, SESSION_KEY_SESSIONKEY};
    use crate::api::wechat_miniprogram::Code2SessionResponse;
    use crate::auth::tests;
    use crate::auth::tests::TestResult;

    const MOCK_USERNAME: &str = "auth_mock_user_username";
    const MOCK_PHONE_NUMBER: &str = "18899990000";
    const MOCK_MP_OPENID: &str = "auth_mock_miniprogram_user_openid";
    const MOCK_MP_OPENID_2: &str = "auth_mock_miniprogram_user_openid_2"; // 多线程测试中不可共用，所以需要区分不同的名字

    fn setup() {
        // 为了在testing下看到logging
        env_logger::try_init().ok();
    }

    #[async_std::test]
    async fn login_by_wechat_miniprogram_openid() -> TestResult<()> {
        setup();

        let pool = tests::db_pool();

        // clear up for testing
        tests::clear_mock_miniprogram_user(MOCK_MP_OPENID, pool.clone()).await?;
        tests::clear_mock_user(MOCK_USERNAME, pool.clone()).await?;

        // mock user
        let user = tests::mock_user(MOCK_USERNAME, pool.clone()).await?;
        // mock miniprogram_user
        let mp_user = tests::mock_miniprogram_user(MOCK_MP_OPENID, user.id, pool.clone()).await?;
        // mock context
        let ctx = tests::mock_context(pool.clone()).await?;

        // success
        let mock_session_key = "mock session_key";
        let mp_session = Code2SessionResponse {
            openid: mp_user.open_id,
            unionid: mp_user.union_id,
            session_key: mock_session_key.to_owned(),
        };
        let result = super::login_by_wechat_miniprogram_openid(mp_session, &ctx)
            .await
            .map_err(|e| format!("{:?}", e))?;
        assert_eq!(result.success, true);
        assert_eq!(ctx.identity.is_login().await, true);
        // update: 这里不应该再测试identity的内部状态，因为这个是auth.service做的事情，有service::tests负责测试，
        // 这个函数有设置session，应该测试是否正确的set session
        assert_eq!(
            ctx.session.get::<String>(SESSION_KEY_SESSIONKEY).await?,
            Some(mock_session_key.to_owned())
        );

        // failure
        let ctx = tests::mock_context(pool.clone()).await?;
        let mock_openid = "mock openid not exist in database";
        let mp_session = Code2SessionResponse {
            openid: mock_openid.to_owned(),
            unionid: None,
            session_key: mock_session_key.to_owned(),
        };
        let result = super::login_by_wechat_miniprogram_openid(mp_session, &ctx)
            .await
            .map_err(|e| format!("{:?}", e))?;
        assert_eq!(result.success, false);
        assert_eq!(ctx.identity.is_login().await, false);
        assert_eq!(
            ctx.session.get::<String>(SESSION_KEY_SESSIONKEY).await?,
            Some(mock_session_key.to_owned())
        );
        assert_eq!(
            ctx.session.get::<String>(SESSION_KEY_OPENID).await?,
            Some(mock_openid.to_owned()),
        );

        // clear up
        tests::clear_mock_miniprogram_user(MOCK_MP_OPENID, pool.clone()).await?;
        tests::clear_mock_user(MOCK_USERNAME, pool.clone()).await?;

        Ok(())
    }

    #[async_std::test]
    async fn register_by_wechat_miniprogram_phonenumber() -> TestResult<()> {
        setup();

        let pool = tests::db_pool();

        // clear up for testing
        tests::clear_mock_miniprogram_user(MOCK_MP_OPENID_2, pool.clone()).await?;
        tests::clear_mock_user(MOCK_PHONE_NUMBER, pool.clone()).await?;

        // mock user by phone number
        tests::mock_user(MOCK_PHONE_NUMBER, pool.clone()).await?;
        // mock context
        let ctx = tests::mock_context(pool.clone()).await?;

        let phone_number = MOCK_PHONE_NUMBER.to_owned();
        let open_id = MOCK_MP_OPENID_2.to_owned();
        let result =
            super::register_by_wechat_miniprogram_phonenumber(phone_number, open_id, None, &ctx)
                .await
                .map_err(|e| e.message().to_owned())?;
        assert_eq!(result.success, true);
        assert!(result.user.is_some());
        assert_eq!(ctx.identity.is_login().await, true);

        // clear up
        tests::clear_mock_miniprogram_user(MOCK_MP_OPENID_2, pool.clone()).await?;
        tests::clear_mock_user(MOCK_PHONE_NUMBER, pool.clone()).await?;

        Ok(())
    }
}
