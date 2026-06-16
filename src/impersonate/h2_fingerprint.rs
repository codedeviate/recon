//! Parser for the Akamai HTTP/2 fingerprint string used by
//! `--http2-fingerprint`.
//!
//! Format (four `|`-separated fields):
//! `SETTINGS|WINDOW_UPDATE|PRIORITY|PSEUDO_HEADER_ORDER`
//!
//! e.g. `1:65536,3:1000,4:6291456,6:262144|15663105|0|m,a,s,p`
//!
//! - **SETTINGS** — comma-joined `id:value` pairs, in wire order. The id
//!   maps to an HTTP/2 SETTINGS parameter (1 HEADER_TABLE_SIZE, 2
//!   ENABLE_PUSH, 3 MAX_CONCURRENT_STREAMS, 4 INITIAL_WINDOW_SIZE, 5
//!   MAX_FRAME_SIZE, 6 MAX_HEADER_LIST_SIZE, 8/9 vendor settings).
//! - **WINDOW_UPDATE** — a single integer connection-level window
//!   increment; `0` / `00` means no WINDOW_UPDATE frame.
//! - **PRIORITY** — `0` for none, else comma-joined
//!   `streamId:exclusive:dependsOn:weight` tuples.
//! - **PSEUDO_HEADER_ORDER** — exactly four of `m`/`a`/`s`/`p`
//!   (method/authority/scheme/path), each once.
//!
//! The parser is pure (string → struct) and validates every field with a
//! field-specific error; the wire→builder mapping lives in the parent
//! module. JA3/JA4 overrides stay deferred (lossy / non-invertible); only
//! the fully-introspectable H2 layer is reconstructed here.

use anyhow::{anyhow, bail, Result};
use wreq::{PseudoOrder, SettingsOrder};

/// SETTINGS ids recon understands, in HTTP/2 spec order. Used to validate
/// ids and to fill the trailing (inert) slots of `settings_order`.
const KNOWN_SETTING_IDS: [u8; 8] = [1, 2, 3, 4, 5, 6, 8, 9];

/// One parsed PRIORITY frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriorityEntry {
    pub stream_id: u32,
    pub exclusive: bool,
    pub depends_on: u32,
    pub weight: u8,
}

/// A parsed Akamai HTTP/2 fingerprint. Settings are kept as ordered
/// `(id, value)` pairs so the parser stays free of wreq builder types;
/// the parent module maps them onto `Http2Builder`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct H2Fingerprint {
    /// `(id, value)` SETTINGS pairs in wire order.
    pub settings: Vec<(u8, u32)>,
    /// Connection-level WINDOW_UPDATE increment; `None` if not sent.
    pub window_update: Option<u32>,
    /// PRIORITY frames; empty if the field was `0`.
    pub priorities: Vec<PriorityEntry>,
    /// Pseudo-header transmission order.
    pub pseudo_order: [PseudoOrder; 4],
}

impl H2Fingerprint {
    /// The `[SettingsOrder; 8]` describing the observable SETTINGS order:
    /// the present ids first (in wire order), then the remaining known
    /// settings in spec order to fill the array. The trailing entries are
    /// inert because those settings aren't actually emitted.
    pub fn settings_order(&self) -> [SettingsOrder; 8] {
        let present: Vec<u8> = self.settings.iter().map(|(id, _)| *id).collect();
        let mut ordered: Vec<u8> = present.clone();
        for id in KNOWN_SETTING_IDS {
            if !present.contains(&id) {
                ordered.push(id);
            }
        }
        let mut out = [SettingsOrder::HeaderTableSize; 8];
        for (slot, id) in out.iter_mut().zip(ordered) {
            *slot = setting_order_for_id(id);
        }
        out
    }
}

/// Map a SETTINGS id to its `SettingsOrder` variant. `id` is assumed valid
/// (validated during parse).
fn setting_order_for_id(id: u8) -> SettingsOrder {
    match id {
        1 => SettingsOrder::HeaderTableSize,
        2 => SettingsOrder::EnablePush,
        3 => SettingsOrder::MaxConcurrentStreams,
        4 => SettingsOrder::InitialWindowSize,
        5 => SettingsOrder::MaxFrameSize,
        6 => SettingsOrder::MaxHeaderListSize,
        8 => SettingsOrder::UnknownSetting8,
        9 => SettingsOrder::UnknownSetting9,
        // Unreachable: parse() rejects unknown ids before this is called.
        _ => SettingsOrder::HeaderTableSize,
    }
}

