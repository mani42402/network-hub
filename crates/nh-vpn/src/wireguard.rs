//! WireGuard profile model, parser, and canonical emitter (WP-M1.2).
//!
//! A WireGuard config is INI-shaped: an `[Interface]` section and one or more
//! `[Peer]` sections of `Key = Value` lines. Unlike OpenVPN it has no directive
//! that can execute code, so hardening here is *strict validation* — every key
//! is checked to be base64 32-byte material, every address and endpoint parsed —
//! plus the shared size caps ([`crate::limits`]).
//!
//! Secrets (`PrivateKey`, `PresharedKey`) are held in [`Secret`] so they never
//! land in a log or `Debug`; the daemon extracts them to the keychain on import
//! (WP-M1.1). [`WireguardProfile::to_config`] re-emits a canonical config —
//! import → emit → re-import is stable.

use std::fmt::Write as _;
use std::net::IpAddr;

use nh_types::Secret;
use zeroize::Zeroizing;

use crate::error::{VpnError, VpnResult};
use crate::limits;
use crate::types::{Cidr, Endpoint};

/// A parsed WireGuard profile: one interface and its peers.
#[derive(Debug, Clone)]
pub struct WireguardProfile {
    /// The local interface configuration.
    pub interface: WgInterface,
    /// The peers (at least one).
    pub peers: Vec<WgPeer>,
}

/// The `[Interface]` section.
#[derive(Debug, Clone)]
pub struct WgInterface {
    /// Our private key (base64). Secret.
    pub private_key: Secret<String>,
    /// Interface addresses (`Address`).
    pub addresses: Vec<Cidr>,
    /// DNS servers to use while up (`DNS`).
    pub dns: Vec<IpAddr>,
    /// Optional MTU override.
    pub mtu: Option<u16>,
    /// Optional listen port.
    pub listen_port: Option<u16>,
}

/// A `[Peer]` section.
#[derive(Debug, Clone)]
pub struct WgPeer {
    /// The peer's public key (base64). Not secret.
    pub public_key: String,
    /// Optional pre-shared key (base64). Secret.
    pub preshared_key: Option<Secret<String>>,
    /// The peer endpoint (`Endpoint`), if pinned.
    pub endpoint: Option<Endpoint>,
    /// Networks routed to this peer (`AllowedIPs`).
    pub allowed_ips: Vec<Cidr>,
    /// Keepalive interval in seconds, if enabled.
    pub persistent_keepalive: Option<u16>,
}

impl WireguardProfile {
    /// Parses a WireGuard config from raw bytes, enforcing the size/encoding caps
    /// first.
    ///
    /// # Errors
    /// [`VpnError`] on any oversized, mis-encoded, malformed, or invalid input.
    pub fn parse(input: &[u8]) -> VpnResult<Self> {
        limits::check_size(input)?;
        let text = limits::decode_text(input)?;
        Self::parse_str(text)
    }

    /// Parses a WireGuard config from already-decoded text.
    ///
    /// # Errors
    /// See [`parse`](Self::parse).
    pub fn parse_str(text: &str) -> VpnResult<Self> {
        let mut interface: Option<WgInterfaceBuilder> = None;
        let mut peers: Vec<WgPeerBuilder> = Vec::new();
        let mut section = Section::None;

        for raw in text.lines() {
            let line = strip_comment(raw).trim();
            if line.is_empty() {
                continue;
            }

            if let Some(name) = section_header(line) {
                match name.as_str() {
                    "interface" => {
                        if interface.is_some() {
                            return Err(VpnError::DuplicateSection("Interface"));
                        }
                        interface = Some(WgInterfaceBuilder::default());
                        section = Section::Interface;
                    }
                    "peer" => {
                        if peers.len() >= limits::MAX_PEERS {
                            return Err(VpnError::TooLarge {
                                what: "peers",
                                actual: peers.len() + 1,
                                limit: limits::MAX_PEERS,
                            });
                        }
                        peers.push(WgPeerBuilder::default());
                        section = Section::Peer;
                    }
                    other => {
                        return Err(VpnError::Malformed(format!("unknown section [{other}]")));
                    }
                }
                continue;
            }

            let (key, value) = split_kv(line)?;
            let key = key.to_ascii_lowercase();
            // The `ok_or` arms cannot fire — entering a section installs its
            // builder — but expressing that as `?` keeps this function
            // panic-free rather than relying on an `expect`.
            match section {
                Section::None => {
                    return Err(VpnError::Malformed(format!(
                        "`{key}` appears before any section"
                    )));
                }
                Section::Interface => interface
                    .as_mut()
                    .ok_or(VpnError::MissingField("[Interface]"))?
                    .set(&key, value)?,
                Section::Peer => peers
                    .last_mut()
                    .ok_or(VpnError::MissingField("[Peer]"))?
                    .set(&key, value)?,
            }
        }

        let interface = interface
            .ok_or(VpnError::MissingField("[Interface]"))?
            .build()?;
        if peers.is_empty() {
            return Err(VpnError::MissingField("[Peer]"));
        }
        let peers = peers
            .into_iter()
            .map(WgPeerBuilder::build)
            .collect::<VpnResult<Vec<_>>>()?;

        Ok(Self { interface, peers })
    }

