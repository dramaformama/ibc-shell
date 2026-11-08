//! TLS certificate expiry on the server (ADR-012): reads certificate files that web servers and
//! Let's Encrypt keep (Let's Encrypt `live`, nginx / Apache `ssl_certificate` paths, Caddy, the
//! common `/root/cert` of VPN panels, acme.sh) with `openssl x509`, plus the certificate the
//! server really presents on 443 for those names. Only public certificate fields leave
//! the server; files are never printed and private keys are never opened as keys.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::docker::sh;
use crate::stats_parse::sanitize;
use crate::{Connection, SshError};

const TIMEOUT: Duration = Duration::from_secs(15);
const MAX_OUTPUT: usize = 256 * 1024;

/// One certificate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct CertInfo {
    /// File on the server.
    pub path: String,
    /// Subject CN followed by the other DNS names (≤ 20).
    pub names: Vec<String>,
    /// Issuer organisation (or CN).
    pub issuer: Option<String>,
    /// Expiry, Unix ms.
    pub not_after: i64,
}

/// Result of [`cert_list`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct CertReport {
    /// Certificates, soonest expiry first (same certificate in two places listed once).
    pub certs: Vec<CertInfo>,
    /// Places that exist but the login user may not read (needs root).
    pub denied: u32,
    /// `openssl` is not installed.
    pub no_openssl: bool,
}

const SCRIPT: &str = r#"export LC_ALL=C
command -v openssl >/dev/null 2>&1 || { echo '@@IBC:NOOPENSSL@@'; exit 0; }
for d in /etc/letsencrypt/live /root/cert /root/.acme.sh /var/lib/caddy/.local/share/caddy/certificates; do
  [ -e "$d" ] && [ ! -r "$d" -o ! -x "$d" ] && echo '@@IBC:DENIED@@'
