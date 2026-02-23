use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProxyProtocol {
    Http,
    #[allow(dead_code)]
    Https,
    Socks4,
    Socks5,
    /// Tor circuit — reserved for future implementation
    Tor,
}

impl fmt::Display for ProxyProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyProtocol::Http => write!(f, "http"),
            ProxyProtocol::Https => write!(f, "https"),
            ProxyProtocol::Socks4 => write!(f, "socks4"),
            ProxyProtocol::Socks5 => write!(f, "socks5"),
            ProxyProtocol::Tor => write!(f, "tor"),
        }
    }
}

/// Operating mode for a proxy instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProxyMode {
    /// Automatically find and rotate upstream proxies.
    Auto,
    /// Use a manually specified upstream proxy.
    Manual,
    /// Route through Tor — reserved for future implementation.
    Tor,
}

impl fmt::Display for ProxyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyMode::Auto => write!(f, "Auto"),
            ProxyMode::Manual => write!(f, "Manual"),
            ProxyMode::Tor => write!(f, "Tor"),
        }
    }
}

impl Default for ProxyMode {
    fn default() -> Self {
        ProxyMode::Auto
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Proxy {
    pub host: String,
    pub port: u16,
    pub protocol: ProxyProtocol,
}

impl Proxy {
    pub fn new(host: String, port: u16, protocol: ProxyProtocol) -> Self {
        Self {
            host,
            port,
            protocol,
        }
    }

    #[allow(dead_code)]
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    #[allow(dead_code)]
    pub fn url(&self) -> String {
        format!("{}://{}:{}", self.protocol, self.host, self.port)
    }
}

impl fmt::Display for Proxy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}:{}", self.protocol, self.host, self.port)
    }
}
