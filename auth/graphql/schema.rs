use super::auth as auth_resolver;
use super::auth::LoginResult;
use super::context::Context;
use juniper::EmptySubscription;

use juniper;
use juniper::FieldResult;

pub struct QueryRoot;
#[juniper::graphql_object(Context = Context)]
impl QueryRoot {
    #[doc = "根据session获取当前用户信息"]
    async fn me(context: &Context) -> FieldResult<auth_resolver::User> {
        auth_resolver::query_me(context).await
    }
}

pub struct MutationRoot;
#[juniper::graphql_object(Context = Context)]
impl MutationRoot {
    #[doc = "登陆"]
    #[graphql(
        description = "结果：true - 登录成功; false - 登录失败，需要提供手机号码注册登录；如果发生error，例如数据库连接错误，请重试"
    )]
    async fn login(js_code: String, context: &Context) -> FieldResult<LoginResult> {
        auth_resolver::login(js_code, context).await
    }

    #[doc = "注销登陆态"]
    async fn logout(context: &Context) -> FieldResult<bool> {
        auth_resolver::logout(context).await
    }
}

pub type Schema = juniper::RootNode<'static, QueryRoot, MutationRoot, EmptySubscription<Context>>;
pub fn create_schema() -> Schema {
    Schema::new(QueryRoot {}, MutationRoot {}, EmptySubscription::new())
}
