//! `bit-cli config show`: the resolved configuration and where each value came
//! from.

use std::path::PathBuf;

use bit_cli_core::ExitCode;
use bit_cli_core::config::{ConfigFile, Origin, PROJECT_CONFIG, Resolved, user_config_path};
use bit_cli_core::error::Result;
use serde::Serialize;

use crate::cli::{ConfigCommand, Global};
use crate::env::Env;
use crate::output::{Renderer, table};

/// What `bit-cli config show` reports.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    #[serde(flatten)]
    pub resolved: Resolved,
}

impl Report {
    /// The text rendering: one row per setting, with its origin.
    pub fn lines(&self) -> Vec<String> {
        let rows: Vec<Vec<String>> = self
            .resolved
            .settings
            .iter()
            .map(|(name, setting)| {
                let value = match &setting.value {
                    serde_json::Value::String(text) => text.clone(),
                    other => other.to_string(),
                };
                vec![name.clone(), value, setting.origin.label()]
            })
            .collect();
        let mut out = table(&["SETTING", "VALUE", "ORIGIN"], &rows);
        if !self.resolved.files_read.is_empty() {
            out.push(String::new());
            for path in &self.resolved.files_read {
                out.push(format!("read    {}", path.display()));
            }
        }
        for path in &self.resolved.files_missing {
            out.push(format!("absent  {}", path.display()));
        }
        out
    }
}

/// Resolve the configuration from every layer.
pub fn resolve(global: &Global, env: &Env) -> Result<Resolved> {
    let mut resolved = Resolved::defaults();
    if global.no_config {
        // `--no-config` skips the files but not the environment or the flags,
        // which are what the caller just typed.
        resolved.apply_env(&env.vars)?;
        apply_flags(&mut resolved, global);
        return Ok(resolved);
    }

    let consider = |resolved: &mut Resolved, path: PathBuf, origin: Origin| -> Result<()> {
        match ConfigFile::read_optional(&path)? {
            Some(file) => resolved.apply_file(&file, origin, &path),
            None => resolved.missed(path),
        }
        Ok(())
    };

    if let Some(path) = user_config_path() {
        consider(&mut resolved, path.clone(), Origin::UserConfig { path })?;
    }
    let project = env.cwd.join(PROJECT_CONFIG);
    consider(
        &mut resolved,
        project.clone(),
        Origin::ProjectConfig { path: project },
    )?;

    if let Some(explicit) = &global.config {
        let path = env.resolve(explicit);
        // An explicit --config that does not exist is an error, unlike the
        // files that are merely looked for.
        let file = ConfigFile::read(&path)?;
        resolved.apply_file(&file, Origin::ExplicitConfig { path: path.clone() }, &path);
    }

    resolved.apply_env(&env.vars)?;
    apply_flags(&mut resolved, global);
    Ok(resolved)
}

/// Fold the global flags into the resolved configuration.
fn apply_flags(resolved: &mut Resolved, global: &Global) {
    if let Some(dir) = &global.dir {
        resolved.apply(
            vec![("download_directory", dir.display().to_string().into())],
            Origin::Flag { name: "dir".into() },
        );
    }
    resolved.apply(
        vec![(
            "log_level",
            format!("{:?}", global.log_level).to_lowercase().into(),
        )],
        match global.verbose > 0 {
            true => Origin::Flag {
                name: "verbose".into(),
            },
            false => Origin::Default,
        },
    );
}

