use crate::api::wechat_miniprogram::Miniprogram;
use crate::auth::service::Identity;
use crate::db_connection::PgPool;
use std::sync::Arc;

pub struct Context {
    pub pool: PgPool,
    pub identity: Identity,
    pub miniprogram: Arc<Miniprogram>,
    // todo 增加一个 session类，暂时包装 actix-session即可
}
impl juniper::Context for Context {}
impl Context {
    pub fn new(pool: PgPool, identity: Identity, miniprogram: Arc<Miniprogram>) -> Self {
        Self {
            pool,
            identity,
            miniprogram,
        }
    }
}
