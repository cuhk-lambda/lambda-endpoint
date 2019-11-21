use crate::config::*;
use crypto_api_osrandom::OsRandom;
use argon2::Config;

pub fn hashed_secret() -> String {
    let mut gen = OsRandom::secure_rng();
    let mut salt  = Vec::new();
    salt.resize(16, 0);
    gen.random(salt.as_mut_slice()).expect("unable to gen salt");
    let password = global_config().secret.as_bytes();
    let config = Config::default();
    argon2::hash_encoded(password, salt.as_slice(), &config).expect("unable to hash secret")
}

pub fn verify(encoded: &str) -> bool {
    match argon2::verify_encoded(encoded, global_config().secret.as_bytes()) {
        Ok(true) => true,
        _ => false
    }
}


#[test]
fn basic_hash() {
    let a = hashed_secret();
    let b = argon2::verify_encoded(a.as_str(), global_config().secret.as_bytes());
    assert!(b.unwrap());
    assert!(verify(a.as_str()))
}