//! Small network value types shared by the profile parsers.

use std::fmt;
use std::net::IpAddr;
use std::str::FromStr;

use crate::error::VpnError;

/// An address + prefix length (`10.0.0.2/32`, `::/0`). Used for WireGuard
/// `Address`/`AllowedIPs` and split-tunnel routes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cidr {
    /// The network/host address.
    pub addr: IpAddr,
    /// The prefix length in bits (≤ 32 for v4, ≤ 128 for v6).
    pub prefix: u8,
}

impl FromStr for Cidr {
    type Err = VpnError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (addr_str, prefix) = match s.split_once('/') {
            Some((a, p)) => {
                let prefix: u8 = p
                    .parse()
                    .map_err(|_| VpnError::InvalidAddress(s.to_owned()))?;
                (a, prefix)
            }
            // A bare address means a host route (/32 or /128).
            None => (s, u8::MAX),
        };
        let addr: IpAddr = addr_str
            .parse()
            .map_err(|_| VpnError::InvalidAddress(s.to_owned()))?;

        let max = match addr {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        let prefix = if prefix == u8::MAX { max } else { prefix };
        if prefix > max {
            return Err(VpnError::InvalidAddress(s.to_owned()));
        }
        Ok(Self { addr, prefix })
    }
}

impl fmt::Display for Cidr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.addr, self.prefix)
    }
}

/// A remote `host:port` — the WireGuard `Endpoint` or an OpenVPN `remote`. The
/// host is kept as text because it may be a hostname resolved later (E-9), not
/// only a literal address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    /// Hostname or IP literal.
    pub host: String,
    /// UDP/TCP port.
    pub port: u16,
}

impl FromStr for Endpoint {
    type Err = VpnError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let invalid = || VpnError::InvalidEndpoint(s.to_owned());

        // Bracketed IPv6 literal: [::1]:51820
        let (host, port_str) = if let Some(rest) = s.strip_prefix('[') {
            let (host, tail) = rest.split_once(']').ok_or_else(invalid)?;
            let port = tail.strip_prefix(':').ok_or_else(invalid)?;
            // The host must be a valid IPv6 literal.
            if host.parse::<std::net::Ipv6Addr>().is_err() {
                return Err(invalid());
            }
            (host.to_owned(), port)
        } else {
            // host:port — split on the LAST colon so a bare IPv6 without brackets
            // is rejected (it has multiple colons and no valid host).
            let (host, port) = s.rsplit_once(':').ok_or_else(invalid)?;
            if host.is_empty() || host.contains(':') {
                return Err(invalid());
            }
            (host.to_owned(), port)
        };

        let port: u16 = port_str.parse().map_err(|_| invalid())?;
        if port == 0 {
            return Err(invalid());
        }
        Ok(Self { host, port })
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Re-bracket an IPv6 literal so the output round-trips.
        if self.host.parse::<std::net::Ipv6Addr>().is_ok() {
            write!(f, "[{}]:{}", self.host, self.port)
        } else {
            write!(f, "{}:{}", self.host, self.port)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn cidr_v4_round_trips() {
        let c: Cidr = "10.0.0.2/24".parse().unwrap();
        assert_eq!(c.prefix, 24);
        assert_eq!(c.to_string(), "10.0.0.2/24");
    }

    #[test]
    fn cidr_bare_address_is_host_route() {
        assert_eq!("10.0.0.2".parse::<Cidr>().unwrap().prefix, 32);
        assert_eq!("fd00::2".parse::<Cidr>().unwrap().prefix, 128);
    }

    #[test]
    fn cidr_rejects_oversized_prefix() {
        assert!("10.0.0.2/33".parse::<Cidr>().is_err());
        assert!("fd00::/129".parse::<Cidr>().is_err());
        assert!("not-an-ip/24".parse::<Cidr>().is_err());
    }

    #[test]
    fn endpoint_host_port() {
        let e: Endpoint = "vpn.example.com:51820".parse().unwrap();
        assert_eq!(e.host, "vpn.example.com");
        assert_eq!(e.port, 51820);
        assert_eq!(e.to_string(), "vpn.example.com:51820");
    }

    #[test]
    fn endpoint_ipv6_bracketed_round_trips() {
        let e: Endpoint = "[2001:db8::1]:51820".parse().unwrap();
        assert_eq!(e.host, "2001:db8::1");
        assert_eq!(e.to_string(), "[2001:db8::1]:51820");
    }

    #[test]
    fn endpoint_rejects_bad_input() {
        assert!("no-port".parse::<Endpoint>().is_err());
        assert!("host:0".parse::<Endpoint>().is_err());
        assert!("host:70000".parse::<Endpoint>().is_err());
        assert!("2001:db8::1:51820".parse::<Endpoint>().is_err()); // unbracketed v6
    }
}
