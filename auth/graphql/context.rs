use crate::api::wechat_miniprogram::Miniprogram;
use crate::auth::graphql::Session;
use crate::auth::service::Identity;
use crate::db_connection::PgPool;

pub struct Context {
    pub pool: PgPool,
    pub identity: Identity,
    pub miniprogram: Miniprogram,
    pub session: Session,
}
impl juniper::Context for Context {}
impl Context {
    pub fn new(
        pool: PgPool,
        identity: Identity,
        miniprogram: Miniprogram,
        session: Session,
    ) -> Self {
        Self {
            pool,
            identity,
            miniprogram,
            session,
        }
    }
}
