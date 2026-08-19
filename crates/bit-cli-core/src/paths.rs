//! Turning torrent paths into paths that can safely be written to disk.
//!
//! A `.torrent` is untrusted input. Its file list is arbitrary bytes chosen by
//! whoever made it, and three things go wrong if it is joined onto the output
//! directory as-is:
//!
//! 1. **Escape.** A component that the platform reads as a root or a drive
//!    takes the join outside the output directory entirely. On Windows
//!    `Path::new("D:/out").join("C:")` is `C:`, not `D:/out/C:`, so a single
//!    two-character component relocates the whole download.
//! 2. **Refusal.** `CON`, `NUL`, `COM1`, a trailing dot or space, and the
//!    characters `< > : " | ? *` cannot exist on NTFS. The write fails, and on
//!    a reserved device name it can succeed against the device instead.
//! 3. **Collision.** NTFS and APFS are case insensitive by default, so a
//!    torrent carrying both `README` and `readme` writes one file twice and
//!    the second write wins. On Linux the same torrent is fine, which is why
//!    it only shows up in production.
//!
//! [`plan`] answers all three at once, from the file list alone, with no I/O.
//! It is deterministic: the same list always yields the same on-disk paths, on
//! every platform. That matters because a resumed download has to find the
//! files the previous run wrote, and because a Windows box and a Linux box
//! seeding the same torrent should lay it out the same way.
//!
//! Sanitising happens on every platform rather than only on Windows. A payload
//! downloaded on Linux and copied to a Windows machine is a normal thing to
//! do, and a layout that only works on one of them is a layout that breaks
//! later, somewhere else, with no way to tell what happened.
//!
//! Nothing here is silent. Every change is reported in [`PathPlan::renames`]
//! with the reason, and the callers put it in `--json`, so a script can
//! reconcile the names it asked for with the names on disk.

use std::collections::HashMap;

use serde::Serialize;

/// The longest a single path component may be, in bytes.
///
/// 255 is the limit on NTFS, ext4, APFS, XFS, and every other filesystem in
/// practical use.
const MAX_COMPONENT: usize = 255;

/// Device names Windows resolves before it looks at the filesystem.
///
/// A file called `CON` cannot exist. Opening one opens the console, which is
/// the failure worth avoiding: the write appears to succeed and the bytes go
/// nowhere.
const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "COM0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9", "LPT0",
];

/// Characters NTFS refuses in a file name.
///
/// `/` and `\` are separators rather than characters and are handled by
/// splitting, not by replacement.
const ILLEGAL: &[char] = &['<', '>', ':', '"', '|', '?', '*'];

/// The character an illegal one is replaced with.
const REPLACEMENT: char = '_';

/// Why a path was changed.
///
/// The names are stable and appear in `--json`, so a caller can branch on
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reason {
    /// A component the platform reads as a drive, a root, or a UNC share, so
    /// joining it would leave the output directory.
    Escape,
    /// A component is a reserved Windows device name, with or without an
    /// extension.
    ReservedName,
    /// A component contains a character NTFS refuses.
    IllegalCharacter,
    /// A component ends in a dot or a space. Windows silently strips both,
    /// which turns two distinct names into one.
    TrailingDotOrSpace,
    /// A component is longer than any filesystem allows.
    TooLong,
    /// A component is empty, or became empty once the above were applied.
    Empty,
    /// Another file in the same torrent already claimed this path, ignoring
    /// case.
    CaseCollision,
    /// Another file in the same torrent already claimed this exact path.
    DuplicatePath,
}

impl Reason {
    /// One line naming what was wrong, for an error message.
    pub const fn description(self) -> &'static str {
        match self {
            Self::Escape => "a path component would leave the output directory",
            Self::ReservedName => "a path component is a reserved Windows device name",
            Self::IllegalCharacter => "a path component contains a character NTFS refuses",
            Self::TrailingDotOrSpace => "a path component ends in a dot or a space",
            Self::TooLong => "a path component is longer than 255 bytes",
            Self::Empty => "a path component is empty",
            Self::CaseCollision => "two paths differ only in case",
            Self::DuplicatePath => "two files claim the same path",
        }
    }
}

