use super::models::{AnyResult, MiniprogramUser};
use crate::db_connection::PgPooledConnection;
use diesel::prelude::*;
use tokio::task;

pub async fn find(_open_id: String, conn: PgPooledConnection) -> QueryResult<MiniprogramUser> {
    task::spawn_blocking(move || {
        use crate::diesel_schema::wechat_miniprogram_users::dsl::*;
        wechat_miniprogram_users
            .filter(open_id.eq(_open_id))
            .first::<MiniprogramUser>(&conn)
    })
    .await
    .unwrap()
}

pub async fn create(
    _open_id: String,
    userid: i32,
    conn: PgPooledConnection,
) -> AnyResult<MiniprogramUser> {
    task::spawn_blocking(move || {
        use crate::diesel_schema::wechat_miniprogram_users::dsl::*;
        let insert = MiniprogramUser {
            open_id: _open_id,
            user_id: userid,
            ..Default::default()
        };
        diesel::insert_into(wechat_miniprogram_users)
            .values(&insert)
            .get_result::<MiniprogramUser>(&conn)
            .map_err(Into::into)
    })
    .await
    .unwrap()
}
pub async fn update(u: MiniprogramUser, conn: PgPooledConnection) -> QueryResult<MiniprogramUser> {
    task::spawn_blocking(move || diesel::update(&u).set(&u).get_result(&conn))
        .await
        .unwrap()
}

// 生存环境，是不允许删除用户资料的，
// 所以这里限定只能在测试里面使用
#[cfg(test)]
pub async fn delete(_openid: &str, conn: PgPooledConnection) -> QueryResult<()> {
    let _openid = _openid.to_owned();
    task::spawn_blocking(move || {
        use crate::diesel_schema::wechat_miniprogram_users::dsl::*;
        diesel::delete(wechat_miniprogram_users.filter(open_id.eq(_openid)))
            .execute(&conn)
            .map(|_| ())
    })
    .await
    .unwrap()
}
