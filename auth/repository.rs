use super::error::AuthResult;
use super::models::{User, UserToken};
use crate::db_connection::PgPooledConnection;
use crate::diesel_schema::{user_tokens, users};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use async_std::task;

// user...
pub async fn find_user(user_id: i32, conn: PgPooledConnection) -> QueryResult<User> {
    task::spawn_blocking(move || {
        use crate::diesel_schema::users::dsl::*;
        users.filter(id.eq(user_id)).first::<User>(&conn)
    })
    .await
}
pub async fn find_user_by_username(
    _username: String,
    conn: PgPooledConnection,
) -> QueryResult<User> {
    task::spawn_blocking(move || {
        use crate::diesel_schema::users::dsl::*;
        users.filter(username.eq(_username)).first::<User>(&conn)
    })
    .await
}
pub async fn find_user_by_token(_token: String) -> QueryResult<User> {
    // let token = find_token(token_str);
    // find_user(token.user_id)
    todo!()
}

#[derive(Insertable, Default)]
#[table_name = "users"]
pub struct InsertUser {
    pub username: String,
    pub name: String,
    pub avatar: String,
}
pub async fn create_user(user: InsertUser, conn: PgPooledConnection) -> AuthResult<User> {
    task::spawn_blocking(move || {
        use crate::diesel_schema::users::dsl::*;
        diesel::insert_into(users)
            .values(&user)
            .get_result::<User>(&conn)
            .map_err(Into::into)
    })
    .await
}
pub async fn update_user() -> AuthResult<()> {
    todo!()
}
pub async fn update_user_name() -> AuthResult<()> {
    todo!()
}
pub async fn list_user() -> AuthResult<Vec<User>> {
    todo!()
}
pub async fn count_user() -> AuthResult<i32> {
    todo!()
}

// 生存环境，是不允许删除用户的，
// 所以这里限定只能在测试里面使用
#[cfg(test)]
use crate::db_connection::PgPool;
#[cfg(test)]
pub async fn delete_user_by_username(_username: &str, pool: PgPool) -> AuthResult<()> {
    let _username = _username.to_owned();
    match find_user_by_username(_username.to_owned(), pool.get()?).await {
        Err(diesel::NotFound) => Ok(()),
        Err(e) => Err(Box::new(e)),
        Ok(user) => {
            task::spawn_blocking(move || {
                // 先删除tokens
                use crate::diesel_schema::user_tokens::dsl::*;
                diesel::delete(user_tokens.filter(user_id.eq(user.id)))
                    .execute(&pool.get()?)
                    .map_err(|e| format!("{}", e))?;

                // 再删除users
                use crate::diesel_schema::users::dsl::*;
                diesel::delete(users.filter(username.eq(_username)))
                    .execute(&pool.get()?)
                    .map(|_| ())
                    .map_err(|e| format!("{}", e).into())
            })
            .await
        }
    }
}

// token...
pub async fn find_refresh_token(
    refresh_token_id: i32,
    conn: PgPooledConnection,
) -> QueryResult<UserToken> {
    task::spawn_blocking(move || {
        use crate::diesel_schema::user_tokens::dsl::*;
        user_tokens
            .filter(id.eq(refresh_token_id)) // id.eq(...).and(deleted_at.is_null())
            .filter(deleted_at.is_null())
            .first::<UserToken>(&conn)
    })
    .await
}
#[derive(Insertable)]
#[table_name = "user_tokens"]
pub struct InsertToken {
    pub user_id: i32,
    pub device: String,
    pub hash: String,
}
pub async fn create_refresh_token(
    token: InsertToken,
    conn: PgPooledConnection,
) -> AuthResult<UserToken> {
    task::spawn_blocking(move || {
        use crate::diesel_schema::user_tokens::dsl::*;
        diesel::insert_into(user_tokens)
            .values(&token)
            .get_result::<UserToken>(&conn)
            .map_err(Into::into)
    })
    .await
}
pub async fn destroy_refresh_token(token_id: i32, conn: PgPooledConnection) -> AuthResult<()> {
    task::spawn_blocking(move || {
        use crate::diesel_schema::user_tokens::dsl::*;
        diesel::update(
            user_tokens
                .filter(id.eq(token_id))
                .filter(deleted_at.is_null()),
        )
        .set(deleted_at.eq(Some(Utc::now())))
        .execute(&conn)
        .map(|_| ())
        .map_err(Into::into)
    })
    .await
}
pub async fn renew_refresh_token(
    token_id: i32,
    hash_str: String,
    conn: PgPooledConnection,
) -> AuthResult<DateTime<Utc>> {
    task::spawn_blocking(move || {
        use crate::diesel_schema::user_tokens::dsl::*;
        let now: DateTime<Utc> = Utc::now();
        diesel::update(
            user_tokens
                .filter(id.eq(token_id))
                .filter(deleted_at.is_null()),
        )
        .set((hash.eq(hash_str), issued_at.eq(now)))
        .execute(&conn)
        .map(|_| now)
        .map_err(Into::into)
    })
    .await
}
