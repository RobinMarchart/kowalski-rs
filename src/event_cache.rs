use parking_lot::RwLock;
use serenity::{
    model::{
        id::{GuildId, ScheduledEventId, UserId},
        Timestamp,
    },
    prelude::{Context, TypeMapKey},
};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    ops::Deref,
    sync::Arc,
};

struct Event {
    users: HashSet<UserId>,
    name: String,
    description: Option<String>,
    start: Timestamp,
    end: Option<Timestamp>,
}

enum NotYet<T> {
    Exists(T),
    Modify(Vec<Box<dyn FnOnce(&mut T) + Send + Sync>>),
}

impl<T> NotYet<T> {
    pub fn modify<F: FnOnce(&mut T) + Send + Sync>(&mut self, f: F) {
        use NotYet::*;
        match self {
            Exists(v) => f(v),
            Modify(hooks) => hooks.push(Box::new(f)),
        }
    }
    pub fn get(&self) -> Option<&T> {
        use NotYet::*;
        match self {
            Exists(v) => Some(v),
            Modify(_) => None,
        }
    }
    pub fn get_mut(&mut self) -> Option<&mut T> {
        use NotYet::*;
        match self {
            Exists(v) => Some(v),
            Modify(_) => None,
        }
    }
    pub fn set(&mut self, t: T) -> bool {
        use NotYet::*;
        match self {
            Exists(v) => false,
            Modify(_) => {
                let mut hooks = NotYet::Exists(t);
                std::mem::swap(self, &mut hooks);
                if let Modify(hooks) = hooks {
                    if let Exists(v) = self {
                        for hook in hooks {
                            hook(v);
                        }
                    } else {
                        panic!()
                    }
                } else {
                    panic!()
                }
                true
            }
        }
    }
}

type Events = HashMap<ScheduledEventId, NotYet<Event>>;
type Guilds = RwLock<HashMap<GuildId, Events>>;

#[derive(Debug, Eq)]
struct IdentityArc<T: ?Sized> {
    inner: Arc<T>,
}

impl<T: ?Sized> Clone for IdentityArc<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: ?Sized> PartialEq for IdentityArc<T> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<T: ?Sized> Hash for IdentityArc<T> {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(Arc::as_ptr(&self.inner) as *const () as usize)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Removed,
    Added,
}

pub type EventCacheHook = dyn Fn(GuildId, ScheduledEventId, UserId, ChangeKind) + Sync + Send;

struct EventCache {
    guilds: Guilds,
    hooks: RwLock<HashSet<IdentityArc<EventCacheHook>>>,
}

struct EventCacheKey {}
impl TypeMapKey for EventCacheKey {
    type Value = EventCache;
}

async fn get_event_cache(context: &Context) -> tokio::sync::RwLockReadGuard<EventCache> {
    let map = context.data.read().await;
    if map.contains_key::<EventCacheKey>() {
        tokio::sync::RwLockReadGuard::map(map, |map| map.get::<EventCacheKey>().unwrap())
    } else {
        drop(map);
        let mut handle = context.data.write().await;
        handle.insert::<EventCacheKey>(EventCache {
            guilds: RwLock::new(HashMap::new()),
            hooks: RwLock::new(HashSet::new()),
        });
        tokio::sync::RwLockReadGuard::map(handle.downgrade(), |map| {
            map.get::<EventCacheKey>().unwrap()
        })
    }
}

pub async fn remove_server<G: Into<GuildId>>(context: &Context, guild: G) {
    let guild: GuildId = guild.into();
    let event_cache = get_event_cache(context).await;
    let removed = event_cache.guilds.write().remove(&guild);
    if let Some(removed) = removed {
        let hooks = (*event_cache.hooks.read().deref()).clone();
        drop(event_cache);
        for (event, users) in removed {
            if let NotYet::Exists(users) = users {
                for user in users.users {
                    for hook in &hooks {
                        (hook.inner)(guild, event, user, ChangeKind::Removed);
                    }
                }
            }
        }
    }
}

pub async fn update_server<G: Into<GuildId>>(context: &Context, guild: G) {
    let guild: GuildId = guild.into();
    let events = guild.scheduled_events(context, false);
}

fn modify_server<T,G: Into<GuildId>,F:FnOnce(&mut Events)->T>(event_cache: & EventCache, guild: G,f:F)->T {
    f(event_cache.guilds.write().entry(guild.into()).or_insert_with(HashMap::new))
}

pub async fn remove_event<G: Into<GuildId>, E: Into<ScheduledEventId>>(
    context: &Context,
    guild: G,
    event: E,
) {
    let guild: GuildId = guild.into();
    let event: ScheduledEventId = event.into();
    let event_cache = get_event_cache(context).await;
    let removed=modify_server(event_cache, guild, |guild|guild.remove(&event));
    if let Some(removed) = removed {
        let hooks = (*event_cache.hooks.read().deref()).clone();
        drop(event_cache);
        for user in removed {
            for hook in &hooks {
                (hook.inner)(guild, event, user, ChangeKind::Removed);
            }
        }
    }
}

pub async fn add_event<G: Into<GuildId>, E: Into<ScheduledEventId>>(
    context: &Context,
    guild: G,
    event: E,
) -> bool {
    let guild: GuildId = guild.into();
    let event: ScheduledEventId = event.into();
    let event_cache = get_event_cache(context).await;
    let mut guilds = event_cache.guilds.write();
    if let Some(guild) = guilds.get_mut(&guild) {
        if !guild.contains_key(&event) {
            guild.insert(event, HashSet::new());
            true
        } else {
            false
        }
    } else {
        false
    }
}

pub async fn remove_user<G: Into<GuildId>, E: Into<ScheduledEventId>, U: Into<UserId>>(
    context: &Context,
    guild: G,
    event: E,
    user: U,
) {
    let guild: GuildId = guild.into();
    let event: ScheduledEventId = event.into();
    let user: UserId = user.into();
    let event_cache = get_event_cache(context).await;
    let removed = event_cache
        .guilds
        .write()
        .get_mut(&guild)
        .and_then(|guild| guild.get_mut(&event))
        .map(|event| event.remove(&user));
    if let Some(true) = removed {
        let hooks = (*event_cache.hooks.read().deref()).clone();
        drop(event_cache);
        for hook in &hooks {
            (hook.inner)(guild, event, user, ChangeKind::Removed);
        }
    }
}

pub async fn add_user<G: Into<GuildId>, E: Into<ScheduledEventId>, U: Into<UserId>>(
    context: &Context,
    guild: G,
    event: E,
    user: U,
) {
    let guild: GuildId = guild.into();
    let event: ScheduledEventId = event.into();
    let user: UserId = user.into();
    let event_cache = get_event_cache(context).await;
    let added = event_cache
        .guilds
        .write()
        .get_mut(&guild)
        .and_then(|guild| guild.get_mut(&event))
        .map(|event| event.insert(user));
    if let Some(true) = added {
        let hooks = (*event_cache.hooks.read().deref()).clone();
        drop(event_cache);
        for hook in &hooks {
            (hook.inner)(guild, event, user, ChangeKind::Removed);
        }
    }
}
