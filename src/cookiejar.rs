use anyhow::{Context, Result};
use colored::Colorize;
use rusqlite::{params, Connection};
use std::path::PathBuf;

// ── Public types ─────────────────────────────────────────────────────────────

pub struct CookieJar {
    conn: Connection,
    pub path: PathBuf,
}

pub struct Cookie {
    pub id: i64,
    pub domain: String,
    pub path: String,
    pub name: String,
    pub value: String,
    pub expires: Option<i64>,
    pub secure: bool,
    pub http_only: bool,
}

// ── CookieJar ─────────────────────────────────────────────────────────────────

impl CookieJar {
    pub fn open(name: &str) -> Result<Self> {
        let path = jar_path(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create cookie jar directory: {}", parent.display()))?;
        }
        let conn = Connection::open(&path)
            .with_context(|| format!("Failed to open cookie jar: {}", path.display()))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS cookies (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                domain     TEXT    NOT NULL,
                path       TEXT    NOT NULL DEFAULT '/',
                name       TEXT    NOT NULL,
                value      TEXT    NOT NULL,
                expires    INTEGER,
                secure     INTEGER NOT NULL DEFAULT 0,
                http_only  INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
                updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
                UNIQUE(domain, path, name)
            );",
        )
        .context("Failed to initialise cookie database")?;
        Ok(Self { conn, path })
    }

    /// Returns `name=value` pairs for all cookies that match the given request.
    pub fn cookies_for(&self, domain: &str, path: &str, is_https: bool) -> Result<Vec<(String, String)>> {
        let now = unix_now();
        let mut stmt = self.conn.prepare(
            "SELECT name, value, domain, path, secure FROM cookies
             WHERE expires IS NULL OR expires > ?",
        )?;
        let rows = stmt.query_map([now], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)? != 0,
            ))
        })?;

        let mut out = Vec::new();
        for r in rows {
            let (name, value, c_domain, c_path, secure) = r?;
            if secure && !is_https {
                continue;
            }
            if !domain_matches(domain, &c_domain) {
                continue;
            }
            if !path_matches(path, &c_path) {
                continue;
            }
            out.push((name, value));
        }
        Ok(out)
    }

    /// Parses a `Set-Cookie` header value and upserts the cookie into the database.
    pub fn process_set_cookie(&self, header: &str, request_domain: &str, request_path: &str) -> Result<()> {
        let Some(c) = parse_set_cookie(header, request_domain, request_path) else {
            return Ok(());
        };
        // Max-Age=0 means delete the cookie
        if c.expires == Some(0) {
            self.conn.execute(
                "DELETE FROM cookies WHERE domain = ?1 AND path = ?2 AND name = ?3",
                params![c.domain, c.path, c.name],
            )?;
            return Ok(());
        }
        let now = unix_now();
        self.conn.execute(
            "INSERT INTO cookies (domain, path, name, value, expires, secure, http_only, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
             ON CONFLICT(domain, path, name) DO UPDATE SET
               value      = excluded.value,
               expires    = excluded.expires,
               secure     = excluded.secure,
               http_only  = excluded.http_only,
               updated_at = excluded.updated_at",
            params![
                c.domain, c.path, c.name, c.value, c.expires,
                c.secure as i64, c.http_only as i64, now
            ],
        )?;
        Ok(())
    }

    /// Returns all cookies ordered by domain → path → name.
    pub fn list(&self) -> Result<Vec<Cookie>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, domain, path, name, value, expires, secure, http_only
             FROM cookies ORDER BY domain, path, name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Cookie {
                id:        row.get(0)?,
                domain:    row.get(1)?,
                path:      row.get(2)?,
                name:      row.get(3)?,
                value:     row.get(4)?,
                expires:   row.get(5)?,
                secure:    row.get::<_, i64>(6)? != 0,
                http_only: row.get::<_, i64>(7)? != 0,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("Failed to read cookies")
    }

    /// Deletes the cookie with the given ID. Returns `true` if a row was removed.
    pub fn delete(&self, id: i64) -> Result<bool> {
        let n = self.conn.execute("DELETE FROM cookies WHERE id = ?1", [id])?;
        Ok(n > 0)
    }

    /// Inserts or updates a cookie from a `Set-Cookie`-style string.
    /// The string must include a `Domain=` attribute.
    pub fn set_from_str(&self, s: &str) -> Result<()> {
        let Some(c) = parse_set_cookie(s, "", "/") else {
            anyhow::bail!(
                "Invalid cookie format — expected: name=value; Domain=example.com; [Path=/]; [Secure]; [HttpOnly]; [Max-Age=N]"
            );
        };
        if c.domain.is_empty() {
            anyhow::bail!("Cookie must include a Domain= attribute");
        }
        let now = unix_now();
        self.conn.execute(
            "INSERT INTO cookies (domain, path, name, value, expires, secure, http_only, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
             ON CONFLICT(domain, path, name) DO UPDATE SET
               value      = excluded.value,
               expires    = excluded.expires,
               secure     = excluded.secure,
               http_only  = excluded.http_only,
               updated_at = excluded.updated_at",
            params![
                c.domain, c.path, c.name, c.value, c.expires,
                c.secure as i64, c.http_only as i64, now
            ],
        )?;
        Ok(())
    }

    /// Prints cookies as a formatted table to stdout.
    pub fn print_table(cookies: &[Cookie]) {
        if cookies.is_empty() {
            println!("(no cookies)");
            return;
        }

        const VAL_MAX: usize = 32;
        const EXP_W: usize = 19; // "YYYY-MM-DD HH:MM:SS"

        let id_w   = cookies.iter().map(|c| digit_count(c.id)).max().unwrap_or(2).max(2);
        let dom_w  = cookies.iter().map(|c| c.domain.len()).max().unwrap_or(6).max(6);
        let path_w = cookies.iter().map(|c| c.path.len()).max().unwrap_or(4).max(4);
        let name_w = cookies.iter().map(|c| c.name.len()).max().unwrap_or(4).max(4);

        let header = format!(
            " {:<id_w$}  {:<dom_w$}  {:<path_w$}  {:<name_w$}  {:<VAL_MAX$}  {:<EXP_W$}  Flags",
            "ID", "Domain", "Path", "Name", "Value", "Expires",
        );
        println!("{}", header.bold());
        println!("{}", "─".repeat(header.len()).dimmed());

        for c in cookies {
            let val = if c.value.len() > VAL_MAX {
                format!("{}…", &c.value[..VAL_MAX - 1])
            } else {
                c.value.clone()
            };
            let exp = match c.expires {
                None     => "session".to_string(),
                Some(ts) => format_unix_time(ts),
            };
            let flags = format!(
                "{}{}",
                if c.secure    { "S" } else { "-" },
                if c.http_only { "H" } else { "-" },
            );
            println!(
                " {:<id_w$}  {:<dom_w$}  {:<path_w$}  {:<name_w$}  {:<VAL_MAX$}  {:<EXP_W$}  {}",
                c.id, c.domain, c.path, c.name, val, exp, flags,
            );
        }
    }
}

