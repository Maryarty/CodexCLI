//! What the server lists to mention, as [`Mention`]s: the skills of `skills/list`, the plugins of
//! `plugin/installed` and the apps of the `/apps` directory (oracle `chatwidget/skills.rs`
//! `set_skills_from_response`, `app/plugin_mentions.rs`, `chatwidget/connector_mentions.rs`,
//! and the rows `chat_composer.rs` `mention_items` builds of them).

use codex_app_server_protocol::PluginAvailability;
use codex_app_server_protocol::PluginInstalledResponse;
use codex_app_server_protocol::PluginSummary;
use codex_app_server_protocol::SkillMetadata;
use codex_app_server_protocol::SkillsListResponse;

use super::candidate::uri;
use super::catalog::{Mention, MentionKind};
use crate::screens::select::apps::App;

/// The enabled skills of the entry for `cwd` (the first entry when none is named, since the
/// server answers for its session directory then): `skills_for_cwd` and
/// `enabled_skills_for_mentions`.
pub fn skills(response: SkillsListResponse, cwd: Option<&str>) -> Vec<Mention> {
    let entry = match cwd {
        Some(cwd) => response
            .data
            .into_iter()
            .find(|entry| entry.cwd.as_path() == std::path::Path::new(cwd)),
        None => response.data.into_iter().next(),
    };
    entry
        .map(|entry| {
            entry
                .skills
                .iter()
                .filter(|skill| skill.enabled)
                .map(skill)
                .collect()
        })
        .unwrap_or_default()
}

fn skill(skill: &SkillMetadata) -> Mention {
    let description = skill
        .interface
        .as_ref()
        .and_then(|interface| interface.short_description.as_deref())
        .or(skill.short_description.as_deref())
        .unwrap_or(&skill.description)
        .trim();
    Mention {
        kind: MentionKind::Skill,
        name: skill_display_name(skill),
        description: description.to_string(),
        insert: skill.name.clone(),
        path: Some(skill.path.as_str().to_string()),
        plugin_id: skill.plugin_id.clone(),
        capabilities: Vec::new(),
    }
}

/// `skills_helpers::skill_display_name`: the interface's name, else `skill (plugin)` for a
/// `plugin:skill` name, else the name.
fn skill_display_name(skill: &SkillMetadata) -> String {
    if let Some(display_name) = skill
        .interface
        .as_ref()
        .and_then(|interface| interface.display_name.as_deref())
    {
        return display_name.to_string();
    }
    if let Some((plugin, name)) = skill.name.split_once(':')
        && !plugin.is_empty()
        && !name.is_empty()
    {
        return format!("{name} ({plugin})");
    }
    skill.name.clone()
}

/// The installed, enabled plugins an admin has not switched off
/// (`plugin_mentions_from_installed_response`).
pub fn plugins(response: PluginInstalledResponse) -> Vec<Mention> {
    response
        .marketplaces
        .into_iter()
        .flat_map(|marketplace| {
            let name = marketplace.name;
            marketplace
                .plugins
                .into_iter()
                .filter_map(move |plugin| plugin_mention(&name, plugin))
        })
        .collect()
}

fn plugin_mention(marketplace: &str, plugin: PluginSummary) -> Option<Mention> {
    if !(plugin.installed
        && plugin.enabled
        && plugin.availability != PluginAvailability::DisabledByAdmin)
    {
        return None;
    }
    let interface = plugin.interface.as_ref();
    let display_name = interface
        .and_then(|interface| interface.display_name.as_deref())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(&plugin.name)
        .to_string();
    // The short description, else the marketplace's name.
    let description = interface
        .and_then(|interface| interface.short_description.as_deref())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .or_else(|| Some(marketplace.trim()).filter(|name| !name.is_empty()))
        .unwrap_or_default()
        .to_string();
    Some(Mention {
        kind: MentionKind::Plugin,
        name: display_name,
        description,
        insert: plugin.name,
        path: Some(uri("plugin", &plugin.id)),
        plugin_id: None,
        // The installed list carries no per-session capability summary.
        capabilities: Vec::new(),
    })
}

/// The apps that are installed and switched on (`on_connector_mentions_loaded`, `mention_items`).
pub fn apps(apps: &[App]) -> Vec<Mention> {
    apps.iter()
        .filter(|app| app.installed && app.enabled)
        .map(|app| Mention {
            kind: MentionKind::App,
            name: app.name.clone(),
            description: app.about().unwrap_or_default().to_string(),
            insert: connector_slug(&app.name),
            path: Some(uri("app", &app.id)),
            plugin_id: None,
            capabilities: Vec::new(),
        })
        .collect()
}

/// `connector_mention_slug`: ASCII letters and digits lowercased, every other character a `-`,
/// the ends trimmed, `app` for nothing left.
fn connector_slug(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "app".to_string()
    } else {
        slug.to_string()
    }
}

