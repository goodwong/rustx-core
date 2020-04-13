use base_62;

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