    /// Re-emits a canonical WireGuard config. Exposes the secret keys because a
    /// config file is exactly how the tunnel is brought up; keep the result out
    /// of logs.
    #[must_use]
    pub fn to_config(&self) -> String {
        let mut out = String::new();
        let i = &self.interface;
        out.push_str("[Interface]\n");
        let _ = writeln!(out, "PrivateKey = {}", i.private_key.expose());
        if !i.addresses.is_empty() {
            let _ = writeln!(out, "Address = {}", join(&i.addresses));
        }
        if !i.dns.is_empty() {
            let dns = i
                .dns
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(out, "DNS = {dns}");
        }
        if let Some(mtu) = i.mtu {
            let _ = writeln!(out, "MTU = {mtu}");
        }
        if let Some(port) = i.listen_port {
            let _ = writeln!(out, "ListenPort = {port}");
        }

        for peer in &self.peers {
            out.push_str("\n[Peer]\n");
            let _ = writeln!(out, "PublicKey = {}", peer.public_key);
            if let Some(psk) = &peer.preshared_key {
                let _ = writeln!(out, "PresharedKey = {}", psk.expose());
            }
            if let Some(ep) = &peer.endpoint {
                let _ = writeln!(out, "Endpoint = {ep}");
            }
            if !peer.allowed_ips.is_empty() {
                let _ = writeln!(out, "AllowedIPs = {}", join(&peer.allowed_ips));
            }
            if let Some(k) = peer.persistent_keepalive {
                let _ = writeln!(out, "PersistentKeepalive = {k}");
            }
        }
        out
    }
}

// --- parsing internals -----------------------------------------------------

enum Section {
    None,
    Interface,
    Peer,
}

#[derive(Default)]
struct WgInterfaceBuilder {
    private_key: Option<String>,
    addresses: Vec<Cidr>,
    dns: Vec<IpAddr>,
    mtu: Option<u16>,
    listen_port: Option<u16>,
}

impl WgInterfaceBuilder {
    fn set(&mut self, key: &str, value: &str) -> VpnResult<()> {
        match key {
            "privatekey" => {
                validate_key(value, "PrivateKey")?;
                self.private_key = Some(value.to_owned());
            }
            "address" => self.addresses = parse_cidr_list(value)?,
            "dns" => self.dns = parse_ip_list(value)?,
            "mtu" => self.mtu = Some(parse_u16(value, "MTU")?),
            "listenport" => self.listen_port = Some(parse_u16(value, "ListenPort")?),
            // Directives wg-quick handles itself (Table, PreUp, ...) are not part
            // of the tunnel crypto; ignore unknown interface keys rather than
            // fail, but never carry them forward.
            _ => {}
        }
        Ok(())
    }

    fn build(self) -> VpnResult<WgInterface> {
        Ok(WgInterface {
            private_key: Secret::new(
                self.private_key
                    .ok_or(VpnError::MissingField("PrivateKey"))?,
            ),
            addresses: self.addresses,
            dns: self.dns,
            mtu: self.mtu,
            listen_port: self.listen_port,
        })
    }
}

#[derive(Default)]
struct WgPeerBuilder {
    public_key: Option<String>,
    preshared_key: Option<String>,
    endpoint: Option<Endpoint>,
    allowed_ips: Vec<Cidr>,
    persistent_keepalive: Option<u16>,
}

