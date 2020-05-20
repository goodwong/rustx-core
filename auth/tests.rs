use base_62;

use crate::auth::graphql::Context;
use crate::auth::models::User;
use crate::auth::repository as user_repository;
use crate::auth::service::AuthService;
use crate::db_connection::{establish_connection, PgPool};
use crate::wechat::miniprogram::models::MiniprogramUser;
use crate::wechat::miniprogram::repository as miniprogram_repository;

pub type TestResult<O> = Result<O, Box<dyn std::error::Error + Send + Sync>>;

pub fn db_pool() -> PgPool {
    use dotenv::dotenv;
    use std::env;

    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    establish_connection(database_url)
}

pub fn auth_service(pool: PgPool) -> AuthService {
    let cipher_key = "Q+mvRWovv4NHANIuevkXtAmC3r2wp8bjyrKCPTgm7m0=";
    AuthService::new(pool, cipher_key)
}

pub async fn mock_user(username: &str, pool: PgPool) -> TestResult<User> {
    let insert = user_repository::InsertUser {
        username: username.to_owned(),
        name: "for test".to_owned(),
        avatar: Default::default(),
    };

    user_repository::create_user(insert, pool.get()?).await
}

pub async fn clear_mock_user(username: &str, pool: PgPool) -> TestResult<()> {
    user_repository::delete_user_by_username(username, pool)
        .await
        .map_err(Into::into)
}

pub async fn mock_miniprogram_user(
    open_id: &str,
    user_id: i32,
    pool: PgPool,
) -> TestResult<MiniprogramUser> {
    miniprogram_repository::create(open_id.to_owned(), user_id, pool.get()?).await
}

pub async fn clear_mock_miniprogram_user(open_id: &str, pool: PgPool) -> TestResult<()> {
    miniprogram_repository::delete(open_id, pool.get()?)
        .await
        .map_err(Into::into)
}

pub async fn mock_context(db_pool: PgPool) -> TestResult<Context> {
    let auth = auth_service(db_pool.clone());
    let identity = auth.get_identity("an invalid token").await?;

    use crate::api::wechat_miniprogram::{Config, Miniprogram};
    let miniprogram = Miniprogram::new(Config::from_env());

    Ok(Context::new(
        db_pool,
        identity,
        miniprogram,
        Default::default(),
    ))
}

#[async_std::test]
async fn clear_mock_user_test() {
    let pool = db_pool();

    clear_mock_user("not_exist_users_username", pool)
        .await
        .unwrap();
}