/// One file whose on-disk path is not the path in the torrent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Rename {
    /// Index into the torrent's file list.
    pub index: usize,
    /// The path as the metainfo gives it, `/`-separated.
    pub torrent_path: String,
    /// The path it is written to, relative to the output directory,
    /// `/`-separated.
    pub disk_path: String,
    /// Every rule that applied, in the order they are listed in [`Reason`].
    pub reasons: Vec<Reason>,
}

/// Where every file in a torrent is written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PathPlan {
    /// One relative, `/`-separated path per input, in the same order. Safe to
    /// join onto the output directory.
    pub disk_paths: Vec<String>,
    /// Only the files whose path changed. Empty for the common torrent, which
    /// is the point: a caller can test `renames.is_empty()`.
    pub renames: Vec<Rename>,
}

impl PathPlan {
    /// Whether any path had to be changed.
    pub fn is_clean(&self) -> bool {
        self.renames.is_empty()
    }

    /// Every distinct reason that applied, sorted, for a summary line.
    pub fn reasons(&self) -> Vec<Reason> {
        let mut out: Vec<Reason> = self
            .renames
            .iter()
            .flat_map(|r| r.reasons.iter().copied())
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// Plan where each of `paths` is written, relative to the output directory.
///
/// `paths` are the torrent's own paths, `/`-separated, in file-index order.
/// The result has exactly one entry per input, in the same order, and no two
/// entries collide even on a case-insensitive filesystem.
///
/// Order matters and is load-bearing: the first file to claim a name keeps it,
/// and later ones are disambiguated. Since the torrent's file order is fixed
/// by its info hash, so is the result.
pub fn plan(paths: &[String]) -> PathPlan {
    // Case-folded path -> the index that claimed it. Folding is what makes the
    // collision check match how NTFS and APFS actually compare names.
    let mut claimed: HashMap<String, usize> = HashMap::with_capacity(paths.len());
    let mut disk_paths = Vec::with_capacity(paths.len());
    let mut renames = Vec::new();

    for (index, original) in paths.iter().enumerate() {
        let mut reasons = Vec::new();
        let components: Vec<String> = original
            .split('/')
            .filter(|c| !c.is_empty() && *c != ".")
            .map(|component| sanitize_component(component, &mut reasons))
            .collect();

        // A path that sanitises away entirely still needs somewhere to go, and
        // the file index is the one name guaranteed not to collide with
        // another file's.
        let mut candidate = if components.is_empty() {
            push_reason(&mut reasons, Reason::Empty);
            format!("file-{index}")
        } else {
            components.join("/")
        };

        // Disambiguate against everything already placed. The suffix goes on
        // the stem so the extension survives, which is what keeps a renamed
        // `.iso` openable.
        let mut collision = None;
        let mut attempt = 1u32;
        while let Some(&other) = claimed.get(&fold(&candidate)) {
            collision = Some(if paths[other] == *original {
                Reason::DuplicatePath
            } else {
                Reason::CaseCollision
            });
            candidate = disambiguate(&components, index, attempt);
            attempt += 1;
        }
        if let Some(reason) = collision {
            push_reason(&mut reasons, reason);
        }

        claimed.insert(fold(&candidate), index);
        if candidate != *original {
            reasons.sort_unstable();
            reasons.dedup();
            renames.push(Rename {
                index,
                torrent_path: original.clone(),
                disk_path: candidate.clone(),
                reasons,
            });
        }
        disk_paths.push(candidate);
    }

    PathPlan {
        disk_paths,
        renames,
    }
}

/// Plan a single path on its own, discarding the report.
///
/// For the one-off cases that are not part of a file list: the directory a
/// multi-file torrent is written into, which comes from the torrent's `name`
/// and is just as untrusted as the file names under it.
pub fn plan_one(path: &str) -> String {
    plan(std::slice::from_ref(&path.to_string()))
        .disk_paths
        .pop()
        .unwrap_or_else(|| REPLACEMENT.to_string())
}

/// Make one path component safe, recording why it changed.
fn sanitize_component(component: &str, reasons: &mut Vec<Reason>) -> String {
    // A component the platform treats as anything but a plain name is the
    // dangerous case, because joining it escapes the output directory rather
    // than producing a wrong name inside it. `C:` on Windows is the one that
    // matters, and it is caught here before the `:` below turns it into
    // something harmless, so the report says "escape" rather than "illegal
    // character".
    if is_escape(component) {
        push_reason(reasons, Reason::Escape);
    }

    let mut out = String::with_capacity(component.len());
    for ch in component.chars() {
        if ILLEGAL.contains(&ch) || ch.is_control() {
            push_reason(reasons, Reason::IllegalCharacter);
            out.push(REPLACEMENT);
        } else if ch == '\\' {
            // librqbit's own validation rejects a separator inside a
            // component, so this is belt and braces: if it ever arrives, it
            // must not become a directory boundary here.
            push_reason(reasons, Reason::IllegalCharacter);
            out.push(REPLACEMENT);
        } else {
            out.push(ch);
        }
    }

    // A component made only of dots and spaces carries no name. `..` is a
    // parent reference, and Windows strips both characters, so trimming would
    // leave nothing. Replacing each character one for one keeps such
    // components distinct from each other instead of collapsing them all onto
    // the same placeholder.
    if !out.is_empty() && out.chars().all(|c| c == '.' || c == ' ') {
        push_reason(reasons, Reason::TrailingDotOrSpace);
        out = REPLACEMENT.to_string().repeat(out.chars().count());
    }

    // Windows strips trailing dots and spaces when it opens a file, so `x .`
    // and `x` are the same file there. Stripping them here makes the two
    // collide visibly and get disambiguated, rather than one silently
    // overwriting the other.
    let trimmed = out.trim_end_matches(['.', ' ']);
    if trimmed.len() != out.len() {
        push_reason(reasons, Reason::TrailingDotOrSpace);
        out.truncate(trimmed.len());
    }

    if is_reserved(&out) {
        push_reason(reasons, Reason::ReservedName);
        out = mark_stem(&out);
    }

    if out.len() > MAX_COMPONENT {
        push_reason(reasons, Reason::TooLong);
        out = truncate(&out);
    }

    if out.is_empty() {
        push_reason(reasons, Reason::Empty);
        out.push(REPLACEMENT);
    }

    out
}

/// Whether the platform would read this component as something other than a
/// plain name.
///
/// The check runs the component through the platform's own path parser rather
/// than pattern matching, so it stays correct for the forms this code does not
/// know about: `\\?\`, `\\server\share`, `C:`, and a bare root.
fn is_escape(component: &str) -> bool {
    use std::path::{Component, Path};
    let mut components = Path::new(component).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(name)), None) => name != std::ffi::OsStr::new(component),
        // An empty component is handled as Empty, not as an escape.
        (None, _) => false,
        _ => true,
    }
}

