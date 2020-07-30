use super::models::{AnyResult, MiniprogramUser};
use crate::db_connection::PgPooledConnection;
use async_std::task;
use diesel::prelude::*;

pub async fn find(open_id: String, conn: PgPooledConnection) -> QueryResult<MiniprogramUser> {
    task::spawn_blocking(move || {
        use crate::diesel_schema::wechat_miniprogram_users;
        wechat_miniprogram_users::table
            .filter(wechat_miniprogram_users::open_id.eq(open_id))
            .first(&conn)
    })
    .await
}

pub async fn create(
    open_id: String,
    user_id: i32,
    conn: PgPooledConnection,
) -> AnyResult<MiniprogramUser> {
    task::spawn_blocking(move || {
        use crate::diesel_schema::wechat_miniprogram_users;
        let insert = MiniprogramUser {
            open_id,
            user_id,
            ..Default::default()
        };
        diesel::insert_into(wechat_miniprogram_users::table)
            .values(&insert)
            .get_result(&conn)
            .map_err(Into::into)
    })
    .await
}
pub async fn update(u: MiniprogramUser, conn: PgPooledConnection) -> QueryResult<MiniprogramUser> {
    task::spawn_blocking(move || diesel::update(&u).set(&u).get_result(&conn)).await
}

// 生存环境，是不允许删除用户资料的，
// 所以这里限定只能在测试里面使用
#[cfg(test)]
pub async fn delete(open_id: &str, conn: PgPooledConnection) -> QueryResult<()> {
    let open_id = open_id.to_owned();
    task::spawn_blocking(move || {
        use crate::diesel_schema::wechat_miniprogram_users;
        diesel::delete(
            wechat_miniprogram_users::table.filter(wechat_miniprogram_users::open_id.eq(open_id)),
        )
        .execute(&conn)
        .map(|_| ())
    })
    .await
}
