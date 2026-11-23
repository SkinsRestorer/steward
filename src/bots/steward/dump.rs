#![warn(clippy::indexing_slicing, clippy::arithmetic_side_effects)]

//! Analysis of the JSON dump that the `/sr dump` command uploads.

use std::{borrow::Cow, sync::LazyLock};

use poise::serenity_prelude as serenity;
use regex::Regex;
use semver::Version;
use serde::{Deserialize, de::IgnoredAny};

use crate::releases::LatestRelease;

const MAX_DUMP_FIELD_CHARS: usize = 100;

/// Returns embeds that summarize a plugin dump, or `None` when the text
/// is not a dump.
pub fn analyze(
    text: &str,
    latest_release: Option<&LatestRelease>,
) -> Option<Vec<serenity::CreateEmbed>> {
    let dump = serde_json::from_str::<SkinsRestorerDump<'_>>(text).ok()?;
    Some(build_dump_embeds(&dump, latest_release))
}

#[derive(Deserialize)]
struct SkinsRestorerDump<'a> {
    #[serde(borrow, rename = "buildInfo")]
    build: BuildInfo<'a>,
    #[serde(borrow, rename = "environmentInfo")]
    environment: EnvironmentInfo<'a>,
    #[serde(borrow, rename = "javaInfo")]
    java: JavaInfo<'a>,
    #[serde(borrow, rename = "osInfo")]
    os: OsInfo<'a>,
    #[serde(borrow, rename = "platformInfo")]
    platform: PlatformInfo<'a>,
    #[serde(rename = "pluginInfo")]
    plugin: PluginInfo,
    #[serde(borrow, rename = "userInfo")]
    user: UserInfo<'a>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildInfo<'a> {
    #[serde(borrow)]
    build_time: Cow<'a, str>,
    #[serde(borrow)]
    version: Cow<'a, str>,
}

#[derive(Deserialize)]
struct EnvironmentInfo<'a> {
    hybrid: bool,
    #[serde(borrow)]
    platform: Cow<'a, str>,
    #[serde(borrow, rename = "platformType")]
    platform_type: Cow<'a, str>,
}

#[derive(Deserialize)]
struct JavaInfo<'a> {
    #[serde(borrow)]
    version: Cow<'a, str>,
}

#[derive(Deserialize)]
struct OsInfo<'a> {
    #[serde(borrow)]
    arch: Cow<'a, str>,
    #[serde(borrow)]
    name: Cow<'a, str>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlatformInfo<'a> {
    #[serde(borrow)]
    platform_name: Cow<'a, str>,
    #[serde(borrow)]
    platform_version: Cow<'a, str>,
    plugins: Vec<IgnoredAny>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginInfo {
    config_data: ConfigData,
    proxy_mode: bool,
}

#[derive(Deserialize)]
struct ConfigData {
    dev: DevConfig,
}

#[derive(Deserialize)]
struct DevConfig {
    debug: bool,
}

#[derive(Deserialize)]
struct UserInfo<'a> {
    #[serde(borrow)]
    dir: Cow<'a, str>,
    #[serde(borrow)]
    home: Cow<'a, str>,
    #[serde(borrow)]
    name: Cow<'a, str>,
}

fn build_dump_embeds(
    dump: &SkinsRestorerDump<'_>,
    latest_release: Option<&LatestRelease>,
) -> Vec<serenity::CreateEmbed> {
    let version = coerce_version(dump.build.version.as_ref());
    let latest_version = latest_release.and_then(|release| coerce_version(&release.tag_name));
    let version_display = version
        .as_ref()
        .map_or_else(|| dump.build.version.to_string(), ToString::to_string);
    let mut embeds = Vec::new();

    if let (Some(version), Some(latest_version)) = (&version, &latest_version)
        && version < latest_version
    {
        embeds.push(
            serenity::CreateEmbed::new()
                .title("Update SkinsRestorer")
                .colour(0xED_42_45)
                .description(format!(
                    "SkinsRestorer `{version}` is outdated. Update to `{latest_version}`."
                ))
                .field(
                    " ",
                    format!(
                        "[Download {latest_version}](https://modrinth.com/plugin/skinsrestorer/version/{latest_version})"
                    ),
                    false,
                ),
        );
    }

    embeds.push(
        serenity::CreateEmbed::new()
            .title("Build")
            .colour(0x58_65_F2)
            .description(format!(
                "SkinsRestorer version: `{}`\nBuild date: `{}`",
                dump_field(&version_display),
                dump_field(&dump.build.build_time)
            )),
    );

    if dump.user.name == "?"
        || dump.user.dir == "/home/container"
        || dump.user.home == "/home/container"
    {
        embeds.push(
            serenity::CreateEmbed::new()
                .title("Docker environment")
                .colour(0x58_65_F2)
                .description(
                    "The dump shows a Docker container, possibly managed by Pterodactyl or Pelican. Include this detail in support requests.",
                ),
        );
    }

    embeds.push(
        serenity::CreateEmbed::new()
            .title("Operating system and Java")
            .colour(0x58_65_F2)
            .description(format!(
                "Operating system: `{}`\nArchitecture: `{}`\nJava: `{}`",
                dump_field(&dump.os.name),
                dump_field(&dump.os.arch),
                dump_field(&dump.java.version)
            )),
    );
    embeds.push(
        serenity::CreateEmbed::new()
            .title("Server platform")
            .colour(0x58_65_F2)
            .description(format!(
                "Platform: `{}`\nEnvironment: `{}` / `{}`\nVersion: `{}`\nPlugins: `{}`",
                dump_field(&dump.platform.platform_name),
                dump_field(&dump.environment.platform),
                dump_field(&dump.environment.platform_type),
                dump_field(&dump.platform.platform_version),
                dump.platform.plugins.len()
            )),
    );
    if dump.environment.hybrid {
        embeds.push(
            serenity::CreateEmbed::new()
                .title("Unsupported hybrid platform")
                .colour(0xED_42_45)
                .description(
                    "This platform mixes mods and plugins. SkinsRestorer does not support hybrid platforms because they can cause compatibility problems.",
                ),
        );
    }
    embeds.push(
        serenity::CreateEmbed::new()
            .title("Plugin configuration")
            .colour(0x58_65_F2)
            .description(format!(
                "Proxy mode: `{}`\nDebug mode: `{}`",
                dump.plugin.proxy_mode, dump.plugin.config_data.dev.debug
            )),
    );
    embeds
}

