//! Turning `--select-file` and `--exclude-file` into explicit file indices.
//!
//! Shared because two commands need the same answer from the same two flags:
//! `download` decides what to fetch and `verify` decides what a piece outside
//! the selection means. A second copy of this would be a second set of
//! off-by-one bugs.
//!
//! The one thing that differs between callers is whether the file count is
//! known yet. `verify` reads a `.torrent` off the disk and knows it before it
//! parses a flag; `download` may be handed a magnet, where the file list does
//! not exist until the metadata resolves over the network. Two forms need the
//! count and nothing else does: an exclusion with no selection beside it, and
//! an open-ended range. Both are refused rather than guessed at when the count
//! is absent. See `TODO/cli-surface.md`, T-185.

use std::collections::HashSet;

use bit_cli_core::error::{Error, Result};

/// Resolve the two flags into the file indices to work on.
///
/// `None` means every file, which is not the same as an empty list: an empty
/// list would select nothing at all, and that is a usage error.
///
/// `file_count` is the number of files in the torrent when it is known.
pub fn resolve(
    select: &[String],
    exclude: &[String],
    file_count: Option<usize>,
) -> Result<Option<Vec<usize>>> {
    if select.is_empty() && exclude.is_empty() {
        return Ok(None);
    }
    let selected = parse(select, "select-file", file_count)?;
    let excluded: HashSet<usize> = parse(exclude, "exclude-file", file_count)?
        .into_iter()
        .collect();

    // With no selection **flag**, the selection is everything the exclusion
    // leaves. That needs the file count, and a caller who cannot supply one
    // gets the old answer: the exclusion is not applied here.
    // `TODO/cli-surface.md` T-185 is the entry for the case where it is never
    // applied anywhere.
    //
    // Keyed on whether the flag was given rather than on what it resolved to.
    // `--select-file 9-` on a five-file torrent resolves to nothing, and that
    // is a caller asking for files that are not there, not a caller asking for
    // all of them.
    let selected = match (select.is_empty(), file_count) {
        (true, Some(count)) => (0..count).collect(),
        (true, None) => return Ok(None),
        (false, _) => selected,
    };

    let mut out: Vec<usize> = selected
        .into_iter()
        .filter(|index| !excluded.contains(index))
        .collect();
    out.sort_unstable();
    out.dedup();
    if out.is_empty() {
        return Err(Error::usage(
            "--select-file and --exclude-file together select no files at all",
        ));
    }
    Ok(Some(out))
}

/// Parse one flag's worth of indices and ranges.
fn parse(values: &[String], flag: &str, file_count: Option<usize>) -> Result<Vec<usize>> {
    let mut out = Vec::new();
    for value in values {
        for term in value.split(',') {
            let term = term.trim();
            if term.is_empty() {
                continue;
            }
            match term.split_once('-') {
                None => out.push(term.parse::<usize>().map_err(|_| index_error(flag, term))?),
                Some((start, "")) => {
                    let start: usize = start.trim().parse().map_err(|_| index_error(flag, term))?;
                    // An open-ended range needs an upper bound. Refuse rather
                    // than guessing at one when the file count is not known.
                    let Some(count) = file_count else {
                        return Err(Error::usage(format!(
                            "--{flag} `{term}`: an open-ended range needs the file count; list the indices or use a closed range"
                        )));
                    };
                    out.extend(start..count);
                }
                Some((start, end)) => {
                    let start: usize = start.trim().parse().map_err(|_| index_error(flag, term))?;
                    let end: usize = end.trim().parse().map_err(|_| index_error(flag, term))?;
                    if start > end {
                        return Err(Error::usage(format!("--{flag} `{term}` runs backwards")));
                    }
                    out.extend(start..=end);
                }
            }
        }
    }
    Ok(out)
}

fn index_error(flag: &str, term: &str) -> Error {
    Error::usage(format!(
        "--{flag} `{term}` is not a file index or an index range"
    ))
    .with("value", term.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit_cli_core::ExitCode;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn no_flags_means_every_file() {
        assert_eq!(resolve(&[], &[], None).unwrap(), None);
        assert_eq!(resolve(&[], &[], Some(5)).unwrap(), None);
    }

    #[test]
    fn indices_and_ranges_both_select() {
        assert_eq!(resolve(&args(&["0"]), &[], None).unwrap(), Some(vec![0]));
        assert_eq!(
            resolve(&args(&["1-3"]), &[], None).unwrap(),
            Some(vec![1, 2, 3])
        );
        assert_eq!(
            resolve(&args(&["1-3", "7"]), &[], None).unwrap(),
            Some(vec![1, 2, 3, 7])
        );
    }

    #[test]
    fn an_exclusion_narrows_a_selection() {
        assert_eq!(
            resolve(&args(&["0-4"]), &args(&["2"]), None).unwrap(),
            Some(vec![0, 1, 3, 4])
        );
    }

    /// With the count, an exclusion on its own is the complement.
    #[test]
    fn an_exclusion_alone_is_every_other_file_when_the_count_is_known() {
        assert_eq!(
            resolve(&[], &args(&["1"]), Some(4)).unwrap(),
            Some(vec![0, 2, 3])
        );
        assert_eq!(
            resolve(&[], &args(&["0", "3"]), Some(4)).unwrap(),
            Some(vec![1, 2])
        );
    }

    /// Without it, it is not applied here, which is the state
    /// `TODO/cli-surface.md` T-185 records.
    #[test]
    fn an_exclusion_alone_needs_the_file_count() {
        assert_eq!(resolve(&[], &args(&["1"]), None).unwrap(), None);
    }

    #[test]
    fn excluding_every_file_is_a_usage_error() {
        let err = resolve(&[], &args(&["0-3"]), Some(4)).unwrap_err();
        assert_eq!(err.code(), ExitCode::Usage);
    }

    #[test]
    fn an_open_ended_range_resolves_against_the_count() {
        assert_eq!(
            resolve(&args(&["2-"]), &[], Some(5)).unwrap(),
            Some(vec![2, 3, 4])
        );
    }

    #[test]
    fn an_open_ended_range_with_no_count_says_why_it_cannot_be_resolved() {
        let err = resolve(&args(&["2-"]), &[], None).unwrap_err();
        assert_eq!(err.code(), ExitCode::Usage);
        assert!(err.message().contains("file count"), "{}", err.message());
    }

    /// An open-ended range starting past the end selects nothing, which is a
    /// usage error rather than a silent empty download.
    #[test]
    fn an_open_ended_range_past_the_end_selects_nothing() {
        let err = resolve(&args(&["9-"]), &[], Some(5)).unwrap_err();
        assert_eq!(err.code(), ExitCode::Usage);
    }

    #[test]
    fn selecting_nothing_at_all_is_a_usage_error_rather_than_an_empty_selection() {
        let err = resolve(&args(&["1-2"]), &args(&["1-2"]), None).unwrap_err();
        assert_eq!(err.code(), ExitCode::Usage);
    }

    #[test]
    fn a_bad_index_names_the_flag_and_the_value() {
        let err = resolve(&args(&["two"]), &[], None).unwrap_err();
        assert_eq!(err.code(), ExitCode::Usage);
        assert!(err.message().contains("select-file"), "{}", err.message());
        assert_eq!(err.context()["value"], "two");

        let err = resolve(&[], &args(&["two"]), Some(4)).unwrap_err();
        assert!(err.message().contains("exclude-file"), "{}", err.message());
    }

    #[test]
    fn a_backwards_range_is_refused() {
        let err = resolve(&args(&["5-2"]), &[], None).unwrap_err();
        assert!(err.message().contains("backwards"), "{}", err.message());
    }
}
