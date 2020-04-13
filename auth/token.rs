use super::error::AuthResult;
use aead::{generic_array::GenericArray, Aead, NewAead};
use aes_gcm_siv::Aes256GcmSiv;
use chrono::{DateTime, Duration, TimeZone, Utc};
use rand::Rng;
use std::convert::TryInto;

// key&nonce length
pub(crate) const KEY_LENGTH: usize = 32;
pub(crate) const NONCE_LENGTH: usize = 12;
// life time
pub(super) const TOKEN_LIFE_HOURS: i64 = 1;
pub(super) const REFRESH_TOKEN_LIFE_DAYS: i64 = 30;

pub(crate) struct Token {
    pub nonce: [u8; NONCE_LENGTH],
    pub user_id: i64,
    pub refresh_token_id: i64,
    pub issued_at: i64,
}
impl Token {
    pub fn from_string(token: &str, key: &[u8; KEY_LENGTH]) -> AuthResult<Token> {
        let mut data = base_62::decode(token).map_err(|e| format!("{}", e))?;
        // data.split_at_mut 会Panic，要提前检查data长度
        if data.len() < NONCE_LENGTH {
            return Err("invalid data length".into());
        }
        let (nonce, cipher) = data.split_at_mut(NONCE_LENGTH);
        let key = GenericArray::clone_from_slice(key);
        let aead = Aes256GcmSiv::new(key);
        let nonce_bytes = GenericArray::from_slice(nonce);
        let plain = aead
            .decrypt(nonce_bytes, cipher.as_ref())
            .map_err(|e| format!("{:?}", e))?;
        if plain.len() != 24 {
            return Err("invalid token length".into());
        }
        let user_id = i64::from_be_bytes(plain[..8].try_into()?);
        let refresh_token_id = i64::from_be_bytes(plain[8..16].try_into()?);
        let issued_at = i64::from_be_bytes(plain[16..24].try_into()?);
        Ok(Token {
            nonce: nonce.as_ref().try_into()?,
            user_id,
            refresh_token_id,
            issued_at,
        })
    }
    pub fn to_string(&self, key: &[u8; KEY_LENGTH]) -> AuthResult<(String, DateTime<Utc>)> {
        // pack
        use std::io::Write;
        let mut buf: Vec<u8> = Vec::with_capacity(24);
        let Token {
            nonce,
            user_id,
            refresh_token_id,
            issued_at,
        } = self;
        buf.write_all(&user_id.to_be_bytes())?;
        buf.write_all(&refresh_token_id.to_be_bytes())?;
        buf.write_all(&issued_at.to_be_bytes())?;
        // encrypt
        //println!("buf:  {:02X?}", &buf);
        //println!("nonce:{:02X?}", cipher_nonce);
        //println!("key:  {:02X?}", cipher_key);
        let key = GenericArray::clone_from_slice(key);
        let aead = Aes256GcmSiv::new(key);
        let nonce_bytes = GenericArray::from_slice(nonce);
        let cipher = aead
            .encrypt(nonce_bytes, buf.as_ref())
            .map_err(|e| format!("{:?}", e))?;
        // encoding
        let sealed = base_62::encode(&[nonce.as_ref(), &cipher].concat());
        let expires = Utc::now() + Duration::hours(TOKEN_LIFE_HOURS);
        Ok((sealed, expires))
    }

    pub fn is_expired(&self) -> bool {
        Utc.timestamp(self.issued_at, 0) + Duration::hours(TOKEN_LIFE_HOURS) < Utc::now()
    }

    pub fn nonce_pair() -> ([u8; NONCE_LENGTH], String) {
        let nonce = loop {
            let n = rand::thread_rng().gen::<[u8; NONCE_LENGTH]>();
            if !n.contains(&0u8) {
                break n;
            }
        };
        // hash 函数有两个地方会Error：
        // 1. COST值超出有效范围，这里使用bcrypt::DEFAULT_COST
        // 2. password(即这里的nonce)包含\0字符，上面通过loop规避了，
        // 因此这里可以安全的unwrap()
        let hash = bcrypt::hash(nonce, bcrypt::DEFAULT_COST).unwrap();
        (nonce, hash)
    }
}

#[cfg(test)]
mod tests {
    use super::Token;

    #[test]
    fn token_to_string() {
        let nonce = b"12345678_234";
        let key = b"12345678_2345678_2345678_2345678";
        let (token_str, expires) = Token {
            nonce: *nonce,
            user_id: 1005i64,
            refresh_token_id: 12i64,
            issued_at: 34861628346i64,
        }
        .to_string(&*key)
        .expect("失败啦");
        println!("token: {}, expires: {}", token_str, expires);
    }
    #[test]
    fn token_from_string() {
        let nonce = b"12345678_234";
        let key = b"12345678_2345678_2345678_2345678";
        let (token_str, expires) = Token {
            nonce: *nonce,
            user_id: 1005i64,
            refresh_token_id: 12i64,
            issued_at: 34861628346i64,
        }
        .to_string(&*key)
        .expect("失败啦");
        println!("token: {}, expires: {}", token_str, expires);

        let Token {
            nonce,
            user_id,
            refresh_token_id,
            issued_at,
        } = Token::from_string(&token_str, key).unwrap();
        println!(
            "decrypt: uid{}, tid{}, iat{}, nonce{:x?}",
            user_id, refresh_token_id, issued_at, nonce
        );
    }
}
