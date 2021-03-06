use super::error::AuthResult;
use super::models::{User, UserToken};
use crate::db_connection::PgPooledConnection;
use crate::diesel_schema::{user_tokens, users};
use async_std::task;
use chrono::{DateTime, Utc};
use diesel::prelude::*;

// user...
pub async fn find_user(id: i32, conn: PgPooledConnection) -> QueryResult<User> {
    task::spawn_blocking(move || users::table.find(id).first(&conn)).await
}
pub async fn find_user_by_username(
    username: String,
    conn: PgPooledConnection,
) -> QueryResult<User> {
    task::spawn_blocking(move || {
        //use crate::diesel_schema::users;
        users::table
            .filter(users::username.eq(username))
            .first(&conn)
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
            .get_result(&conn)
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
pub async fn delete_user_by_username(username: &str, pool: PgPool) -> AuthResult<()> {
    let username = username.to_owned();
    match find_user_by_username(username.to_owned(), pool.get()?).await {
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
                //use crate::diesel_schema::users;
                diesel::delete(users::table.filter(users::username.eq(username)))
                    .execute(&pool.get()?)
                    .map(|_| ())
                    .map_err(|e| format!("{}", e).into())
            })
            .await
        }
    }
}

// token...
pub async fn find_refresh_token(id: i32, conn: PgPooledConnection) -> QueryResult<UserToken> {
    task::spawn_blocking(move || {
        user_tokens::table
            .find(id)
            .filter(user_tokens::deleted_at.is_null()) // soft delete
            // 另一种写法 .filter(id.eq(...).and(deleted_at.is_null()))
            .first(&conn)
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
            .get_result(&conn)
            .map_err(Into::into)
    })
    .await
}
pub async fn destroy_refresh_token(id: i32, conn: PgPooledConnection) -> AuthResult<()> {
    task::spawn_blocking(move || {
        diesel::update(
            user_tokens::table
                .find(id)
                .filter(user_tokens::deleted_at.is_null()), // soft delete
        )
        .set(user_tokens::deleted_at.eq(Some(Utc::now())))
        .execute(&conn)
        .map(|_| ())
        .map_err(Into::into)
    })
    .await
}
pub async fn renew_refresh_token(
    id: i32,
    hash_str: String,
    conn: PgPooledConnection,
) -> AuthResult<DateTime<Utc>> {
    task::spawn_blocking(move || {
        let now: DateTime<Utc> = Utc::now();
        diesel::update(
            user_tokens::table
                .find(id)
                .filter(user_tokens::deleted_at.is_null()), // soft delete
        )
        .set((
            user_tokens::hash.eq(hash_str),
            user_tokens::issued_at.eq(now),
        ))
        .execute(&conn)
        .map(|_| now)
        .map_err(Into::into)
    })
    .await
}
