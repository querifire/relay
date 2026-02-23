use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProxyProtocol {
    Http,
    #[allow(dead_code)]
    Https,
    Socks4,
    Socks5,
}

impl fmt::Display for ProxyProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyProtocol::Http => write!(f, "http"),
            ProxyProtocol::Https => write!(f, "https"),
            ProxyProtocol::Socks4 => write!(f, "socks4"),
            ProxyProtocol::Socks5 => write!(f, "socks5"),
        }
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
        Self { host, port, protocol }
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