done
FILES=$({
  ls -d /etc/letsencrypt/live/*/cert.pem 2>/dev/null
  grep -rhsE '^[[:space:]]*(ssl_certificate|SSLCertificateFile)[[:space:]]' /etc/nginx /etc/apache2 /etc/httpd 2>/dev/null | awk '{print $2}' | tr -d ';"'"'"
  find /opt /srv /root /home -maxdepth 4 -name '*.conf' -size -200k 2>/dev/null | head -300 | xargs grep -hsE '^[[:space:]]*ssl_certificate[[:space:]]' 2>/dev/null | awk '{print $2}' | tr -d ';"'"'"
  ls -d /var/lib/caddy/.local/share/caddy/certificates/*/*/*.crt 2>/dev/null
  ls -d /root/cert/*.crt /root/cert/*/fullchain.pem /root/cert/*/*.crt /root/cert/*/cert.pem 2>/dev/null
  ls -d /root/.acme.sh/*/fullchain.cer /root/.acme.sh/*/*.cer /home/*/.acme.sh/*/fullchain.cer 2>/dev/null
  ls -d /etc/ssl/*/*.crt /etc/x-ui/*.crt /etc/ssl/private/*.pem 2>/dev/null
} | grep -v '\$' | sort -u | head -64)
show() {
  openssl x509 -noout -enddate -subject -issuer 2>/dev/null
}
printf '%s\n' "$FILES" | while IFS= read -r f; do
  [ -f "$f" ] || continue
  if [ ! -r "$f" ]; then echo '@@IBC:DENIED@@'; continue; fi
  echo '@@IBC:CERT@@'
  echo "path=$f"
  openssl x509 -in "$f" -noout -enddate -subject -issuer 2>/dev/null
  openssl x509 -in "$f" -noout -text 2>/dev/null | grep -o 'DNS:[^,]*' | head -20
done
# What the server really presents on 443 right now (Docker, a panel, any other way of issuing).
printf '%s\n' "$FILES" | while IFS= read -r f; do
  [ -r "$f" ] && openssl x509 -in "$f" -noout -text 2>/dev/null | grep -o 'DNS:[^,]*'
done | sed 's/^DNS://' | grep -E '^[A-Za-z0-9][A-Za-z0-9.-]*$' | sort -u | head -8 | while IFS= read -r n; do
  c=$(echo | { command -v timeout >/dev/null 2>&1 && timeout 5 openssl s_client -connect 127.0.0.1:443 -servername "$n" || openssl s_client -connect 127.0.0.1:443 -servername "$n"; } 2>/dev/null | openssl x509 2>/dev/null)
  [ -n "$c" ] || continue
  echo '@@IBC:CERT@@'
  echo "path=live:$n:443"
  printf '%s\n' "$c" | show
  printf '%s\n' "$c" | openssl x509 -noout -text 2>/dev/null | grep -o 'DNS:[^,]*' | head -20
done
echo '@@IBC:END@@'
"#;

/// Finds certificates and their expiry. Read-only.
///
/// Errors: [`SshError::Timeout`] after 15 s, network errors of the exec channel.
pub async fn cert_list(conn: &Connection) -> Result<CertReport, SshError> {
    let out = tokio::time::timeout(TIMEOUT, conn.exec(&sh(SCRIPT, &[]), MAX_OUTPUT))
        .await
        .map_err(|_| SshError::Timeout)??;
    Ok(parse_certs(&String::from_utf8_lossy(&out.stdout)))
}

/// Days since 1970-01-01 of a civil date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `Jan  5 12:00:00 2027 GMT` → Unix ms.
fn parse_date(text: &str) -> Option<i64> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let parts: Vec<&str> = text.split_whitespace().collect();
    let [mon, day, time, year, ..] = parts[..] else {
        return None;
    };
    let month = i64::try_from(MONTHS.iter().position(|x| *x == mon)?).ok()? + 1;
    let day: i64 = day.parse().ok()?;
    let year: i64 = year.parse().ok()?;
    let hms: Vec<i64> = time
        .split(':')
        .map(|x| x.parse().ok())
        .collect::<Option<_>>()?;
    let [hour, min, sec] = hms[..] else {
        return None;
    };
    Some((days_from_civil(year, month, day) * 86_400 + hour * 3600 + min * 60 + sec) * 1000)
}

/// Value of `key` in a distinguished name, either `CN = a, O = b` or `/CN=a/O=b`.
fn dn_field(dn: &str, key: &str) -> Option<String> {
    dn.split([',', '/'])
        .filter_map(|part| part.split_once('='))
        .find(|(k, _)| k.trim() == key)
        .map(|(_, v)| sanitize(v.trim().trim_matches('"')))
        .filter(|v| !v.is_empty())
}

pub(crate) fn parse_certs(text: &str) -> CertReport {
    let mut rep = CertReport {
        certs: vec![],
        denied: 0,
        no_openssl: false,
    };
    if text.contains("@@IBC:NOOPENSSL@@") {
        rep.no_openssl = true;
        return rep;
    }
    rep.denied = u32::try_from(text.matches("@@IBC:DENIED@@").count()).unwrap_or(u32::MAX);
    for block in text.split("@@IBC:CERT@@").skip(1) {
        let block = block.split("@@IBC:").next().unwrap_or("");
        let (mut path, mut after, mut cn, mut issuer, mut names) =
            (String::new(), None, None, None, Vec::<String>::new());
        for line in block.lines().map(str::trim) {
            if let Some(v) = line.strip_prefix("path=") {
                path = sanitize(v);
            } else if let Some(v) = line.strip_prefix("notAfter=") {
                after = parse_date(v);
            } else if let Some(v) = line.strip_prefix("subject=") {
                cn = dn_field(v, "CN");
            } else if let Some(v) = line.strip_prefix("issuer=") {
                issuer = dn_field(v, "O").or_else(|| dn_field(v, "CN"));
            } else if let Some(v) = line.strip_prefix("DNS:") {
                let v = sanitize(v);
                if !v.is_empty() && !names.contains(&v) {
                    names.push(v);
                }
            }
        }
        let Some(not_after) = after else { continue };
        if let Some(cn) = cn {
            names.retain(|n| *n != cn);
            names.insert(0, cn);
        }
        names.truncate(20);
        let dup = rep
            .certs
            .iter()
            .any(|c| c.not_after == not_after && c.names == names);
        if !dup {
            rep.certs.push(CertInfo {
                path,
                names,
                issuer,
                not_after,
            });
        }
    }
    rep.certs.sort_by_key(|c| c.not_after);
    // A certificate that a later-expiring one covers completely (same names, renewed another
    // way) is an old copy: the renewed one is what runs.
    let all = rep.certs.clone();
    rep.certs.retain(|c| {
        !all.iter()
            .any(|d| d.not_after > c.not_after && c.names.iter().all(|n| d.names.contains(n)))
    });
    rep
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(parse_date("Jan  1 00:00:00 1970 GMT"), Some(0));
        // 2027-01-05T12:00:00Z
        assert_eq!(
            parse_date("Jan  5 12:00:00 2027 GMT"),
            Some(1_799_150_400_000)
        );
        assert_eq!(parse_date("garbage"), None);
    }

    #[test]
    fn parses_and_dedupes() {
        let text = "@@IBC:DENIED@@\n\
            @@IBC:CERT@@\npath=/etc/letsencrypt/live/a.example/cert.pem\n\
            notAfter=Feb  1 10:00:00 2027 GMT\nsubject=CN = a.example\n\
            issuer=C = US, O = Let's Encrypt, CN = R11\nDNS:a.example\nDNS:www.a.example\n\
            @@IBC:CERT@@\npath=/etc/nginx/ssl/a.pem\n\
            notAfter=Feb  1 10:00:00 2027 GMT\nsubject=CN = a.example\nDNS:a.example\nDNS:www.a.example\n\
            @@IBC:CERT@@\npath=/root/cert/b/fullchain.pem\n\
            notAfter=Dec 30 00:00:00 2026 GMT\nsubject= /CN=b.example\nissuer= /O=ZeroSSL/CN=x\n\
            @@IBC:END@@\n";
        let r = parse_certs(text);
        assert_eq!(r.denied, 1);
        assert_eq!(r.certs.len(), 2);
        assert_eq!(r.certs[0].names, vec!["b.example".to_string()]);
        assert_eq!(r.certs[0].issuer.as_deref(), Some("ZeroSSL"));
        assert_eq!(r.certs[1].names, vec!["a.example", "www.a.example"]);
        assert_eq!(r.certs[1].issuer.as_deref(), Some("Let's Encrypt"));
        assert!(parse_certs("@@IBC:NOOPENSSL@@\n").no_openssl);
    }

    #[test]
    fn a_renewed_certificate_hides_the_old_copy() {
        let text = "@@IBC:CERT@@\npath=/etc/letsencrypt/live/a/cert.pem\n\
            notAfter=Jul 25 10:00:00 2026 GMT\nsubject=CN = a.example\nDNS:a.example\n\
            @@IBC:CERT@@\npath=live:a.example:443\n\
            notAfter=Feb  1 10:00:00 2027 GMT\nsubject=CN = a.example\nDNS:a.example\nDNS:www.a.example\n\
            @@IBC:CERT@@\npath=/root/cert/b.pem\n\
            notAfter=Jul 25 10:00:00 2026 GMT\nsubject=CN = b.example\nDNS:b.example\n\
            @@IBC:END@@\n";
        let r = parse_certs(text);
        let paths: Vec<_> = r.certs.iter().map(|c| c.path.as_str()).collect();
        assert_eq!(paths, vec!["/root/cert/b.pem", "live:a.example:443"]);
    }
}