/// Parse an Akamai HTTP/2 fingerprint string.
pub fn parse(s: &str) -> Result<H2Fingerprint> {
    let fields: Vec<&str> = s.split('|').collect();
    if fields.len() != 4 {
        bail!(
            "--http2-fingerprint: expected 4 '|'-separated fields \
             (SETTINGS|WINDOW_UPDATE|PRIORITY|PSEUDO_HEADER_ORDER), got {}",
            fields.len()
        );
    }
    let settings = parse_settings(fields[0])?;
    let window_update = parse_window_update(fields[1])?;
    let priorities = parse_priorities(fields[2])?;
    let pseudo_order = parse_pseudo_order(fields[3])?;
    Ok(H2Fingerprint { settings, window_update, priorities, pseudo_order })
}

fn parse_settings(field: &str) -> Result<Vec<(u8, u32)>> {
    let mut out = Vec::new();
    if field.is_empty() {
        return Ok(out);
    }
    for pair in field.split(',') {
        let (id_s, val_s) = pair.split_once(':').ok_or_else(|| {
            anyhow!("--http2-fingerprint: SETTINGS pair '{pair}' is not 'id:value'")
        })?;
        let id: u8 = id_s.parse().map_err(|_| {
            anyhow!("--http2-fingerprint: SETTINGS id '{id_s}' is not a number")
        })?;
        if !KNOWN_SETTING_IDS.contains(&id) {
            bail!("--http2-fingerprint: unknown SETTINGS id {id} (expected 1-6, 8 or 9)");
        }
        let val: u32 = val_s.parse().map_err(|_| {
            anyhow!("--http2-fingerprint: SETTINGS value '{val_s}' is not a number")
        })?;
        if out.iter().any(|(existing, _)| *existing == id) {
            bail!("--http2-fingerprint: duplicate SETTINGS id {id}");
        }
        out.push((id, val));
    }
    Ok(out)
}

fn parse_window_update(field: &str) -> Result<Option<u32>> {
    // "0" or "00" → no WINDOW_UPDATE frame.
    if field == "0" || field == "00" {
        return Ok(None);
    }
    let n: u32 = field.parse().map_err(|_| {
        anyhow!("--http2-fingerprint: WINDOW_UPDATE '{field}' is not a number")
    })?;
    Ok(Some(n))
}

