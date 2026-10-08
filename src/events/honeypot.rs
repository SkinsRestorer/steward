use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

use anyhow::{Context as _, Result, anyhow, ensure};
use poise::serenity_prelude as serenity;

use crate::state::AppState;

#[derive(Default)]
pub struct HoneypotService {
    pending_bans: Mutex<HashSet<(serenity::GuildId, serenity::UserId)>>,
}

impl HoneypotService {
    fn start_ban(
        &self,
        guild_id: serenity::GuildId,
        user_id: serenity::UserId,
    ) -> Result<Option<PendingBan<'_>>> {
        let key = (guild_id, user_id);
        let inserted = self
            .pending_bans
            .lock()
            .map_err(|_| anyhow!("honeypot pending bans lock was poisoned"))?
            .insert(key);
        Ok(inserted.then(|| PendingBan { service: self, key }))
    }
}

struct PendingBan<'a> {
    service: &'a HoneypotService,
    key: (serenity::GuildId, serenity::UserId),
}

impl Drop for PendingBan<'_> {
    fn drop(&mut self) {
        self.service
            .pending_bans
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.key);
    }
}

// Return true for trap messages, including exempt members and duplicate events.
pub async fn handle(
    ctx: &serenity::Context,
    data: &AppState,
    message: &serenity::Message,
) -> Result<bool> {
    let config = data.bot.honeypot;
    let Some(guild_id) = message.guild_id else {
        return Ok(false);
    };
    if config.channel_names.is_empty() {
        return Ok(false);
    }

    if !is_honeypot_channel(ctx, guild_id, message.channel_id, config.channel_names).await? {
        return Ok(false);
    }
    if message.author.bot
        || message.webhook_id.is_some()
        || !matches!(
            message.kind,
            serenity::MessageType::Regular | serenity::MessageType::InlineReply
        )
    {
        return Ok(true);
    }
    let Some(_pending_ban) = data.honeypot.start_ban(guild_id, message.author.id)? else {
        return Ok(true);
    };

    // Fetch current roles so a recent staff promotion is respected.
    let bot_id = ctx.cache.current_user().id;
    let (guild, member, bot_member) = tokio::try_join!(
        ctx.http.get_guild(guild_id),
        ctx.http.get_member(guild_id, message.author.id),
        ctx.http.get_member(guild_id, bot_id),
    )
    .context("failed to fetch current membership for honeypot ban")?;
    let everyone = serenity::RoleId::new(guild_id.get());
    ensure!(
        guild.roles.contains_key(&everyone)
            && member
                .roles
                .iter()
                .chain(&bot_member.roles)
                .all(|id| guild.roles.contains_key(id)),
        "honeypot cannot determine permissions because guild roles are missing"
    );
    if is_exempt(
        guild.owner_id,
        &member,
        guild.member_permissions(&member),
        config.exempt_role_ids,
    ) {
        return Ok(true);
    }
    ensure!(
        guild.member_permissions(&bot_member).ban_members(),
        "honeypot requires the Ban Members permission"
    );
    ensure!(
        highest_role(&guild.roles, everyone, &bot_member)
            > highest_role(&guild.roles, everyone, &member),
        "honeypot bot role must be above the target member's highest role"
    );

    let reason = format!(
        "Honeypot: posted in channel {} (message {})",
        message.channel_id, message.id
    );
    guild_id
        .ban_with_reason(
            &ctx.http,
            member.user.id,
            config.delete_message_days,
            reason,
        )
        .await
        .context("failed to ban honeypot sender")?;
    tracing::info!(
        bot = data.bot.id,
        %guild_id,
        channel_id = %message.channel_id,
        message_id = %message.id,
        user_id = %member.user.id,
        delete_message_days = config.delete_message_days,
        "banned honeypot sender"
    );
    Ok(true)
}

async fn is_honeypot_channel(
    ctx: &serenity::Context,
    guild_id: serenity::GuildId,
    channel_id: serenity::ChannelId,
    channel_names: &[&str],
) -> Result<bool> {
    let cached_match = ctx.cache.guild(guild_id).and_then(|guild| {
        guild
            .channels
            .get(&channel_id)
            .or_else(|| guild.threads.iter().find(|thread| thread.id == channel_id))
            .map(|channel| {
                channel.kind == serenity::ChannelType::Text
                    && channel_names.contains(&channel.name.as_str())
            })
    });
    let matches = if let Some(matches) = cached_match {
        matches
    } else {
        let channel = channel_id
            .to_channel(&ctx.http)
            .await
            .context("failed to resolve channel for honeypot check")?;
        channel.guild().is_some_and(|channel| {
            channel.guild_id == guild_id
                && channel.kind == serenity::ChannelType::Text
                && channel_names.contains(&channel.name.as_str())
        })
    };
    Ok(matches)
}

