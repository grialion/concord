use std::fmt;

use crate::discord::ids::{
    Id,
    marker::{ChannelMarker, GuildMarker, UserMarker},
};

use crate::discord::MemberInfo;

#[derive(Clone, Eq, PartialEq)]
pub struct VoiceStateInfo {
    pub guild_id: Id<GuildMarker>,
    pub channel_id: Option<Id<ChannelMarker>>,
    pub user_id: Id<UserMarker>,
    pub session_id: Option<String>,
    pub member: Option<MemberInfo>,
    pub deaf: bool,
    pub mute: bool,
    pub self_deaf: bool,
    pub self_mute: bool,
    pub self_stream: bool,
}

#[cfg(test)]
#[allow(dead_code)]
impl VoiceStateInfo {
    pub(crate) fn test(
        guild_id: Id<GuildMarker>,
        channel_id: Option<Id<ChannelMarker>>,
        user_id: Id<UserMarker>,
    ) -> Self {
        Self {
            guild_id,
            channel_id,
            user_id,
            session_id: None,
            member: None,
            deaf: false,
            mute: false,
            self_deaf: false,
            self_mute: false,
            self_stream: false,
        }
    }
}

impl fmt::Debug for VoiceStateInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VoiceStateInfo")
            .field("guild_id", &self.guild_id)
            .field("channel_id", &self.channel_id)
            .field("user_id", &self.user_id)
            .field(
                "session_id",
                &self.session_id.as_ref().map(|_| "<redacted>"),
            )
            .field("member", &self.member)
            .field("deaf", &self.deaf)
            .field("mute", &self.mute)
            .field("self_deaf", &self.self_deaf)
            .field("self_mute", &self.self_mute)
            .field("self_stream", &self.self_stream)
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct VoiceServerInfo {
    pub guild_id: Id<GuildMarker>,
    pub endpoint: Option<String>,
    pub token: String,
}

#[cfg(test)]
#[allow(dead_code)]
impl VoiceServerInfo {
    pub(crate) fn test(guild_id: Id<GuildMarker>) -> Self {
        Self {
            guild_id,
            endpoint: None,
            token: String::new(),
        }
    }
}

impl fmt::Debug for VoiceServerInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VoiceServerInfo")
            .field("guild_id", &self.guild_id)
            .field("endpoint", &self.endpoint)
            .field("token", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VoiceConnectionStatus {
    Connecting,
    Connected,
    Disconnected,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VoiceSoundKind {
    Join,
    Leave,
}