fn parse_priorities(field: &str) -> Result<Vec<PriorityEntry>> {
    if field == "0" {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for tuple in field.split(',') {
        let parts: Vec<&str> = tuple.split(':').collect();
        if parts.len() != 4 {
            bail!(
                "--http2-fingerprint: PRIORITY '{tuple}' must be \
                 streamId:exclusive:dependsOn:weight"
            );
        }
        let stream_id: u32 = parts[0]
            .parse()
            .map_err(|_| anyhow!("--http2-fingerprint: PRIORITY streamId '{}' invalid", parts[0]))?;
        let exclusive = match parts[1] {
            "0" => false,
            "1" => true,
            other => bail!("--http2-fingerprint: PRIORITY exclusive '{other}' must be 0 or 1"),
        };
        let depends_on: u32 = parts[2]
            .parse()
            .map_err(|_| anyhow!("--http2-fingerprint: PRIORITY dependsOn '{}' invalid", parts[2]))?;
        let weight: u8 = parts[3]
            .parse()
            .map_err(|_| anyhow!("--http2-fingerprint: PRIORITY weight '{}' invalid (0-255)", parts[3]))?;
        out.push(PriorityEntry { stream_id, exclusive, depends_on, weight });
    }
    Ok(out)
}

fn parse_pseudo_order(field: &str) -> Result<[PseudoOrder; 4]> {
    let items: Vec<&str> = field.split(',').collect();
    if items.len() != 4 {
        bail!(
            "--http2-fingerprint: PSEUDO_HEADER_ORDER needs exactly 4 entries \
             (m,a,s,p in some order), got {}",
            items.len()
        );
    }
    let mut out = [PseudoOrder::Method; 4];
    let mut seen = [false; 4]; // m, a, s, p
    for (slot, item) in out.iter_mut().zip(items) {
        let (po, idx) = match item.trim() {
            "m" => (PseudoOrder::Method, 0),
            "a" => (PseudoOrder::Authority, 1),
            "s" => (PseudoOrder::Scheme, 2),
            "p" => (PseudoOrder::Path, 3),
            other => bail!(
                "--http2-fingerprint: PSEUDO_HEADER_ORDER entry '{other}' must be one of m/a/s/p"
            ),
        };
        if seen[idx] {
            bail!("--http2-fingerprint: PSEUDO_HEADER_ORDER entry '{item}' repeated");
        }
        seen[idx] = true;
        *slot = po;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC_EXAMPLE: &str = "1:65536,3:1000,4:6291456,6:262144|15663105|0|m,a,s,p";

    #[test]
    fn parses_doc_example() {
        let fp = parse(DOC_EXAMPLE).unwrap();
        assert_eq!(fp.settings, vec![(1, 65536), (3, 1000), (4, 6291456), (6, 262144)]);
        assert_eq!(fp.window_update, Some(15663105));
        assert!(fp.priorities.is_empty());
        assert_eq!(
            fp.pseudo_order,
            [PseudoOrder::Method, PseudoOrder::Authority, PseudoOrder::Scheme, PseudoOrder::Path]
        );
    }

    #[test]
    fn settings_order_present_first_then_fill() {
        let fp = parse(DOC_EXAMPLE).unwrap();
        let order = fp.settings_order();
        // present ids 1,3,4,6 → their variants first, in wire order
        assert_eq!(order[0], SettingsOrder::HeaderTableSize); // 1
        assert_eq!(order[1], SettingsOrder::MaxConcurrentStreams); // 3
        assert_eq!(order[2], SettingsOrder::InitialWindowSize); // 4
        assert_eq!(order[3], SettingsOrder::MaxHeaderListSize); // 6
        // remaining (2,5,8,9) appended in spec order
        assert_eq!(order[4], SettingsOrder::EnablePush); // 2
        assert_eq!(order[5], SettingsOrder::MaxFrameSize); // 5
        assert_eq!(order[6], SettingsOrder::UnknownSetting8); // 8
        assert_eq!(order[7], SettingsOrder::UnknownSetting9); // 9
    }

    #[test]
    fn window_update_zero_is_none() {
        assert_eq!(parse("1:1|0|0|m,a,s,p").unwrap().window_update, None);
        assert_eq!(parse("1:1|00|0|m,a,s,p").unwrap().window_update, None);
    }

    #[test]
    fn parses_priority_frames() {
        let fp = parse("1:65536|0|3:1:0:201,5:0:0:101|m,a,s,p").unwrap();
        assert_eq!(
            fp.priorities,
            vec![
                PriorityEntry { stream_id: 3, exclusive: true, depends_on: 0, weight: 201 },
                PriorityEntry { stream_id: 5, exclusive: false, depends_on: 0, weight: 101 },
            ]
        );
    }

    #[test]
    fn empty_settings_ok() {
        assert!(parse("|0|0|m,a,s,p").unwrap().settings.is_empty());
    }

    #[test]
    fn reordered_pseudo() {
        let fp = parse("1:1|0|0|m,p,s,a").unwrap();
        assert_eq!(
            fp.pseudo_order,
            [PseudoOrder::Method, PseudoOrder::Path, PseudoOrder::Scheme, PseudoOrder::Authority]
        );
    }

    #[test]
    fn rejects_wrong_field_count() {
        assert!(parse("1:1|0|0").is_err());
        assert!(parse("1:1|0|0|m,a,s,p|extra").is_err());
    }

    #[test]
    fn rejects_non_numeric_setting_value() {
        let e = parse("1:abc|0|0|m,a,s,p").unwrap_err().to_string();
        assert!(e.contains("SETTINGS value"), "got: {e}");
    }

    #[test]
    fn rejects_unknown_setting_id() {
        let e = parse("7:1|0|0|m,a,s,p").unwrap_err().to_string();
        assert!(e.contains("unknown SETTINGS id 7"), "got: {e}");
    }

    #[test]
    fn rejects_duplicate_setting() {
        assert!(parse("1:1,1:2|0|0|m,a,s,p").is_err());
    }

    #[test]
    fn rejects_bad_pseudo_letter() {
        assert!(parse("1:1|0|0|m,a,s,x").is_err());
    }

    #[test]
    fn rejects_short_pseudo() {
        assert!(parse("1:1|0|0|m,a,s").is_err());
    }

    #[test]
    fn rejects_duplicate_pseudo() {
        assert!(parse("1:1|0|0|m,m,s,p").is_err());
    }

    #[test]
    fn rejects_malformed_priority() {
        assert!(parse("1:1|0|3:1:0|m,a,s,p").is_err()); // only 3 parts
        assert!(parse("1:1|0|3:2:0:1|m,a,s,p").is_err()); // exclusive not 0/1
    }
}