/// Shortens a user-controlled dump value and removes backticks so it fits in an
/// inline code span and the embed stays within Discord's limits.
fn dump_field(value: &str) -> String {
    let value = value.replace('`', "'");
    if value.chars().count() <= MAX_DUMP_FIELD_CHARS {
        return value;
    }
    let mut shortened = value
        .chars()
        .take(MAX_DUMP_FIELD_CHARS.saturating_sub(3))
        .collect::<String>();
    shortened.push_str("...");
    shortened
}

fn coerce_version(input: &str) -> Option<Version> {
    static VERSION_PATTERN: LazyLock<Result<Regex, regex::Error>> =
        LazyLock::new(|| Regex::new(r"(?P<major>\d+)\.(?P<minor>\d+)(?:\.(?P<patch>\d+))?"));
    let captures = VERSION_PATTERN.as_ref().ok()?.captures(input)?;
    let major = captures.name("major")?.as_str();
    let minor = captures.name("minor")?.as_str();
    let patch = captures
        .name("patch")
        .map_or("0", |capture| capture.as_str());
    Version::parse(&format!("{major}.{minor}.{patch}")).ok()
}

#[cfg(test)]
mod tests {
    use anyhow::{Context as _, Result, ensure};
    use semver::Version;

    use super::{MAX_DUMP_FIELD_CHARS, SkinsRestorerDump, analyze, coerce_version, dump_field};
    use crate::releases::LatestRelease;

    fn dump(version: &str, user_dir: &str, hybrid: bool) -> String {
        serde_json::json!({
            "buildInfo": {"buildTime": "today", "version": version},
            "environmentInfo": {"hybrid": hybrid, "platform": "Paper", "platformType": "SERVER"},
            "javaInfo": {"version": "21"},
            "osInfo": {"arch": "x86_64", "name": "Linux"},
            "platformInfo": {"platformName": "Paper", "platformVersion": "1.21", "plugins": [{"name": "A"}]},
            "pluginInfo": {"configData": {"dev": {"debug": false}}, "proxyMode": false},
            "userInfo": {"dir": user_dir, "home": "/home/user", "name": "user"}
        })
        .to_string()
    }

    fn titles(text: &str, latest: &str) -> Result<Vec<String>> {
        let release = LatestRelease {
            tag_name: latest.to_owned(),
        };
        let embeds = analyze(text, Some(&release)).context("dump should parse")?;
        embeds
            .into_iter()
            .map(|embed| {
                let value = serde_json::to_value(embed)?;
                Ok(value
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned())
            })
            .collect()
    }

    #[test]
    fn reads_escaped_windows_paths() -> Result<()> {
        let input = dump("15.2.1", r"C:\server", false);
        let parsed = serde_json::from_str::<SkinsRestorerDump<'_>>(&input)?;
        ensure!(parsed.user.dir == r"C:\server");
        Ok(())
    }

    #[test]
    fn ignores_text_that_is_not_a_dump() -> Result<()> {
        ensure!(analyze("java.lang.NullPointerException", None).is_none());
        ensure!(analyze(r#"{"other":true}"#, None).is_none());
        Ok(())
    }

    #[test]
    fn flags_outdated_versions_only() -> Result<()> {
        ensure!(
            titles(&dump("15.1.0", "/srv", false), "v15.2.0")?
                .contains(&"Update SkinsRestorer".to_owned())
        );
        ensure!(
            !titles(&dump("15.2.0", "/srv", false), "v15.2.0")?
                .contains(&"Update SkinsRestorer".to_owned())
        );
        // 15.10 is newer than 15.9, which a string comparison would get wrong.
        ensure!(
            !titles(&dump("15.10.0", "/srv", false), "15.9.0")?
                .contains(&"Update SkinsRestorer".to_owned())
        );
        Ok(())
    }

    #[test]
    fn notes_docker_and_hybrid_platforms() -> Result<()> {
        let titles = titles(&dump("15.2.0", "/home/container", true), "15.2.0")?;
        ensure!(titles.contains(&"Docker environment".to_owned()));
        ensure!(titles.contains(&"Unsupported hybrid platform".to_owned()));
        Ok(())
    }

    #[test]
    fn bounds_and_escapes_dump_fields() -> Result<()> {
        ensure!(dump_field("Paper `1.21`") == "Paper '1.21'");
        let long = dump_field(&"x".repeat(5_000));
        ensure!(long.chars().count() == MAX_DUMP_FIELD_CHARS && long.ends_with("..."));
        Ok(())
    }

    #[test]
    fn coerces_release_versions() -> Result<()> {
        ensure!(coerce_version("v15.2.1-SNAPSHOT") == Some(Version::new(15, 2, 1)));
        ensure!(coerce_version("15.2") == Some(Version::new(15, 2, 0)));
        ensure!(coerce_version("no version").is_none());
        Ok(())
    }
}