impl WgPeerBuilder {
    fn set(&mut self, key: &str, value: &str) -> VpnResult<()> {
        match key {
            "publickey" => {
                validate_key(value, "PublicKey")?;
                self.public_key = Some(value.to_owned());
            }
            "presharedkey" => {
                validate_key(value, "PresharedKey")?;
                self.preshared_key = Some(value.to_owned());
            }
            "endpoint" => self.endpoint = Some(value.parse()?),
            "allowedips" => self.allowed_ips = parse_cidr_list(value)?,
            "persistentkeepalive" => {
                self.persistent_keepalive = parse_keepalive(value)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn build(self) -> VpnResult<WgPeer> {
        Ok(WgPeer {
            public_key: self.public_key.ok_or(VpnError::MissingField("PublicKey"))?,
            preshared_key: self.preshared_key.map(Secret::new),
            endpoint: self.endpoint,
            allowed_ips: self.allowed_ips,
            persistent_keepalive: self.persistent_keepalive,
        })
    }
}

/// Strips a `#`/`;` comment. A key is base64 or an address, neither of which
/// contains `#` or `;`, so stripping from the first occurrence is safe.
fn strip_comment(line: &str) -> &str {
    let cut = line.find(['#', ';']).unwrap_or(line.len());
    &line[..cut]
}

/// Returns the lowercased section name if `line` is a `[Section]` header.
fn section_header(line: &str) -> Option<String> {
    let inner = line.strip_prefix('[')?.strip_suffix(']')?;
    Some(inner.trim().to_ascii_lowercase())
}

fn split_kv(line: &str) -> VpnResult<(&str, &str)> {
    let (k, v) = line
        .split_once('=')
        .ok_or_else(|| VpnError::Malformed(format!("expected `key = value`, got `{line}`")))?;
    let k = k.trim();
    if k.is_empty() {
        return Err(VpnError::Malformed("empty key".to_owned()));
    }
    Ok((k, v.trim()))
}

/// Validates base64 32-byte key material without retaining the decoded bytes
/// (zeroized on drop). The base64 text itself stays in the model.
fn validate_key(value: &str, field: &'static str) -> VpnResult<()> {
    use base64::Engine as _;
    let decoded = Zeroizing::new(
        base64::engine::general_purpose::STANDARD
            .decode(value.trim())
            .map_err(|_| VpnError::InvalidKey(field))?,
    );
    if decoded.len() != 32 {
        return Err(VpnError::InvalidKey(field));
    }
    Ok(())
}

fn parse_cidr_list(value: &str) -> VpnResult<Vec<Cidr>> {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::parse)
        .collect()
}

fn parse_ip_list(value: &str) -> VpnResult<Vec<IpAddr>> {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<IpAddr>()
                .map_err(|_| VpnError::InvalidAddress(s.to_owned()))
        })
        .collect()
}

fn parse_u16(value: &str, field: &'static str) -> VpnResult<u16> {
    value.trim().parse().map_err(|_| VpnError::InvalidNumber {
        field,
        value: value.to_owned(),
    })
}

/// `PersistentKeepalive` is a seconds count, or `off`/`0` for disabled.
fn parse_keepalive(value: &str) -> VpnResult<Option<u16>> {
    let v = value.trim();
    if v.eq_ignore_ascii_case("off") || v == "0" {
        return Ok(None);
    }
    Ok(Some(parse_u16(v, "PersistentKeepalive")?))
}

