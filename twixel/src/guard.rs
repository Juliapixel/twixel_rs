#![allow(unused)]

use twixel_core::{irc_message::{tags::OwnedTag, AnySemantic}, IrcCommand, IrcMessage};

pub struct GuardContext<'a> {
    // pub channel_info: &'a ChannelInfo,
    pub message: &'a AnySemantic<'a>,
}

impl<'a> GuardContext<'a> {
    // pub fn channel_info(&self) -> &'a ChannelInfo {
    //     self.channel_info
    // }

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
        let elapsed = std::time::Duration::from_secs(elapsed.num_seconds().try_into().unwrap_or_default());

        // on cooldown
        if elapsed <= self.cooldown {
            false
        } else {
            *self.last_used.lock() = ts;
            true
        }
    }
}

pub struct UserGuard {
    user_id: String,
}

impl UserGuard {
    pub fn allow(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
        }
    }

    pub fn forbid(user_id: impl Into<String>) -> NotGuard<Self> {
        NotGuard(Self {
            user_id: user_id.into(),
        })
    }
}

impl Guard for UserGuard {
    fn check(&self, ctx: &GuardContext) -> bool {
        let AnySemantic::PrivMsg(msg) = ctx.message else {
            return false;
        };
        msg.sender_id()
            .map(|t| t == self.user_id)
            .unwrap_or(false)
    }
}

pub struct ChannelGuard {
    channel_id: String,
}

impl ChannelGuard {
    pub fn allow(channel_id: impl Into<String>) -> Self {
        Self {
            channel_id: channel_id.into(),
        }
    }

    pub fn forbid(channel_id: impl Into<String>) -> NotGuard<Self> {
        NotGuard(Self {
            channel_id: channel_id.into(),
        })
    }
}

impl Guard for ChannelGuard {
    fn check(&self, ctx: &GuardContext) -> bool {
        self.channel_id.as_str() == ctx.message().get_tag(OwnedTag::RoomId).unwrap_or_default()
    }
}
