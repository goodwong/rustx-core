use super::models::{AnyResult, MiniprogramUser};
use crate::db_connection::PgPooledConnection;
use diesel::prelude::*;
use tokio::task;

pub async fn find(openid: &str, conn: PgPooledConnection) -> QueryResult<MiniprogramUser> {
    let openid = openid.to_owned();
    task::spawn_blocking(move || {
        use crate::diesel_schema::wechat_miniprogram_users::dsl::*;
        wechat_miniprogram_users
            .filter(open_id.eq(openid))
            .first::<MiniprogramUser>(&conn)
    })
    .await
    .unwrap()
}

pub async fn create(
    openid: &str,
    userid: i32,
    conn: PgPooledConnection,
) -> AnyResult<MiniprogramUser> {
    let openid = openid.to_owned();

    task::spawn_blocking(move || {
        use crate::diesel_schema::wechat_miniprogram_users::dsl::*;
        let insert = MiniprogramUser {
            open_id: openid,
            user_id: userid,
            ..Default::default()
        };
        diesel::insert_into(wechat_miniprogram_users)
            .values(&insert)
            .get_result::<MiniprogramUser>(&conn)
            .map_err(|e| e.into())
    })
    .await
    .unwrap()
}
pub async fn update(u: MiniprogramUser, conn: PgPooledConnection) -> QueryResult<MiniprogramUser> {
    task::spawn_blocking(move || diesel::update(&u).set(&u).get_result(&conn))
        .await
        .unwrap()
}
