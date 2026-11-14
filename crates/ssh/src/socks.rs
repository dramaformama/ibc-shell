//! Minimal SOCKS5 server side for dynamic forwards (RFC 1928): no auth, CONNECT only.

use std::io;
use std::net::{Ipv4Addr, Ipv6Addr};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const VER: u8 = 5;
const NO_AUTH: u8 = 0;
const NO_METHOD: u8 = 0xFF;
const CMD_CONNECT: u8 = 1;
const REP_CMD_UNSUPPORTED: u8 = 7;
const REP_ATYP_UNSUPPORTED: u8 = 8;

fn bad(msg: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

/// Reads the greeting and the CONNECT request; returns the requested destination.
/// Unsupported requests get the matching SOCKS error reply before the error is returned.
pub(crate) async fn accept<S>(s: &mut S) -> io::Result<(String, u16)>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut head = [0u8; 2];
    s.read_exact(&mut head).await?;
    if head[0] != VER {
        return Err(bad("not socks5"));
    }
    let mut methods = vec![0u8; usize::from(head[1])];
    s.read_exact(&mut methods).await?;
    if !methods.contains(&NO_AUTH) {
        s.write_all(&[VER, NO_METHOD]).await?;
        return Err(bad("no acceptable auth method"));
    }
    s.write_all(&[VER, NO_AUTH]).await?;

    let mut req = [0u8; 4];
    s.read_exact(&mut req).await?;
    if req[0] != VER {
        return Err(bad("not socks5"));
    }
    if req[1] != CMD_CONNECT {
        reply(s, REP_CMD_UNSUPPORTED).await?;
        return Err(bad("only CONNECT is supported"));
    }
    let host = match req[3] {
        1 => {
            let mut a = [0u8; 4];
            s.read_exact(&mut a).await?;
            Ipv4Addr::from(a).to_string()
        }
        3 => {
            let mut len = [0u8; 1];
            s.read_exact(&mut len).await?;
            let mut name = vec![0u8; usize::from(len[0])];
            s.read_exact(&mut name).await?;
            match String::from_utf8(name) {
                Ok(n) if !n.is_empty() => n,
                _ => {
                    reply(s, REP_ATYP_UNSUPPORTED).await?;
                    return Err(bad("invalid domain name"));
                }
            }
        }
        4 => {
            let mut a = [0u8; 16];
            s.read_exact(&mut a).await?;
            Ipv6Addr::from(a).to_string()
        }
        _ => {
            reply(s, REP_ATYP_UNSUPPORTED).await?;
            return Err(bad("unknown address type"));
        }
    };
    let mut port = [0u8; 2];
    s.read_exact(&mut port).await?;
    Ok((host, u16::from_be_bytes(port)))
}

/// Sends a reply: `0` = success, otherwise a SOCKS5 error code (5 = connection refused).
pub(crate) async fn reply<S: AsyncWrite + Unpin>(s: &mut S, code: u8) -> io::Result<()> {
    s.write_all(&[VER, code, 0, 1, 0, 0, 0, 0, 0, 0]).await
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn run(input: &[u8]) -> (io::Result<(String, u16)>, Vec<u8>) {
        let (mut client, mut server) = tokio::io::duplex(1024);
        client.write_all(input).await.unwrap();
        let r = accept(&mut server).await;
        drop(server);
        let mut out = Vec::new();
        client.read_to_end(&mut out).await.unwrap();
        (r, out)
    }

    #[tokio::test]
    async fn connect_domain() {
        let mut msg = vec![5, 1, 0, 5, 1, 0, 3, 11];
        msg.extend_from_slice(b"example.com");
        msg.extend_from_slice(&443u16.to_be_bytes());
        let (r, out) = run(&msg).await;
        assert_eq!(r.unwrap(), ("example.com".into(), 443));
        assert_eq!(out, [5, 0]);
    }

    #[tokio::test]
    async fn connect_ipv4_and_ipv6() {
        let (r, _) = run(&[5, 1, 0, 5, 1, 0, 1, 10, 0, 0, 7, 0, 22]).await;
        assert_eq!(r.unwrap(), ("10.0.0.7".into(), 22));
        let mut msg = vec![5, 1, 0, 5, 1, 0, 4];
        msg.extend_from_slice(&Ipv6Addr::LOCALHOST.octets());
        msg.extend_from_slice(&80u16.to_be_bytes());
        let (r, _) = run(&msg).await;
        assert_eq!(r.unwrap(), ("::1".into(), 80));
    }

    #[tokio::test]
    async fn rejects_auth_only_clients() {
        let (r, out) = run(&[5, 1, 2]).await;
        assert!(r.is_err());
        assert_eq!(out, [5, 0xFF]);
    }

    #[tokio::test]
    async fn rejects_bind_and_udp() {
        let (r, out) = run(&[5, 1, 0, 5, 2, 0, 1, 1, 2, 3, 4, 0, 80]).await;
        assert!(r.is_err());
        assert_eq!(out[..4], [5, 0, 5, REP_CMD_UNSUPPORTED]);
    }

    #[tokio::test]
    async fn rejects_socks4_and_empty_domain() {
        assert!(run(&[4, 1, 0, 80]).await.0.is_err());
        let (r, _) = run(&[5, 1, 0, 5, 1, 0, 3, 0, 0, 80]).await;
        assert!(r.is_err());
    }
}
