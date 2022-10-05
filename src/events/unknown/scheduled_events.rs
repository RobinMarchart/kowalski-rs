use serde::{Serialize, Deserialize};
use serenity::model::{prelude::{ScheduledEventId, GuildId, ChannelId, ScheduledEventStatus, ScheduledEventType}, Timestamp};

#[non_exhaustive]
#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct GuildScheduledEventUser{
    pub guild_scheduled_event_id:ScheduledEventId,
    pub user_id:UserId,
    pub guild_id:GuildId
}