fn is_exempt(
    owner_id: serenity::UserId,
    member: &serenity::Member,
    permissions: serenity::Permissions,
    exempt_role_ids: &[u64],
) -> bool {
    member.user.bot
        || member.user.id == owner_id
        || permissions.intersects(
            serenity::Permissions::ADMINISTRATOR
                | serenity::Permissions::MANAGE_GUILD
                | serenity::Permissions::BAN_MEMBERS
                | serenity::Permissions::KICK_MEMBERS
                | serenity::Permissions::MODERATE_MEMBERS,
        )
        || member
            .roles
            .iter()
            .any(|role| exempt_role_ids.contains(&role.get()))
}

fn highest_role<'a>(
    roles: &'a HashMap<serenity::RoleId, serenity::Role>,
    everyone: serenity::RoleId,
    member: &serenity::Member,
) -> Option<&'a serenity::Role> {
    member
        .roles
        .iter()
        .chain(std::iter::once(&everyone))
        .filter_map(|id| roles.get(id))
        .max()
}

#[cfg(test)]
mod tests {
    use anyhow::{Result, ensure};

    use super::*;

    fn member(id: u64, roles: &[u64]) -> serenity::Member {
        let mut member = serenity::Member::default();
        member.user.id = serenity::UserId::new(id);
        member.roles = roles.iter().copied().map(serenity::RoleId::new).collect();
        member
    }

    #[test]
    fn exempts_owners_bots_staff_permissions_and_configured_roles() {
        let owner = serenity::UserId::new(1);
        let ordinary = member(2, &[3]);
        assert!(!is_exempt(
            owner,
            &ordinary,
            serenity::Permissions::SEND_MESSAGES,
            &[4]
        ));
        assert!(is_exempt(
            owner,
            &member(1, &[]),
            serenity::Permissions::empty(),
            &[]
        ));
        assert!(is_exempt(
            owner,
            &ordinary,
            serenity::Permissions::empty(),
            &[3]
        ));
        for permission in [
            serenity::Permissions::ADMINISTRATOR,
            serenity::Permissions::MANAGE_GUILD,
            serenity::Permissions::BAN_MEMBERS,
            serenity::Permissions::KICK_MEMBERS,
            serenity::Permissions::MODERATE_MEMBERS,
        ] {
            assert!(is_exempt(owner, &ordinary, permission, &[]));
        }
        let mut bot = ordinary;
        bot.user.bot = true;
        assert!(is_exempt(owner, &bot, serenity::Permissions::empty(), &[]));
    }

    #[test]
    fn hierarchy_includes_everyone_and_uses_discord_role_ordering() -> Result<()> {
        let roles: HashMap<_, _> = [(1, 0), (3, 2), (4, 2), (5, 1)]
            .into_iter()
            .map(|(id, position)| {
                let mut role = serenity::Role::default();
                role.id = serenity::RoleId::new(id);
                role.position = position;
                (role.id, role)
            })
            .collect();
        let everyone = serenity::RoleId::new(1);
        ensure!(
            highest_role(&roles, everyone, &member(2, &[])).map(|role| role.id) == Some(everyone)
        );
        ensure!(
            highest_role(&roles, everyone, &member(2, &[5, 4, 3])).map(|role| role.id)
                == Some(serenity::RoleId::new(3))
        );
        ensure!(
            highest_role(&roles, everyone, &member(2, &[3]))
                == highest_role(&roles, everyone, &member(6, &[3]))
        );
        Ok(())
    }

    #[test]
    fn deduplicates_pending_bans_per_guild_and_releases_on_drop() -> Result<()> {
        let service = HoneypotService::default();
        let guild = serenity::GuildId::new(1);
        let user = serenity::UserId::new(2);
        let first = service.start_ban(guild, user)?;
        ensure!(first.is_some());
        ensure!(service.start_ban(guild, user)?.is_none());
        ensure!(service.start_ban(guild, user)?.is_none());
        let other_guild = service.start_ban(serenity::GuildId::new(3), user)?;
        ensure!(other_guild.is_some());
        drop(first);
        ensure!(service.start_ban(guild, user)?.is_some());
        Ok(())
    }
}