/// Run the command.
pub fn run(
    command: &ConfigCommand,
    global: &Global,
    renderer: &mut Renderer,
    env: &mut Env,
) -> Result<ExitCode> {
    match command {
        ConfigCommand::Show => {
            let report = Report {
                resolved: resolve(global, env)?,
            };
            renderer.emit(env, "config", &report, || report.lines())?;
            Ok(ExitCode::Success)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{run_err, run_json, run_ok};

    fn workspace() -> tempfile::TempDir {
        tempfile::tempdir().expect("temp dir")
    }

    #[test]
    fn every_setting_is_reported_with_its_origin() {
        let dir = workspace();
        let doc = run_json(&["config", "show", "--no-config"], dir.path());
        let settings = doc["settings"].as_object().unwrap();
        assert_eq!(settings.len(), bit_cli_core::config::SETTINGS.len());
        for (name, _, _) in bit_cli_core::config::SETTINGS {
            let setting = &settings[*name];
            assert!(
                setting["origin"]["kind"].is_string(),
                "{name} has no origin"
            );
        }
    }

    #[test]
    fn the_text_form_shows_the_same_settings() {
        let dir = workspace();
        let out = run_ok(&["config", "show", "--no-config"], dir.path());
        assert!(out.starts_with("SETTING"), "{out}");
        for (name, _, _) in bit_cli_core::config::SETTINGS {
            assert!(out.contains(name), "{name} missing from:\n{out}");
        }
    }

    #[test]
    fn a_project_config_overrides_the_defaults() {
        let dir = workspace();
        std::fs::write(dir.path().join(PROJECT_CONFIG), "max_peers = 42\n").unwrap();
        let doc = run_json(&["config", "show"], dir.path());
        assert_eq!(doc["settings"]["max_peers"]["value"], 42);
        assert_eq!(
            doc["settings"]["max_peers"]["origin"]["kind"],
            "project_config"
        );
    }

    #[test]
    fn no_config_ignores_the_files() {
        let dir = workspace();
        std::fs::write(dir.path().join(PROJECT_CONFIG), "max_peers = 42\n").unwrap();
        let doc = run_json(&["config", "show", "--no-config"], dir.path());
        assert_eq!(doc["settings"]["max_peers"]["origin"]["kind"], "default");
    }

    #[test]
    fn an_explicit_config_beats_the_project_one() {
        let dir = workspace();
        std::fs::write(dir.path().join(PROJECT_CONFIG), "max_peers = 42\n").unwrap();
        let explicit = dir.path().join("other.toml");
        std::fs::write(&explicit, "max_peers = 99\n").unwrap();
        let doc = run_json(
            &["config", "show", "--config", explicit.to_str().unwrap()],
            dir.path(),
        );
        assert_eq!(doc["settings"]["max_peers"]["value"], 99);
        assert_eq!(
            doc["settings"]["max_peers"]["origin"]["kind"],
            "explicit_config"
        );
    }

    #[test]
    fn a_flag_beats_every_file() {
        let dir = workspace();
        std::fs::write(
            dir.path().join(PROJECT_CONFIG),
            "download_directory = \"/from-file\"\n",
        )
        .unwrap();
        let doc = run_json(&["config", "show", "-d", "/from-flag"], dir.path());
        assert_eq!(doc["settings"]["download_directory"]["value"], "/from-flag");
        assert_eq!(
            doc["settings"]["download_directory"]["origin"]["kind"],
            "flag"
        );
    }

    #[test]
    fn a_missing_explicit_config_is_an_error() {
        let dir = workspace();
        run_err(
            &["config", "show", "--config", "nope.toml"],
            dir.path(),
            ExitCode::Disk,
        );
    }

    #[test]
    fn an_invalid_config_file_is_a_config_error() {
        let dir = workspace();
        std::fs::write(dir.path().join(PROJECT_CONFIG), "max_peerz = 1\n").unwrap();
        let err = run_err(&["config", "show"], dir.path(), ExitCode::Config);
        assert!(err.contains("max_peerz"), "{err}");
    }

    #[test]
    fn files_that_were_looked_for_are_reported() {
        let dir = workspace();
        let doc = run_json(&["config", "show"], dir.path());
        let missing = doc["files_missing"].as_array().unwrap();
        assert!(
            missing
                .iter()
                .any(|p| p.as_str().unwrap().ends_with(PROJECT_CONFIG)),
            "the project config should be listed as absent: {missing:?}"
        );
    }
}
