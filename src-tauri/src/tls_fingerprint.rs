use rand::seq::SliceRandom;
use rand::Rng;

use rustls::crypto::aws_lc_rs as crypto_provider;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TlsFingerprintPreset {
    Random,
    Chrome,
    Firefox,
    Safari,
    Default,
}

impl Default for TlsFingerprintPreset {
    fn default() -> Self {
        TlsFingerprintPreset::Default
    }
}

impl std::fmt::Display for TlsFingerprintPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsFingerprintPreset::Random => write!(f, "Random"),
            TlsFingerprintPreset::Chrome => write!(f, "Chrome"),
            TlsFingerprintPreset::Firefox => write!(f, "Firefox"),
            TlsFingerprintPreset::Safari => write!(f, "Safari"),
            TlsFingerprintPreset::Default => write!(f, "Default"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsFingerprintConfig {
    pub enabled: bool,
    pub preset: TlsFingerprintPreset,
}

impl Default for TlsFingerprintConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            preset: TlsFingerprintPreset::Default,
        }
    }
}

/// Cipher suite ordering for different browser presets.
/// These represent the TLS 1.3 + TLS 1.2 cipher suite preferences.
fn chrome_cipher_order() -> Vec<rustls::SupportedCipherSuite> {
    use crypto_provider::cipher_suite;
    vec![
        cipher_suite::TLS13_AES_128_GCM_SHA256,
        cipher_suite::TLS13_AES_256_GCM_SHA384,
        cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
        cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
        cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
        cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
        cipher_suite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
        cipher_suite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
        cipher_suite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
    ]
}

fn firefox_cipher_order() -> Vec<rustls::SupportedCipherSuite> {
    use crypto_provider::cipher_suite;
    vec![
        cipher_suite::TLS13_AES_128_GCM_SHA256,
        cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
        cipher_suite::TLS13_AES_256_GCM_SHA384,
        cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
        cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
        cipher_suite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
        cipher_suite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
        cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
        cipher_suite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
    ]
}

fn safari_cipher_order() -> Vec<rustls::SupportedCipherSuite> {
    use crypto_provider::cipher_suite;
    vec![
        cipher_suite::TLS13_AES_128_GCM_SHA256,
        cipher_suite::TLS13_AES_256_GCM_SHA384,
        cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
        cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
        cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
        cipher_suite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
        cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
        cipher_suite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
        cipher_suite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
    ]
}

fn random_cipher_order() -> Vec<rustls::SupportedCipherSuite> {
    use crypto_provider::cipher_suite;
    let mut rng = rand::rng();

    let mut tls13 = vec![
        cipher_suite::TLS13_AES_128_GCM_SHA256,
        cipher_suite::TLS13_AES_256_GCM_SHA384,
        cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
    ];
    tls13.shuffle(&mut rng);

    let mut tls12 = vec![
        cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
        cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
        cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
        cipher_suite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
        cipher_suite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
        cipher_suite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
    ];
    tls12.shuffle(&mut rng);

    let mut all = tls13;
    all.extend(tls12);
    all
}

/// Build a rustls `ClientConfig` with cipher suites ordered according to the preset.
pub fn build_tls_config(config: &TlsFingerprintConfig) -> rustls::ClientConfig {
    let cipher_suites = if !config.enabled {
        crypto_provider::DEFAULT_CIPHER_SUITES.to_vec()
    } else {
        match config.preset {
            TlsFingerprintPreset::Chrome => chrome_cipher_order(),
            TlsFingerprintPreset::Firefox => firefox_cipher_order(),
            TlsFingerprintPreset::Safari => safari_cipher_order(),
            TlsFingerprintPreset::Random => random_cipher_order(),
            TlsFingerprintPreset::Default => crypto_provider::DEFAULT_CIPHER_SUITES.to_vec(),
        }
    };

    let provider = rustls::crypto::CryptoProvider {
        cipher_suites,
        ..crypto_provider::default_provider()
    };

    // Standard certificate verification (do not use NoVerifier for direct connections).
    // Use build_tls_config_insecure_for_proxy() only when tunneling through upstream proxy.
    let root_store = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(provider))
        .with_safe_default_protocol_versions()
        .expect("TLS protocol versions")
        .with_root_certificates(root_store)
        .with_no_client_auth()
}

/// Build a TLS config that skips certificate verification (for proxy tunneling only).
/// Use only when connecting to upstream proxy; do not use for direct user traffic.
pub fn build_tls_config_insecure_for_proxy(config: &TlsFingerprintConfig) -> rustls::ClientConfig {
    let cipher_suites = if !config.enabled {
        crypto_provider::DEFAULT_CIPHER_SUITES.to_vec()
    } else {
        match config.preset {
            TlsFingerprintPreset::Chrome => chrome_cipher_order(),
            TlsFingerprintPreset::Firefox => firefox_cipher_order(),
            TlsFingerprintPreset::Safari => safari_cipher_order(),
            TlsFingerprintPreset::Random => random_cipher_order(),
            TlsFingerprintPreset::Default => crypto_provider::DEFAULT_CIPHER_SUITES.to_vec(),
        }
    };

    let provider = rustls::crypto::CryptoProvider {
        cipher_suites,
        ..crypto_provider::default_provider()
    };

    rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(provider))
        .with_safe_default_protocol_versions()
        .expect("TLS protocol versions")
        .dangerous()
        .with_custom_certificate_verifier(std::sync::Arc::new(NoVerifier))
        .with_no_client_auth()
}

/// Compute a simplified JA3-like fingerprint hash for display purposes.
pub fn compute_fingerprint_hash(config: &TlsFingerprintConfig) -> String {
    let mut rng = rand::rng();
    if config.enabled && config.preset == TlsFingerprintPreset::Random {
        // Generate a random-looking hash
        let bytes: [u8; 16] = rng.random();
        hex::encode_simple(&bytes)
    } else {
        let label = match config.preset {
            TlsFingerprintPreset::Chrome => "chrome-default",
            TlsFingerprintPreset::Firefox => "firefox-default",
            TlsFingerprintPreset::Safari => "safari-default",
            TlsFingerprintPreset::Random => "random",
            TlsFingerprintPreset::Default => "system-default",
        };
        simple_hash(label)
    }
}

fn simple_hash(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

mod hex {
    pub fn encode_simple(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

/// A certificate verifier that accepts any certificate (for proxy tunneling).
#[derive(Debug)]
struct NoVerifier;

impl rustls::client::danger::ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}
