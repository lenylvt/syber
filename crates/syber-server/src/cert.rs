//! TLS certificate generation for the Syber server.
//! Uses self-signed ECDSA P-256 (rcgen 0.13 API).
//! The SHA-256 fingerprint is shown to the user and entered on the client.

use anyhow::{Context, Result};
use ring::digest::{digest, SHA256};

pub struct ServerCert {
    pub cert_pem:  String,
    pub key_pem:   String,
    pub cert_hash: String, // SHA-256 hex — displayed in UI / entered in client
    pub cert_der:  Vec<u8>,
}

impl ServerCert {
    /// Generate a fresh self-signed ECDSA P-256 certificate.
    pub fn generate() -> Result<Self> {
        let key_pair = rcgen::KeyPair::generate()
            .context("generate key pair")?;

        let cert = rcgen::CertificateParams::new(vec!["syber-server".to_string()])
            .context("cert params")?
            .self_signed(&key_pair)
            .context("self-sign cert")?;

        let cert_pem  = cert.pem();
        let key_pem   = key_pair.serialize_pem();
        let cert_der  = cert.der().to_vec();

        let hash_bytes = digest(&SHA256, &cert_der);
        let cert_hash  = hex::encode(hash_bytes.as_ref());

        Ok(Self { cert_pem, key_pem, cert_hash, cert_der })
    }

    /// Load a previously-generated cert from PEM strings.
    pub fn from_pem(cert_pem: &str, key_pem: &str) -> Result<Self> {
        // Decode PEM to get DER bytes for fingerprint
        let cert_der = pem_to_der(cert_pem, "CERTIFICATE")
            .context("decode cert PEM")?;

        let hash_bytes = digest(&SHA256, &cert_der);
        let cert_hash  = hex::encode(hash_bytes.as_ref());

        Ok(Self {
            cert_pem:  cert_pem.to_string(),
            key_pem:   key_pem.to_string(),
            cert_hash,
            cert_der,
        })
    }

    /// Format cert hash for display: "A3:4F:2E:..." (colon-separated uppercase pairs)
    pub fn format_fingerprint(hex_hash: &str) -> String {
        hex_hash
            .chars()
            .collect::<Vec<_>>()
            .chunks(2)
            .map(|c| c.iter().collect::<String>().to_uppercase())
            .collect::<Vec<_>>()
            .join(":")
    }

    /// Convert cert + key PEM to kynet/rustls types for kyproto::common::CommonServer.
    pub fn to_kynet_types(
        &self,
    ) -> Result<(Vec<kyproto::cert::Certificate>, kyproto::cert::PrivateKey)> {
        use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

        let cert_der = self.cert_der.clone();
        let certificate: CertificateDer<'static> = CertificateDer::from(cert_der);

        let key_der = pem_to_der(&self.key_pem, "PRIVATE KEY")
            .or_else(|_| pem_to_der(&self.key_pem, "EC PRIVATE KEY"))
            .context("decode key PEM")?;
        let private_key: PrivateKeyDer<'static> =
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_der));

        Ok((vec![certificate], private_key))
    }
}

/// Parse a PEM block with the given label to raw DER bytes.
fn pem_to_der(pem: &str, label: &str) -> Result<Vec<u8>> {
    let begin = format!("-----BEGIN {}-----", label);
    let end   = format!("-----END {}-----",   label);

    let start = pem.find(&begin)
        .ok_or_else(|| anyhow::anyhow!("PEM begin marker not found: {begin}"))?
        + begin.len();
    let finish = pem.find(&end)
        .ok_or_else(|| anyhow::anyhow!("PEM end marker not found"))?;

    let b64: String = pem[start..finish]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    let bytes = {
        // Simple base64 decode
        base64_decode(&b64)?
    };
    Ok(bytes)
}

fn base64_decode(input: &str) -> Result<Vec<u8>> {
    // Use the standard alphabet
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut table = [255u8; 256];
    for (i, &c) in ALPHABET.iter().enumerate() {
        table[c as usize] = i as u8;
    }

    let bytes: Vec<u8> = input.bytes().filter(|&b| b != b'=').collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0u32;

    for &b in &bytes {
        let v = table[b as usize];
        anyhow::ensure!(v != 255, "invalid base64 character: {b}");
        buf  = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(out)
}