/// Whether a name resolves to a Windows device.
///
/// The stem is what matters: `CON`, `con.txt`, and `CON.tar.gz` are all the
/// console. Trailing dots and spaces are already gone by the time this runs.
fn is_reserved(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name);
    RESERVED.iter().any(|r| stem.eq_ignore_ascii_case(r))
}

/// Append the replacement character to the stem, keeping the extension.
///
/// `CON.txt` becomes `CON_.txt` rather than `CON.txt_`, so the file still
/// opens with whatever reads that extension.
fn mark_stem(name: &str) -> String {
    match name.split_once('.') {
        Some((stem, rest)) => format!("{stem}{REPLACEMENT}.{rest}"),
        None => format!("{name}{REPLACEMENT}"),
    }
}

/// Split a name into its stem and its extension, including the dot.
///
/// The last dot wins, so `archive.tar.gz` keeps `.gz`. A very long tail is not
/// an extension, it is the name, and keeping it would leave no room for the
/// stem.
fn split_extension(name: &str) -> (&str, &str) {
    match name.rsplit_once('.') {
        Some((stem, ext)) if !ext.is_empty() && ext.len() <= 16 => (stem, &name[stem.len()..]),
        _ => (name, ""),
    }
}

/// Cut a string to at most `limit` bytes, on a character boundary.
fn cut_at(text: &str, limit: usize) -> &str {
    if text.len() <= limit {
        return text;
    }
    let mut cut = limit;
    while cut > 0 && !text.is_char_boundary(cut) {
        cut -= 1;
    }
    &text[..cut]
}