// ── Set-Cookie parser ─────────────────────────────────────────────────────────

struct ParsedCookie {
    domain:    String,
    path:      String,
    name:      String,
    value:     String,
    expires:   Option<i64>,
    secure:    bool,
    http_only: bool,
}

fn parse_set_cookie(header: &str, request_domain: &str, request_path: &str) -> Option<ParsedCookie> {
    let mut iter = header.splitn(2, ';');
    let name_value = iter.next()?.trim();
    let (raw_name, raw_value) = name_value.split_once('=')?;
    let name  = raw_name.trim().to_string();
    let value = raw_value.trim().to_string();
    if name.is_empty() {
        return None;
    }

    let mut domain:    Option<String> = None;
    let mut path:      Option<String> = None;
    let mut max_age:   Option<i64>    = None;
    let mut expires_s: Option<String> = None;
    let mut secure    = false;
    let mut http_only = false;

    for attr in header.split(';').skip(1) {
        let attr = attr.trim();
        let (key, val) = if let Some(pos) = attr.find('=') {
            (attr[..pos].trim().to_lowercase(), Some(attr[pos + 1..].trim()))
        } else {
            (attr.to_lowercase(), None)
        };
        match key.as_str() {
            "domain" => {
                if let Some(v) = val {
                    // Presence of Domain attribute enables subdomain matching (RFC 6265 §5.2.3)
                    let d = v.trim_start_matches('.').to_lowercase();
                    domain = Some(format!(".{d}"));
                }
            }
            "path"     => path      = val.map(|v| v.to_string()),
            "max-age"  => max_age   = val.and_then(|v| v.parse().ok()),
            "expires"  => expires_s = val.map(|v| v.to_string()),
            "secure"   => secure    = true,
            "httponly" => http_only = true,
            _          => {}
        }
    }

    // Domain: if not set in header, use request host (exact-match only, no leading dot)
    let domain = domain.unwrap_or_else(|| request_domain.to_lowercase());
    let path   = path.unwrap_or_else(|| default_path(request_path));

    let expires = if let Some(age) = max_age {
        Some(if age <= 0 { 0 } else { unix_now() + age })
    } else if let Some(ref s) = expires_s {
        parse_http_date(s)
    } else {
        None
    };

    Some(ParsedCookie { domain, path, name, value, expires, secure, http_only })
}

