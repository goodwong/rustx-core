use crate::auth::service::Identity;
use crate::db_connection::PgPool;

pub struct Context {
    pub pool: PgPool,
    pub identity: Identity,
}
impl juniper::Context for Context {}
impl Context {
    pub fn new(pool: PgPool, identity: Identity) -> Self {
        Self { pool, identity }
    }
}
