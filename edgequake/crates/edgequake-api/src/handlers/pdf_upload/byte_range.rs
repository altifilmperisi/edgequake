//! HTTP `Range` header parsing for binary downloads (RFC 9110 §14).
//!
//! Lets the browser's PDF engine fetch only the byte ranges it needs
//! (first page, then pages on demand) instead of the whole file.
//!
//! Only a single `bytes=` range is honoured. Anything else (multi-range,
//! non-`bytes` unit, malformed syntax) falls back to a full `200` response,
//! which RFC 9110 explicitly permits.

/// Outcome of evaluating a `Range` header against a representation length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteRange {
    /// No usable range: serve the whole body with `200`.
    Full,
    /// Inclusive byte window `start..=end`: serve with `206`.
    Partial { start: u64, end: u64 },
    /// Range starts beyond the end of the body: reply `416`.
    Unsatisfiable,
}

/// Evaluate an optional `Range` header value against a body of `len` bytes.
pub fn evaluate_range(header: Option<&str>, len: u64) -> ByteRange {
    let Some(raw) = header else {
        return ByteRange::Full;
    };
    if len == 0 {
        return ByteRange::Full;
    }
    let Some(spec) = raw.trim().strip_prefix("bytes=") else {
        return ByteRange::Full;
    };
    // Multi-range requests are optional to support.
    if spec.contains(',') {
        return ByteRange::Full;
    }
    let Some((first, last)) = spec.split_once('-') else {
        return ByteRange::Full;
    };
    let (first, last) = (first.trim(), last.trim());

    match (first.is_empty(), last.is_empty()) {
        // "-N": the final N bytes.
        (true, false) => match last.parse::<u64>() {
            Ok(0) => ByteRange::Unsatisfiable,
            Ok(n) => ByteRange::Partial {
                start: len.saturating_sub(n),
                end: len - 1,
            },
            Err(_) => ByteRange::Full,
        },
        // "A-" or "A-B".
        (false, _) => {
            let Ok(start) = first.parse::<u64>() else {
                return ByteRange::Full;
            };
            let end = if last.is_empty() {
                len - 1
            } else {
                match last.parse::<u64>() {
                    Ok(e) => e.min(len - 1),
                    Err(_) => return ByteRange::Full,
                }
            };
            if start >= len {
                ByteRange::Unsatisfiable
            } else if end < start {
                // Syntactically invalid (last < first): ignore the header.
                ByteRange::Full
            } else {
                ByteRange::Partial { start, end }
            }
        }
        (true, true) => ByteRange::Full,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_header_is_full() {
        assert_eq!(evaluate_range(None, 100), ByteRange::Full);
    }

    #[test]
    fn closed_range() {
        assert_eq!(
            evaluate_range(Some("bytes=0-9"), 100),
            ByteRange::Partial { start: 0, end: 9 }
        );
    }

    #[test]
    fn open_ended_range_runs_to_eof() {
        assert_eq!(
            evaluate_range(Some("bytes=90-"), 100),
            ByteRange::Partial { start: 90, end: 99 }
        );
    }

    #[test]
    fn end_is_clamped_to_length() {
        assert_eq!(
            evaluate_range(Some("bytes=50-5000"), 100),
            ByteRange::Partial { start: 50, end: 99 }
        );
    }

    #[test]
    fn suffix_range_returns_tail() {
        assert_eq!(
            evaluate_range(Some("bytes=-10"), 100),
            ByteRange::Partial { start: 90, end: 99 }
        );
        // Suffix longer than the body returns the whole body as 206.
        assert_eq!(
            evaluate_range(Some("bytes=-500"), 100),
            ByteRange::Partial { start: 0, end: 99 }
        );
    }

    #[test]
    fn start_past_eof_is_unsatisfiable() {
        assert_eq!(
            evaluate_range(Some("bytes=100-"), 100),
            ByteRange::Unsatisfiable
        );
        assert_eq!(
            evaluate_range(Some("bytes=-0"), 100),
            ByteRange::Unsatisfiable
        );
    }

    #[test]
    fn unusable_headers_fall_back_to_full() {
        for h in [
            "items=0-9",
            "bytes=0-9,20-29",
            "bytes=abc-",
            "bytes=9-0",
            "bytes=-",
            "bytes",
            "",
        ] {
            assert_eq!(evaluate_range(Some(h), 100), ByteRange::Full, "{h:?}");
        }
    }

    #[test]
    fn empty_body_is_always_full() {
        assert_eq!(evaluate_range(Some("bytes=0-9"), 0), ByteRange::Full);
    }
}
