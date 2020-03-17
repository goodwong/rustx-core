use super::models::User;
use crate::db_connection::PgPooledConnection;
use diesel::prelude::*;
use tokio::task;

pub async fn all_users(conn: PgPooledConnection) -> QueryResult<Vec<User>> {
    task::spawn_blocking(move || {
        use crate::diesel_schema::users::dsl::*;
        users.limit(20).load::<User>(&conn)
    })
    .await
    .unwrap()
}
