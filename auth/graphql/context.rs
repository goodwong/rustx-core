use crate::auth::service::Identity;
use crate::db_connection::PgPool;

pub struct Context {
    pub pool: PgPool,
    pub identity: Identity,
    // todo 增加一个 session类，暂时包装 actix-session即可
}
impl juniper::Context for Context {}
impl Context {
    pub fn new(pool: PgPool, identity: Identity) -> Self {
        Self { pool, identity }
    }
}