/// RFC 6265 §5.1.4 — default cookie path from the request-URI path.
fn default_path(request_path: &str) -> String {
    if request_path.is_empty() || !request_path.starts_with('/') {
        return "/".to_string();
    }
    if let Some(pos) = request_path.rfind('/') {
        if pos == 0 { "/".to_string() } else { request_path[..pos].to_string() }
    } else {
        "/".to_string()
    }
}

// ── Matching helpers ──────────────────────────────────────────────────────────

/// Checks whether `request` matches `cookie_domain`.
/// A leading `.` on the cookie domain enables subdomain matching.
fn domain_matches(request: &str, cookie_domain: &str) -> bool {
    if let Some(cd) = cookie_domain.strip_prefix('.') {
        request == cd || request.ends_with(&format!(".{cd}"))
    } else {
        request == cookie_domain
    }
}

/// Checks whether `request_path` is under `cookie_path`.
fn path_matches(request_path: &str, cookie_path: &str) -> bool {
    if cookie_path == "/" {
        return true;
    }
    request_path == cookie_path
        || request_path.starts_with(&format!("{cookie_path}/"))
}

// ── Storage path ──────────────────────────────────────────────────────────────

pub fn jar_path(name: &str) -> PathBuf {
    // Treat name as a literal path if it looks like one
    if name.contains('/') || name.contains('\\') || name.ends_with(".db") {
        return PathBuf::from(name);
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".recon").join("jars").join(format!("{name}.db"))
}

// ── Time helpers ──────────────────────────────────────────────────────────────

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Converts a Unix timestamp to `YYYY-MM-DD HH:MM:SS`.
fn format_unix_time(ts: i64) -> String {
    let days = ts / 86400;
    let rem  = ts % 86400;
    let h = rem / 3600;
    let m = (rem % 3600) / 60;
    let s = rem % 60;

    // Gregorian calendar from Julian Day Number (JDN for Unix epoch = 2440588)
    let j = days + 2440588;
    let f = j + 1401 + (((4 * j + 274277) / 146097) * 3) / 4 - 38;
    let e = 4 * f + 3;
    let g = (e % 1461) / 4;
    let h2 = 5 * g + 2;
    let day   = (h2 % 153) / 5 + 1;
    let month = (h2 / 153 + 2) % 12 + 1;
    let year  = e / 1461 - 4716 + (14 - month) / 12;

    format!("{year:04}-{month:02}-{day:02} {h:02}:{m:02}:{s:02}")
}

/// Parses an HTTP-date (RFC 1123: `Thu, 01 Jan 2026 00:00:00 GMT`) to a Unix timestamp.
fn parse_http_date(s: &str) -> Option<i64> {
    let s = s.trim();
    // Skip optional weekday
    let s = s.find(',').map(|p| s[p + 1..].trim()).unwrap_or(s);
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 4 {
        return None;
    }
    let day:   i64 = parts[0].parse().ok()?;
    let month: i64 = match parts[1].to_lowercase().as_str() {
        "jan" => 1,  "feb" => 2,  "mar" => 3,  "apr" => 4,
        "may" => 5,  "jun" => 6,  "jul" => 7,  "aug" => 8,
        "sep" => 9,  "oct" => 10, "nov" => 11, "dec" => 12,
        _ => return None,
    };
    let year: i64 = parts[2].parse().ok()?;
    let t: Vec<&str> = parts[3].split(':').collect();
    if t.len() != 3 { return None; }
    let hour: i64 = t[0].parse().ok()?;
    let min:  i64 = t[1].parse().ok()?;
    let sec:  i64 = t[2].parse().ok()?;

    // Julian Day Number → Unix timestamp
    let y = if month <= 2 { year - 1 } else { year };
    let mo = if month <= 2 { month + 12 } else { month };
    let a = y / 100;
    let b = 2 - a + a / 4;
    let jd = (365.25 * (y + 4716) as f64) as i64
           + (30.6001 * (mo + 1) as f64) as i64
           + day + b - 1524;
    Some((jd - 2440588) * 86400 + hour * 3600 + min * 60 + sec)
}

fn digit_count(n: i64) -> usize {
    if n == 0 { return 1; }
    let mut n = n.unsigned_abs();
    let mut count = 0;
    while n > 0 { n /= 10; count += 1; }
    count
}
