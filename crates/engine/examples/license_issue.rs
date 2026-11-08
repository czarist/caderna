//! Offline issuer. Keep the private PKCS#8 file outside the repository.
//! `license_issue keygen private.pk8 public.txt`
//! `license_issue sign private.pk8 claims.json license.txt`

use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use ring::{
    rand::SystemRandom,
    signature::{Ed25519KeyPair, KeyPair},
};
use std::{fs, io::Write, path::Path};

fn create(path: &str, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(Path::new(path))?.write_all(bytes)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, private, public] if command == "keygen" => {
            let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new())
                .map_err(|_| "key generation failed")?;
            let key =
                Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).map_err(|_| "invalid generated key")?;
            create(private, pkcs8.as_ref())?;
            create(
                public,
                STANDARD.encode(key.public_key().as_ref()).as_bytes(),
            )?;
        }
        [command, private, claims, output] if command == "sign" => {
            let key = Ed25519KeyPair::from_pkcs8(&fs::read(private)?)
                .map_err(|_| "invalid private key")?;
            let claims: kg_engine::license::Claims = serde_json::from_slice(&fs::read(claims)?)?;
            let payload = serde_json::to_vec(&claims)?;
            let token = format!(
                "{}.{}",
                URL_SAFE_NO_PAD.encode(&payload),
                URL_SAFE_NO_PAD.encode(
                    key.sign(&kg_engine::license::signing_message(&payload))
                        .as_ref()
                )
            );
            kg_engine::license::verify(
                &token,
                &STANDARD.encode(key.public_key().as_ref()),
                &claims.brand,
            )?;
            create(output, token.as_bytes())?;
        }
        _ => {
            return Err(
                "usage: license_issue keygen PRIVATE PUBLIC | sign PRIVATE CLAIMS OUTPUT".into(),
            );
        }
    }
    Ok(())
}