/// The catalog the popups list: skills, plugins, apps (the order of `mention_items`).
pub fn catalog(skills: &[Mention], plugins: &[Mention], apps: &[Mention]) -> Vec<Mention> {
    skills
        .iter()
        .chain(plugins)
        .chain(apps)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn skills_response(json: serde_json::Value) -> SkillsListResponse {
        serde_json::from_value(json).expect("a skills/list answer")
    }

    fn skill_json(name: &str, enabled: bool) -> serde_json::Value {
        serde_json::json!({
            "name": name,
            "description": "  Does a thing.  ",
            "path": (["/skills/", name, "/SKILL.md"].concat()),
            "scope": "user",
            "enabled": enabled,
            "pluginId": null,
        })
    }

    #[test]
    fn skills_of_the_asked_cwd_keep_the_enabled_ones_with_their_files() {
        let response = skills_response(serde_json::json!({
            "data": [
                {"cwd": "/other", "skills": [skill_json("elsewhere", true)], "errors": []},
                {"cwd": "/repo", "skills": [skill_json("figma", true), skill_json("off", false)],
                 "errors": []},
            ]
        }));
        let found = skills(response.clone(), Some("/repo"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, MentionKind::Skill);
        assert_eq!(found[0].name, "figma");
        assert_eq!(found[0].insert, "figma");
        assert_eq!(found[0].description, "Does a thing.");
        assert_eq!(found[0].path.as_deref(), Some("/skills/figma/SKILL.md"));
        // no cwd named: the server's own entry, the first
        assert_eq!(skills(response.clone(), None)[0].name, "elsewhere");
        assert_eq!(skills(response, Some("/nowhere")), Vec::new());
    }

    #[test]
    fn a_plugin_skill_reads_as_skill_and_plugin_and_an_interface_name_wins() {
        let mut named = skill_json("plugin:review", true);
        named["interface"] = serde_json::json!({"displayName": "Reviewer", "shortDescription": "Short"});
        let response = skills_response(serde_json::json!({
            "data": [{"cwd": "/repo", "skills": [skill_json("acme:lint", true), named], "errors": []}]
        }));
        let found = skills(response, Some("/repo"));
        assert_eq!(found[0].name, "lint (acme)");
        assert_eq!(found[0].insert, "acme:lint");
        assert_eq!(found[1].name, "Reviewer");
        assert_eq!(found[1].description, "Short");
    }

    fn plugin_json(
        id: &str,
        installed: bool,
        availability: &str,
        interface: serde_json::Value,
    ) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "name": id.split('@').next().unwrap_or(id),
            "source": {"type": "local", "path": "/plugins/p"},
            "installed": installed,
            "enabled": true,
            "installPolicy": "AVAILABLE",
            "authPolicy": "ON_USE",
            "availability": availability,
            "interface": interface,
        })
    }

    #[test]
    fn plugins_are_the_installed_ones_an_admin_has_not_switched_off() {
        let named = serde_json::json!({
            "displayName": " Google Calendar ", "shortDescription": " Schedule ", "capabilities": [], "screenshots": [], "screenshotUrls": []
        });
        let response: PluginInstalledResponse = serde_json::from_value(serde_json::json!({
            "marketplaces": [{
                "name": "debug", "path": null, "interface": null,
                "plugins": [
                    plugin_json("calendar@debug", true, "AVAILABLE", named),
                    plugin_json("bare@debug", true, "AVAILABLE", serde_json::Value::Null),
                    plugin_json("admin@debug", true, "DISABLED_BY_ADMIN", serde_json::Value::Null),
                    plugin_json("later@debug", false, "AVAILABLE", serde_json::Value::Null),
                ],
            }],
        }))
        .expect("a plugin/installed answer");
        let found = plugins(response);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].kind, MentionKind::Plugin);
        assert_eq!(found[0].name, "Google Calendar");
        assert_eq!(found[0].description, "Schedule");
        assert_eq!(found[0].path.as_deref(), Some("plugin://calendar@debug"));
        // no interface: the name, and the marketplace as the description
        assert_eq!(found[1].name, "bare");
        assert_eq!(found[1].description, "debug");
    }

    #[test]
    fn apps_must_be_installed_and_on_and_take_their_slug_from_the_name() {
        let app = |name: &str, installed: bool, enabled: bool| App {
            id: format!("id-{name}"),
            name: name.to_string(),
            description: Some(" Drive ".to_string()),
            install_url: None,
            installed,
            enabled,
        };
        let found = apps(&[
            app("Google Drive", true, true),
            app("Off", true, false),
            app("Not Here", false, true),
            app("!!", true, true),
        ]);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].insert, "google-drive");
        assert_eq!(found[0].path.as_deref(), Some("app://id-Google Drive"));
        assert_eq!(found[0].description, "Drive");
        assert_eq!(found[1].insert, "app");
    }
}
