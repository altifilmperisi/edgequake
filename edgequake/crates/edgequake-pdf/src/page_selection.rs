//! Parse human page-range strings into pdf2md [`PageSelection`] (SPEC-094 / SPEC-151).
//!
//! Supported forms (1-indexed):
//! - `"all"` — every page
//! - `"5"` — single page
//! - `"1-10"` — inclusive range
//! - `"1,3,5"` — explicit set
//! - `"1-3,7"` — mixed ranges and singles (SPEC-151)

use edgequake_pdf2md::PageSelection;

use crate::error::PdfConversionError;

/// Parse a pages option string into [`PageSelection`].
pub fn parse_page_selection(raw: &str) -> Result<PageSelection, PdfConversionError> {
    let s = raw.trim().to_ascii_lowercase();
    if s.is_empty() || s == "all" {
        return Ok(PageSelection::All);
    }

    // SPEC-151: mixed forms always go through the list parser when commas present
    // or when a single range/single page is requested.
    if s.contains(',') {
        let pages = parse_page_list(raw)?;
        let pages_usize: Vec<usize> = pages.into_iter().map(|p| p as usize).collect();
        return Ok(PageSelection::Set(pages_usize));
    }

    if let Some((start, end)) = s.split_once('-') {
        // Reject bare minus / non-numeric by requiring both sides parse.
        if !start.is_empty() && !end.is_empty() && !start.contains(',') {
            let start: usize = start.trim().parse().map_err(|_| {
                PdfConversionError::Backend(format!("Invalid start page in range: '{raw}'"))
            })?;
            let end: usize = end.trim().parse().map_err(|_| {
                PdfConversionError::Backend(format!("Invalid end page in range: '{raw}'"))
            })?;
            if start < 1 {
                return Err(PdfConversionError::Backend(format!(
                    "Pages are 1-indexed, minimum is 1 (got {start})"
                )));
            }
            if start > end {
                return Err(PdfConversionError::Backend(format!(
                    "Invalid page range '{start}-{end}': start must be <= end"
                )));
            }
            return Ok(PageSelection::Range(start, end));
        }
    }

    let page: usize = s
        .parse()
        .map_err(|_| PdfConversionError::Backend(format!("Invalid page number: '{raw}'")))?;
    if page < 1 {
        return Err(PdfConversionError::Backend(format!(
            "Pages are 1-indexed, minimum is 1 (got {page})"
        )));
    }
    Ok(PageSelection::Single(page))
}

/// Parse a page-list string into sorted unique 1-indexed page numbers.
///
/// Accepts `1`, `1-3`, `1,3,5`, `1-3,7` (SPEC-151). Does not accept `all`.
pub fn parse_page_list(raw: &str) -> Result<Vec<u32>, PdfConversionError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(PdfConversionError::Backend(
            "at least one page is required".into(),
        ));
    }
    if s.eq_ignore_ascii_case("all") {
        return Err(PdfConversionError::Backend(
            "page list does not accept 'all'; pass an explicit set".into(),
        ));
    }
    let mut set = std::collections::BTreeSet::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((start, end)) = part.split_once('-') {
            let start: u32 = start.trim().parse().map_err(|_| {
                PdfConversionError::Backend(format!("Invalid start page in range: '{part}'"))
            })?;
            let end: u32 = end.trim().parse().map_err(|_| {
                PdfConversionError::Backend(format!("Invalid end page in range: '{part}'"))
            })?;
            if start < 1 {
                return Err(PdfConversionError::Backend(format!(
                    "Pages are 1-indexed, minimum is 1 (got {start})"
                )));
            }
            if start > end {
                return Err(PdfConversionError::Backend(format!(
                    "Invalid page range '{start}-{end}': start must be <= end"
                )));
            }
            for p in start..=end {
                set.insert(p);
            }
        } else {
            let page: u32 = part.parse().map_err(|_| {
                PdfConversionError::Backend(format!("Invalid page number: '{part}'"))
            })?;
            if page < 1 {
                return Err(PdfConversionError::Backend(format!(
                    "Pages are 1-indexed, minimum is 1 (got {page})"
                )));
            }
            set.insert(page);
        }
    }
    if set.is_empty() {
        return Err(PdfConversionError::Backend(
            "at least one page is required".into(),
        ));
    }
    Ok(set.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_forms() {
        assert!(matches!(
            parse_page_selection("all").unwrap(),
            PageSelection::All
        ));
        assert!(matches!(
            parse_page_selection("5").unwrap(),
            PageSelection::Single(5)
        ));
        assert!(matches!(
            parse_page_selection("1-10").unwrap(),
            PageSelection::Range(1, 10)
        ));
        assert!(matches!(
            parse_page_selection("1,3,5").unwrap(),
            PageSelection::Set(ref v) if v == &[1, 3, 5]
        ));
        assert!(matches!(
            parse_page_selection("1-3,7").unwrap(),
            PageSelection::Set(ref v) if v == &[1, 2, 3, 7]
        ));
    }

    #[test]
    fn parse_page_list_mixed() {
        assert_eq!(parse_page_list("1-3,7").unwrap(), vec![1, 2, 3, 7]);
        assert_eq!(parse_page_list("5").unwrap(), vec![5]);
    }
}