/// Cut a component to the length limit without splitting a character or losing
/// the extension.
fn truncate(name: &str) -> String {
    let (stem, extension) = split_extension(name);
    let room = MAX_COMPONENT.saturating_sub(extension.len());
    format!("{}{extension}", cut_at(stem, room))
}

/// A distinct path for a file whose first choice was taken.
///
/// The suffix goes on the last component's stem, so `disc 1/readme.txt`
/// becomes `disc 1/readme-1.txt` and stays in the same directory.
fn disambiguate(components: &[String], index: usize, attempt: u32) -> String {
    let Some((last, parents)) = components.split_last() else {
        return format!("file-{index}");
    };
    let (stem, extension) = split_extension(last);
    let suffix = format!("-{attempt}");
    // The room comes out of the stem, never out of the suffix. A name already
    // at the length limit would otherwise truncate straight back onto the
    // neighbour it is trying to avoid, and the search for a free name would
    // not terminate.
    let room = MAX_COMPONENT.saturating_sub(suffix.len() + extension.len());
    let renamed = format!("{}{suffix}{extension}", cut_at(stem, room));
    let mut out: Vec<&str> = parents.iter().map(String::as_str).collect();
    out.push(&renamed);
    out.join("/")
}

/// Case-fold a path for collision detection.
///
/// `to_lowercase` is Unicode aware, which is closer to what NTFS does than
/// an ASCII fold. It will not match every case NTFS considers equal, so this
/// errs toward missing an exotic collision rather than inventing one.
fn fold(path: &str) -> String {
    path.to_lowercase()
}