#[test]
fn base62_vs_base64() {
    let input = "123456781234567812345678";
    let encoded = base_62::encode(input.as_bytes());
    println!("{} = {}", input, encoded);

    let input = "123456781234567812345679";
    let encoded = base_62::encode(input.as_bytes());
    println!("{} = {}", input, encoded);

    let input = "123456781234567912345679";
    let encoded = base_62::encode(input.as_bytes());
    println!("{} = {}", input, encoded);

    let input = "223456781234567812345678";
    let encoded = base_62::encode(input.as_bytes());
    println!("{} = {}", input, encoded);

    // 9位
    let input = "1X1c1f1Z0";
    let encoded = base_62::encode(input.as_bytes());
    println!("{} = {}", input, encoded);
    let input = "1u1-1$1D1";
    let encoded = base_62::encode(input.as_bytes());
    println!("{} = {}", input, encoded);

    let input = "X11c1f1Z0";
    let encoded = base_62::encode(input.as_bytes());
    println!("{} = {}", input, encoded);
    let input = "u11-1$1D1";
    let encoded = base_62::encode(input.as_bytes());
    println!("{} = {}", input, encoded);

    let input = ",1181x10";
    let encoded = base_62::encode(input.as_bytes());
    println!("{} = {}", input, encoded);
    let input = "u1181x10";
    let encoded = base_62::encode(input.as_bytes());
    println!("{} = {}", input, encoded);

    let plain = base_62::decode(&encoded).unwrap();
    println!("plain = {:x?}", plain);
    println!("plain = {}", std::str::from_utf8(&plain).unwrap());
}
/*
#[test]
fn nacl() {
    use nacl::secret_box;
    let m = b"0123456701234567012345670123456701234567";
    let n = b"012345670123456701234567";
    let k = b"01234567012345670123456701234567";
    println!("m: {:02X?}", &m[..]);
    println!("n: {:02X?}", n);
    println!("k: {:02X?}", k);
    let ciphed = secret_box::pack(m, n, k).expect("加密失败");
    println!("ciphed: {:02X?}", ciphed);
}

#[test]
fn aes_gcm() {
    use aead::{generic_array::GenericArray, Aead, NewAead};
    use aes_gcm::Aes256Gcm; // Or `Aes128Gcm`

    let key = GenericArray::clone_from_slice(b"an example very very secret key.");
    let aead = Aes256Gcm::new(key);

    println!("aes gcm:");
    let nonce = GenericArray::from_slice(b"unique nonce"); // 96-bits; unique per message
    let message = b"plaintext message";
    let ciphertext = aead
        .encrypt(nonce, message.as_ref())
        .expect("encryption failure!");
    let plaintext = aead
        .decrypt(nonce, ciphertext.as_ref())
        .expect("decryption failure!");

    println!("message:  {}", std::str::from_utf8(message).unwrap());
    println!("encrypted:{}", base_62::encode(message));
    println!("message:{:02X?}", message);
    println!("cipher: {:02X?}", ciphertext);
    //println!("plain:  {:02X?}", plaintext);
    assert_eq!(&plaintext, message);
}

#[test]
fn sodium() {
    use sodiumoxide::crypto::secretbox;
    //use std::convert::TryInto;
    let key = secretbox::Key(*b"01234567012345670123456701234567"); // expected an array with a fixed size of 32 elements, found one with 33 elements
    let nonce = secretbox::gen_nonce();
    let plaintext = b"some data";
    let ciphertext = secretbox::seal(plaintext, &nonce, &key);
    let their_plaintext = secretbox::open(&ciphertext, &nonce, &key).unwrap();
    assert!(plaintext == &their_plaintext[..]);

    println!("nonce:      {:02X?}", nonce);
    println!("plaintext:  {:02X?}", plaintext);
    println!("ciphertext: {:02X?}", ciphertext);
}

#[test]
fn chacha20_poly1305() {
    use aead::{generic_array::GenericArray, Aead, NewAead};
    use chacha20poly1305::ChaCha20Poly1305; // Or `XChaCha20Poly1305`
    let key = GenericArray::clone_from_slice(b"an example very very secret key."); // 32-bytes
    let aead = ChaCha20Poly1305::new(key);
    let nonce = GenericArray::from_slice(b"unique nonce"); // 12-bytes; unique per message
    let ciphertext = aead
        .encrypt(nonce, b"plaintext message".as_ref())
        .expect("encryption failure!");
    let plaintext = aead
        .decrypt(nonce, ciphertext.as_ref())
        .expect("decryption failure!");
    assert_eq!(&plaintext, b"plaintext message");
}
*/

#[test]
fn aes_gcm_siv() {
    use aead::{generic_array::GenericArray, Aead, NewAead};
    use aes_gcm_siv::Aes256GcmSiv; // Or `Aes128GcmSiv`

    let key = GenericArray::clone_from_slice(b"an example very very secret key.");
    let aead = Aes256GcmSiv::new(key);

    println!("aes gcm siv:");
    let nonce = GenericArray::from_slice(b"unique nonce"); // 96-bits; unique per message
    let message = b"plaintext message";
    let ciphertext = aead
        .encrypt(nonce, message.as_ref())
        .expect("encryption failure!");
    let plaintext = aead
        .decrypt(nonce, ciphertext.as_ref())
        .expect("decryption failure!");

    println!("message:  {}", std::str::from_utf8(message).unwrap());
    println!("encrypted:{}", base_62::encode(message));
    println!("message:{:02X?}", message);
    println!("cipher: {:02X?}", ciphertext);
    //println!("plain:  {:02X?}", plaintext);
    assert_eq!(&plaintext, message);
}