fn join(cidrs: &[Cidr]) -> String {
    cidrs
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // Valid base64 32-byte keys (all-zero, all-one bytes) — shape-valid, which is
    // all the parser checks.
    const KEY_A: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    const KEY_B: &str = "//////////////////////////////////////////8=";

    fn sample() -> String {
        format!(
            "[Interface]\n\
             PrivateKey = {KEY_A}\n\
             Address = 10.0.0.2/32, fd00::2/128\n\
             DNS = 1.1.1.1, 8.8.8.8\n\
             MTU = 1420\n\
             ListenPort = 51820\n\
             \n\
             [Peer]\n\
             PublicKey = {KEY_B}\n\
             PresharedKey = {KEY_A}\n\
             Endpoint = vpn.example.com:51820\n\
             AllowedIPs = 0.0.0.0/0, ::/0\n\
             PersistentKeepalive = 25\n"
        )
    }

    #[test]
    fn parses_a_full_profile() {
        let p = WireguardProfile::parse(sample().as_bytes()).unwrap();
        assert_eq!(p.interface.private_key.expose(), KEY_A);
        assert_eq!(p.interface.addresses.len(), 2);
        assert_eq!(p.interface.dns.len(), 2);
        assert_eq!(p.interface.mtu, Some(1420));
        assert_eq!(p.interface.listen_port, Some(51820));
        assert_eq!(p.peers.len(), 1);
        let peer = &p.peers[0];
        assert_eq!(peer.public_key, KEY_B);
        assert!(peer.preshared_key.is_some());
        assert_eq!(peer.endpoint.as_ref().unwrap().port, 51820);
        assert_eq!(peer.allowed_ips.len(), 2);
        assert_eq!(peer.persistent_keepalive, Some(25));
    }

    #[test]
    fn import_emit_reimport_is_stable() {
        // Emission is idempotent: parse → emit → parse → emit yields the same
        // canonical config. (Profiles hold secrets and so are deliberately not
        // `PartialEq`; comparing the canonical text is the stability contract.)
        let p1 = WireguardProfile::parse(sample().as_bytes()).unwrap();
        let emitted = p1.to_config();
        let p2 = WireguardProfile::parse(emitted.as_bytes()).unwrap();
        assert_eq!(emitted, p2.to_config(), "round-trip must be stable");
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let cfg = format!(
            "# a comment\n[Interface]\nPrivateKey = {KEY_A}  ; trailing\n\n[Peer]\nPublicKey = {KEY_B}\nAllowedIPs = 0.0.0.0/0\n"
        );
        let p = WireguardProfile::parse(cfg.as_bytes()).unwrap();
        assert_eq!(p.interface.private_key.expose(), KEY_A);
        assert_eq!(p.peers.len(), 1);
    }

    #[test]
    fn rejects_missing_private_key() {
        let cfg = format!(
            "[Interface]\nAddress = 10.0.0.2/32\n[Peer]\nPublicKey = {KEY_B}\nAllowedIPs = 0.0.0.0/0\n"
        );
        assert!(matches!(
            WireguardProfile::parse(cfg.as_bytes()),
            Err(VpnError::MissingField("PrivateKey"))
        ));
    }

    #[test]
    fn rejects_no_peers() {
        let cfg = format!("[Interface]\nPrivateKey = {KEY_A}\n");
        assert!(matches!(
            WireguardProfile::parse(cfg.as_bytes()),
            Err(VpnError::MissingField("[Peer]"))
        ));
    }

    #[test]
    fn rejects_bad_key() {
        let cfg = format!(
            "[Interface]\nPrivateKey = not-base64!!\n[Peer]\nPublicKey = {KEY_B}\nAllowedIPs = 0.0.0.0/0\n"
        );
        assert!(matches!(
            WireguardProfile::parse(cfg.as_bytes()),
            Err(VpnError::InvalidKey("PrivateKey"))
        ));
    }

    #[test]
    fn rejects_short_key() {
        // Valid base64, but decodes to 3 bytes, not 32.
        let cfg = format!(
            "[Interface]\nPrivateKey = YWJj\n[Peer]\nPublicKey = {KEY_B}\nAllowedIPs = 0.0.0.0/0\n"
        );
        assert!(matches!(
            WireguardProfile::parse(cfg.as_bytes()),
            Err(VpnError::InvalidKey("PrivateKey"))
        ));
    }

    #[test]
    fn rejects_duplicate_interface() {
        let cfg = format!("[Interface]\nPrivateKey = {KEY_A}\n[Interface]\nPrivateKey = {KEY_B}\n");
        assert!(matches!(
            WireguardProfile::parse(cfg.as_bytes()),
            Err(VpnError::DuplicateSection("Interface"))
        ));
    }

    #[test]
    fn rejects_key_before_section() {
        let cfg = format!("PrivateKey = {KEY_A}\n[Interface]\n");
        assert!(matches!(
            WireguardProfile::parse(cfg.as_bytes()),
            Err(VpnError::Malformed(_))
        ));
    }

    #[test]
    fn rejects_too_many_peers() {
        let mut cfg = format!("[Interface]\nPrivateKey = {KEY_A}\n");
        let peer_block = format!("[Peer]\nPublicKey = {KEY_B}\nAllowedIPs = 0.0.0.0/0\n");
        for _ in 0..=limits::MAX_PEERS {
            cfg.push_str(&peer_block);
        }
        assert!(matches!(
            WireguardProfile::parse(cfg.as_bytes()),
            Err(VpnError::TooLarge { what: "peers", .. })
        ));
    }

    #[test]
    fn keepalive_off_is_none() {
        let cfg = format!(
            "[Interface]\nPrivateKey = {KEY_A}\n[Peer]\nPublicKey = {KEY_B}\nAllowedIPs = 0.0.0.0/0\nPersistentKeepalive = off\n"
        );
        let p = WireguardProfile::parse(cfg.as_bytes()).unwrap();
        assert_eq!(p.peers[0].persistent_keepalive, None);
    }

    #[test]
    fn debug_does_not_leak_private_key() {
        let p = WireguardProfile::parse(sample().as_bytes()).unwrap();
        let dbg = format!("{p:?}");
        assert!(!dbg.contains(KEY_A), "Debug must not print the private key");
    }
}
