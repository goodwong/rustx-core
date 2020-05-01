use super::auth as auth_resolver;
use super::context::Context;

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
    async fn login(context: &Context) -> FieldResult<bool> {
        auth_resolver::login(context).await
    }

    #[doc = "注销登陆态"]
    async fn logout(context: &Context) -> FieldResult<bool> {
        auth_resolver::logout(context).await
    }
}

pub type Schema = juniper::RootNode<'static, QueryRoot, MutationRoot>;
pub fn create_schema() -> Schema {
    Schema::new(QueryRoot {}, MutationRoot {})
}
