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
) -> Option<serenity::CreateEmbed> {
    let dump = serde_json::from_str::<SkinsRestorerDump<'_>>(text).ok()?;
    Some(build_dump_embed(&dump, latest_release))
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
    /// `null` on proxy platforms, where the server-side proxy mode does not
    /// apply.
    proxy_mode: Option<bool>,
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

/// Builds one embed: problems that need action first, then the details as
/// compact fields.
fn build_dump_embed(
    dump: &SkinsRestorerDump<'_>,
    latest_release: Option<&LatestRelease>,
) -> serenity::CreateEmbed {
    let version = coerce_version(dump.build.version.as_ref());
    let latest_version = latest_release.and_then(|release| coerce_version(&release.tag_name));
    let version_display = version
        .as_ref()
        .map_or_else(|| dump.build.version.to_string(), ToString::to_string);

    let mut problems = Vec::new();
    if let (Some(version), Some(latest_version)) = (&version, &latest_version)
        && version < latest_version
    {
        problems.push(format!(
            "**Outdated:** `{version}` is installed. [Update to {latest_version}](https://modrinth.com/plugin/skinsrestorer/version/{latest_version})."
        ));
    }
    if dump.environment.hybrid {
        problems.push(
            "**Unsupported platform:** this server mixes mods and plugins. SkinsRestorer does not support hybrid platforms."
                .to_owned(),
        );
    }
    let mut description = if problems.is_empty() {
        "No known problems found.".to_owned()
    } else {
        problems.join("\n")
    };
    if dump.user.name == "?"
        || dump.user.dir == "/home/container"
        || dump.user.home == "/home/container"
    {
        description.push_str(
            "\n-# The server runs in a Docker container, possibly managed by Pterodactyl or Pelican. Mention this in support requests.",
        );
    }

    let code = |value: &str| format!("`{}`", dump_field(value));
    let yes_no = |value: bool| if value { "Yes" } else { "No" };
    serenity::CreateEmbed::new()
        .title("SkinsRestorer dump")
        .colour(if problems.is_empty() {
            0x58_65_F2
        } else {
            0xED_42_45
        })
        .description(description)
        .field("SkinsRestorer", code(&version_display), true)
        .field("Built", code(&dump.build.build_time), true)
        .field(
            "Platform",
            format!(
                "{} {}",
                code(&dump.platform.platform_name),
                code(&dump.platform.platform_version)
            ),
            true,
        )
        .field(
            "Environment",
            format!(
                "{} / {}",
                code(&dump.environment.platform),
                code(&dump.environment.platform_type)
            ),
            true,
        )
        .field("Plugins", dump.platform.plugins.len().to_string(), true)
        .field("Java", code(&dump.java.version), true)
        .field(
            "Operating system",
            format!("{} ({})", code(&dump.os.name), code(&dump.os.arch)),
            true,
        )
        .field(
            "Proxy mode",
            dump.plugin.proxy_mode.map_or("Not applicable", yes_no),
            true,
        )
        .field(
            "Debug mode",
            yes_no(dump.plugin.config_data.dev.debug),
            true,
        )
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

    fn description(text: &str, latest: &str) -> Result<String> {
        let release = LatestRelease {
            tag_name: latest.to_owned(),
        };
        let embed = analyze(text, Some(&release)).context("dump should parse")?;
        Ok(serde_json::to_value(embed)?
            .get("description")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned())
    }

    #[test]
    fn reads_escaped_windows_paths() -> Result<()> {
        let input = dump("15.2.1", r"C:\server", false);
        let parsed = serde_json::from_str::<SkinsRestorerDump<'_>>(&input)?;
        ensure!(parsed.user.dir == r"C:\server");
        Ok(())
    }

    #[test]
    fn reads_proxy_dumps_without_proxy_mode() -> Result<()> {
        let input =
            dump("15.2.1", "/srv", false).replace(r#""proxyMode":false"#, r#""proxyMode":null"#);
        ensure!(input.contains(r#""proxyMode":null"#));
        let embed = analyze(&input, None).context("proxy dump should parse")?;
        let embed = serde_json::to_value(embed)?;
        let proxy_mode = embed
            .get("fields")
            .and_then(serde_json::Value::as_array)
            .context("missing fields")?
            .iter()
            .find(|field| {
                field.get("name").and_then(serde_json::Value::as_str) == Some("Proxy mode")
            })
            .context("missing proxy mode field")?;
        ensure!(
            proxy_mode.get("value").and_then(serde_json::Value::as_str) == Some("Not applicable")
        );
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
        ensure!(description(&dump("15.1.0", "/srv", false), "v15.2.0")?.contains("**Outdated:**"));
        ensure!(
            description(&dump("15.2.0", "/srv", false), "v15.2.0")? == "No known problems found."
        );
        // 15.10 is newer than 15.9, which a string comparison would get wrong.
        ensure!(!description(&dump("15.10.0", "/srv", false), "15.9.0")?.contains("Outdated"));
        Ok(())
    }

    #[test]
    fn lists_problems_before_the_docker_note() -> Result<()> {
        let text = description(&dump("15.1.0", "/home/container", true), "15.2.0")?;
        let outdated = text
            .find("**Outdated:**")
            .context("missing outdated line")?;
        let hybrid = text
            .find("**Unsupported platform:**")
            .context("missing hybrid line")?;
        let docker = text
            .find("Docker container")
            .context("missing Docker note")?;
        ensure!(
            outdated < hybrid && hybrid < docker,
            "unexpected order: {text}"
        );
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
