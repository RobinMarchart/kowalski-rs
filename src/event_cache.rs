
use serenity::{model::id::{ScheduledEventId,UserId,GuildId}, prelude::{TypeMapKey, Context}};
use std::collections::{HashMap,HashSet};
use parking_lot::RwLock;

use crate::commands::guild;

struct EventCache{
   map: RwLock<HashMap<GuildId,RwLock<HashMap<ScheduledEventId,RwLock<HashSet<UserId>>>>>>,
}

pub struct EventCacheKey{}
impl TypeMapKey for EventCacheKey{
    type Value=EventCache;
}

async fn get_event_cache(context:&Context)->tokio::sync::RwLockReadGuard<EventCache>{
    let map= context.data.read().await;
    if map.contains_key::<EventCacheKey>(){
        tokio::sync::RwLockReadGuard::map(map, |map|map.get::<EventCacheKey>().unwrap())
    }else{
        drop(map);
        let handle=context.data.write().await;
        handle.insert::<EventCacheKey>(EventCache{map:RwLock::new(HashMap::new())});
        tokio::sync::RwLockReadGuard::map(handle.downgrade(), |map|map.get::<EventCacheKey>().unwrap())
    }
}

pub async fn remove_server(context:&Context,guild:Into<GuildId>){
    get_event_cache(context).await;
}