fn push_reason(reasons: &mut Vec<Reason>, reason: Reason) {
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan_of(paths: &[&str]) -> PathPlan {
        plan(&paths.iter().map(|p| (*p).to_string()).collect::<Vec<_>>())
    }

    fn disk(paths: &[&str]) -> Vec<String> {
        plan_of(paths).disk_paths
    }

    #[test]
    fn an_ordinary_torrent_is_left_alone() {
        let plan = plan_of(&["disc 1/a.flac", "notes.nfo", "art/cover.jpg"]);
        assert!(plan.is_clean());
        assert_eq!(
            plan.disk_paths,
            ["disc 1/a.flac", "notes.nfo", "art/cover.jpg"]
        );
    }

    #[test]
    fn a_drive_component_cannot_escape_the_output_directory() {
        let plan = plan_of(&["C:/pwned.txt"]);
        assert_eq!(plan.disk_paths, ["C_/pwned.txt"]);
        // Both are true and both are reported: it is an escape, and the way it
        // stops being one is that the colon is not a legal character.
        assert_eq!(
            plan.renames[0].reasons,
            [Reason::Escape, Reason::IllegalCharacter]
        );
    }

    #[test]
    fn a_bare_drive_component_cannot_escape_either() {
        // The dangerous shape: one component, no separator, so nothing that
        // splits on `/` sees anything unusual.
        let plan = plan_of(&["C:"]);
        assert_eq!(plan.disk_paths, ["C_"]);
        assert!(plan.renames[0].reasons.contains(&Reason::Escape));
    }

    #[test]
    fn a_parent_reference_cannot_escape() {
        let plan = plan_of(&["../../etc/passwd"]);
        assert_eq!(plan.disk_paths, ["__/__/etc/passwd"]);
        assert!(plan.renames[0].reasons.contains(&Reason::Escape));
    }

    #[test]
    fn a_unc_component_cannot_escape() {
        let plan = plan_of(&["//server/share/file.bin"]);
        assert!(!plan.disk_paths[0].starts_with('/'));
        assert_eq!(plan.disk_paths, ["server/share/file.bin"]);
    }

    #[test]
    fn a_leading_separator_does_not_make_the_path_absolute() {
        assert_eq!(disk(&["/etc/passwd"]), ["etc/passwd"]);
    }

    #[test]
    fn reserved_device_names_are_marked_with_and_without_an_extension() {
        assert_eq!(
            disk(&["CON", "con.txt", "NUL", "lpt9.log", "COM1.bin", "aux"]),
            ["CON_", "con_.txt", "NUL_", "lpt9_.log", "COM1_.bin", "aux_"]
        );
    }

    #[test]
    fn a_name_that_merely_starts_with_a_device_name_is_left_alone() {
        let plan = plan_of(&["CONTENTS.txt", "console/aux-data.bin", "com10.bin"]);
        assert!(plan.is_clean());
    }

    #[test]
    fn illegal_characters_are_replaced() {
        let plan = plan_of(&["a<b>c:d\"e|f?g*h.bin"]);
        assert_eq!(plan.disk_paths, ["a_b_c_d_e_f_g_h.bin"]);
        assert_eq!(plan.renames[0].reasons, [Reason::IllegalCharacter]);
    }

    #[test]
    fn control_characters_are_replaced() {
        assert_eq!(disk(&["tab\there.bin"]), ["tab_here.bin"]);
    }

    #[test]
    fn trailing_dots_and_spaces_are_stripped() {
        let plan = plan_of(&["x .", "dir. /y"]);
        assert_eq!(plan.disk_paths, ["x", "dir/y"]);
        assert!(
            plan.renames
                .iter()
                .all(|r| r.reasons.contains(&Reason::TrailingDotOrSpace))
        );
    }

    #[test]
    fn a_name_that_windows_would_strip_into_another_collides_visibly() {
        // `x .` and `x` are the same file on Windows. Both must land, and
        // under distinct names.
        let plan = plan_of(&["x", "x ."]);
        assert_eq!(plan.disk_paths, ["x", "x-1"]);
        assert!(plan.renames[0].reasons.contains(&Reason::CaseCollision));
    }

    #[test]
    fn case_colliding_paths_both_land_under_distinct_names() {
        let plan = plan_of(&["README", "readme", "ReadMe"]);
        assert_eq!(plan.disk_paths, ["README", "readme-1", "ReadMe-2"]);
        for rename in &plan.renames {
            assert!(rename.reasons.contains(&Reason::CaseCollision));
        }
    }

    #[test]
    fn a_case_collision_keeps_the_extension_and_the_directory() {
        assert_eq!(
            disk(&["disc 1/Track.flac", "disc 1/track.flac"]),
            ["disc 1/Track.flac", "disc 1/track-1.flac"]
        );
    }

    #[test]
    fn an_exact_duplicate_is_reported_as_a_duplicate_not_a_case_collision() {
        let plan = plan_of(&["a.bin", "a.bin"]);
        assert_eq!(plan.disk_paths, ["a.bin", "a-1.bin"]);
        assert_eq!(plan.renames[0].reasons, [Reason::DuplicatePath]);
    }

    #[test]
    fn a_collision_created_by_sanitising_is_still_resolved() {
        // Both sanitise to `a_b.bin`, so the second has to move.
        let plan = plan_of(&["a<b.bin", "a>b.bin"]);
        assert_eq!(plan.disk_paths, ["a_b.bin", "a_b-1.bin"]);
    }

    #[test]
    fn a_long_component_is_truncated_and_keeps_its_extension() {
        let long = format!("{}.iso", "a".repeat(300));
        let plan = plan_of(&[&long]);
        let out = &plan.disk_paths[0];
        assert!(out.len() <= MAX_COMPONENT, "{} bytes", out.len());
        assert!(out.ends_with(".iso"));
        assert!(plan.renames[0].reasons.contains(&Reason::TooLong));
    }

    #[test]
    fn truncation_does_not_split_a_character() {
        // Two-byte characters, so the cut lands mid-character unless handled.
        let long = "é".repeat(200);
        let out = &disk(&[&long])[0];
        assert!(out.len() <= MAX_COMPONENT);
        assert!(std::str::from_utf8(out.as_bytes()).is_ok());
    }

    #[test]
    fn truncated_names_that_now_collide_are_still_distinct() {
        let a = format!("{}-one.iso", "x".repeat(300));
        let b = format!("{}-two.iso", "x".repeat(300));
        let plan = plan_of(&[&a, &b]);
        assert_ne!(plan.disk_paths[0], plan.disk_paths[1]);
        assert!(plan.disk_paths.iter().all(|p| p.len() <= MAX_COMPONENT));
    }

    #[test]
    fn a_path_that_sanitises_away_gets_its_file_index() {
        assert_eq!(disk(&["", "/", "."]), ["file-0", "file-1", "file-2"]);
    }

    #[test]
    fn planning_is_deterministic() {
        let paths = ["README", "readme", "CON.txt", "C:/x", "a<b.bin", "x ."];
        assert_eq!(disk(&paths), disk(&paths));
    }

    #[test]
    fn every_planned_path_is_relative_and_stays_inside() {
        use std::path::{Component, Path};
        let plan = plan_of(&[
            "C:/x",
            "//server/share/y",
            "../../z",
            "/abs",
            "CON",
            "a<b",
            "x .",
        ]);
        for path in &plan.disk_paths {
            let path = Path::new(path);
            assert!(path.is_relative(), "{path:?} is not relative");
            for component in path.components() {
                assert!(
                    matches!(component, Component::Normal(_)),
                    "{path:?} has a {component:?} component"
                );
            }
        }
    }

    #[test]
    fn no_two_planned_paths_collide_under_case_folding() {
        let plan = plan_of(&[
            "README", "readme", "ReadMe", "a.bin", "A.BIN", "a.bin", "x", "x .", "x  ",
        ]);
        let mut folded: Vec<String> = plan.disk_paths.iter().map(|p| fold(p)).collect();
        let before = folded.len();
        folded.sort();
        folded.dedup();
        assert_eq!(folded.len(), before, "planned paths collide: {folded:?}");
    }

    #[test]
    fn the_plan_has_one_entry_per_input_in_order() {
        let plan = plan_of(&["b.bin", "a.bin", "CON"]);
        assert_eq!(plan.disk_paths.len(), 3);
        assert_eq!(plan.disk_paths[0], "b.bin");
        assert_eq!(plan.disk_paths[1], "a.bin");
        assert_eq!(plan.disk_paths[2], "CON_");
    }

    #[test]
    fn renames_carry_the_index_and_both_paths() {
        let plan = plan_of(&["ok.bin", "CON.txt"]);
        assert_eq!(plan.renames.len(), 1);
        let rename = &plan.renames[0];
        assert_eq!(rename.index, 1);
        assert_eq!(rename.torrent_path, "CON.txt");
        assert_eq!(rename.disk_path, "CON_.txt");
    }

    #[test]
    fn reasons_are_deduplicated_and_summarised() {
        let plan = plan_of(&["a<b/c>d/CON.txt"]);
        assert_eq!(
            plan.renames[0].reasons,
            [Reason::ReservedName, Reason::IllegalCharacter]
        );
        assert_eq!(
            plan.reasons(),
            [Reason::ReservedName, Reason::IllegalCharacter]
        );
    }

    #[test]
    fn an_empty_file_list_plans_to_nothing() {
        let plan = plan(&[]);
        assert!(plan.is_clean());
        assert!(plan.disk_paths.is_empty());
    }
}
