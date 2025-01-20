#![allow(unused)]

use std::sync::Arc;

use hashbrown::HashSet;
use twixel_core::{
    irc_message::{tags::OwnedTag, AnySemantic},
    user::ChannelRoles,
    IrcCommand, IrcMessage,
};

use crate::bot::BotData;

pub struct GuardContext<'a> {
    pub data_store: &'a BotData,
    pub message: &'a AnySemantic<'a>,
}

impl<'a> GuardContext<'a> {
    pub fn data_store(&'a self) -> &'a BotData {
        self.data_store
    }

    pub fn message(&self) -> &AnySemantic<'a> {
        self.message
    }
}

pub trait Guard {
    fn check(&self, ctx: &GuardContext) -> bool;

    fn and<G: Guard>(self, rhs: G) -> AndGuard<Self, G>
    where
        Self: Sized,
    {
        AndGuard { lhs: self, rhs }
    }

    fn or<G: Guard>(self, rhs: G) -> OrGuard<Self, G>
    where
        Self: Sized,
    {
        OrGuard { lhs: self, rhs }
    }

    fn not(self) -> NotGuard<Self>
    where
        Self: Sized,
    {
        NotGuard(self)
    }
}

pub struct AndGuard<G1: Guard + Sized, G2: Guard + Sized> {
    lhs: G1,
    rhs: G2,
}

impl<G1: Guard, G2: Guard> Guard for AndGuard<G1, G2> {
    fn check(&self, ctx: &GuardContext) -> bool {
        self.lhs.check(ctx) && self.rhs.check(ctx)
    }
}

pub struct OrGuard<G1: Guard + Sized, G2: Guard + Sized> {
    lhs: G1,
    rhs: G2,
}

impl<G: Guard, G2: Guard> Guard for OrGuard<G, G2> {
    fn check(&self, ctx: &GuardContext) -> bool {
        self.lhs.check(ctx) || self.rhs.check(ctx)
    }
}

/// Inverts the result of the inner guard
pub struct NotGuard<G: Guard>(G);

impl<G: Guard> Guard for NotGuard<G> {
    fn check(&self, ctx: &GuardContext) -> bool {
        !self.0.check(ctx)
    }
}

/// Always returns true
pub struct NoOpGuard;

impl Guard for NoOpGuard {
    fn check(&self, _ctx: &GuardContext) -> bool {
        true
    }
}

pub struct AllGuard {
    guards: Vec<Box<dyn Guard + 'static>>,
}

impl AllGuard {
    pub fn new() -> Self {
        Self { guards: vec![] }
    }

    pub fn add_guard(&mut self, guard: impl Guard + 'static) {
        self.guards.push(Box::new(guard));
    }
}

impl Guard for AllGuard {
    fn check(&self, ctx: &GuardContext) -> bool {
        if self.guards.is_empty() {
            return true;
        }
        self.guards.iter().all(|g| g.check(ctx))
    }
}

impl<A: Guard + 'static> FromIterator<A> for AllGuard {
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        Self {
            guards: iter
                .into_iter()
                .map(|i| Box::new(i) as Box<dyn Guard>)
                .collect(),
        }
    }
}

pub struct AnyGuard {
    guards: Vec<Box<dyn Guard>>,
}

impl AnyGuard {
    pub fn new() -> Self {
        Self { guards: vec![] }
    }

    pub fn add_guard(&mut self, guard: impl Guard + 'static) {
        self.guards.push(Box::new(guard));
    }
}

impl Guard for AnyGuard {
    fn check(&self, ctx: &GuardContext) -> bool {
        if self.guards.is_empty() {
            return true;
        }
        self.guards.iter().any(|g| g.check(ctx))
    }
}

impl<A: Guard + 'static> FromIterator<A> for AnyGuard {
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        Self {
            guards: iter
                .into_iter()
                .map(|i| Box::new(i) as Box<dyn Guard>)
                .collect(),
        }
    }
}

pub struct CooldownGuard {
    cooldown: std::time::Duration,
    last_used: parking_lot::Mutex<chrono::DateTime<chrono::Utc>>,
}

impl Guard for CooldownGuard {
    fn check(&self, ctx: &GuardContext) -> bool {
        let last_used = self.last_used.lock();
        let ts = ctx.message.get_timestamp().unwrap_or(chrono::Utc::now());
        let elapsed = (ts - *last_used);
        // either some number of seconds or 0
        let elapsed =
            std::time::Duration::from_secs(elapsed.num_seconds().try_into().unwrap_or_default());

        // on cooldown
        if elapsed <= self.cooldown {
            false
        } else {
            *self.last_used.lock() = ts;
            true
        }
    }
}

/// allows or forbids users based on their twitch ID
pub struct UserGuard {
    user_ids: HashSet<String>,
}

impl UserGuard {
    pub fn allow(user_ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            user_ids: user_ids.into_iter().map(Into::into).collect(),
        }
    }

    pub fn forbid(user_ids: impl IntoIterator<Item = impl Into<String>>) -> NotGuard<Self> {
        Self {
            user_ids: user_ids.into_iter().map(Into::into).collect(),
        }
        .not()
    }
}

impl Guard for UserGuard {
    fn check(&self, ctx: &GuardContext) -> bool {
        let AnySemantic::PrivMsg(msg) = ctx.message else {
            return false;
        };
        msg.sender_id()
            .map(|t| self.user_ids.contains(t))
            .unwrap_or(false)
    }
}

/// allows or forbids channels based on their twitch ID
pub struct ChannelGuard {
    channel_ids: HashSet<String>,
}

impl ChannelGuard {
    pub fn allow(channel_id: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            channel_ids: channel_id.into_iter().map(Into::into).collect(),
        }
    }

    pub fn forbid(channel_id: impl IntoIterator<Item = impl Into<String>>) -> NotGuard<Self> {
        Self {
            channel_ids: channel_id.into_iter().map(Into::into).collect(),
        }
        .not()
    }
}

impl Guard for ChannelGuard {
    fn check(&self, ctx: &GuardContext) -> bool {
        let AnySemantic::PrivMsg(msg) = ctx.message else {
            return false;
        };
        msg.channel_id()
            .map(|t| self.channel_ids.contains(t))
            .unwrap_or(false)
    }
}

/// returns true if the sender has any of the roles specified
pub struct RoleGuard {
    roles: ChannelRoles,
}

impl RoleGuard {
    pub fn new(roles: ChannelRoles) -> Self {
        Self { roles }
    }
}

impl Guard for RoleGuard {
    fn check(&self, ctx: &GuardContext) -> bool {
        let AnySemantic::PrivMsg(msg) = ctx.message else {
            return false;
        };

        self.roles.intersects(msg.sender_roles())
    }
}
