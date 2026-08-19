//! `bit-cli man`: generate a man page.

use std::io::Write;

use bit_cli_core::ExitCode;
use bit_cli_core::error::{Result, from_io};
use clap::CommandFactory;

use crate::cli::{Cli, ManArgs};
use crate::env::Env;

/// Run the command.
pub fn run(args: &ManArgs, env: &mut Env) -> Result<ExitCode> {
    let mut page = Vec::new();
    clap_mangen::Man::new(Cli::command())
        .render(&mut page)
        .map_err(|e| from_io(e, "cannot render the man page"))?;

    // Each subcommand gets its own section, so `bit-cli.1` documents the whole
    // surface rather than only the top level.
    for sub in Cli::command().get_subcommands() {
        let mut section = Vec::new();
        clap_mangen::Man::new(sub.clone().name(format!("bit-cli-{}", sub.get_name())))
            .render(&mut section)
            .map_err(|e| from_io(e, "cannot render a subcommand man page"))?;
        page.extend_from_slice(&section);
    }

    match &args.output {
        Some(path) => {
            let path = env.resolve(path);
            std::fs::write(&path, &page)
                .map_err(|e| from_io(e, format!("cannot write {}", path.display())))?;
        }
        None => {
            env.out
                .write_all(&page)
                .map_err(|e| from_io(e, "cannot write to stdout"))?;
        }
    }
    Ok(ExitCode::Success)
}

#[cfg(test)]
mod tests {
    use crate::test_support::run_ok;

    #[test]
    fn the_man_page_renders_and_names_the_tool() {
        let out = run_ok(&["man"], ".");
        assert!(out.contains("bit-cli"), "{out}");
        assert!(out.contains(".TH"), "not roff output");
    }

    #[test]
    fn every_subcommand_gets_a_section() {
        let out = run_ok(&["man"], ".");
        for sub in [
            "bit-cli-download",
            "bit-cli-webseed",
            "bit-cli-create",
            "bit-cli-verify",
        ] {
            assert!(out.contains(sub), "`{sub}` has no man section");
        }
    }

    #[test]
    fn the_page_can_be_written_to_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bit-cli.1");
        run_ok(&["man", "-o", path.to_str().unwrap()], dir.path());
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("bit-cli"));
    }
}
