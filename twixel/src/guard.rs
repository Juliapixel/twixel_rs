#![allow(unused)]

use std::sync::Arc;

use hashbrown::HashSet;
use twixel_core::{
    IrcCommand, IrcMessage,
    irc_message::{AnySemantic, tags::OwnedTag},
    user::ChannelRoles,
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

pub trait Guard: Send + 'static {
    fn check(&self, ctx: &GuardContext) -> bool;

    fn clone_boxed(&self) -> Box<dyn Guard>;

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

#[derive(Clone)]
pub struct AndGuard<G1: Guard + Sized, G2: Guard + Sized> {
    lhs: G1,
    rhs: G2,
}

impl<G1: Guard + Clone, G2: Guard + Clone> Guard for AndGuard<G1, G2> {
    fn clone_boxed(&self) -> Box<dyn Guard> {
        Box::new(Self {
            lhs: self.lhs.clone(),
            rhs: self.rhs.clone(),
        })
    }

    fn check(&self, ctx: &GuardContext) -> bool {
        self.lhs.check(ctx) && self.rhs.check(ctx)
    }
}

#[derive(Clone)]
pub struct OrGuard<G1: Guard + Sized, G2: Guard + Sized> {
    lhs: G1,
    rhs: G2,
}

impl<G: Guard + Clone, G2: Guard + Clone> Guard for OrGuard<G, G2> {
    fn clone_boxed(&self) -> Box<dyn Guard> {
        Box::new(Self {
            lhs: self.lhs.clone(),
            rhs: self.rhs.clone(),
        })
    }

    fn check(&self, ctx: &GuardContext) -> bool {
        self.lhs.check(ctx) || self.rhs.check(ctx)
    }
}

/// Inverts the result of the inner guard
#[derive(Clone)]
pub struct NotGuard<G: Guard>(G);

impl<G: Guard + Clone> Guard for NotGuard<G> {
    fn clone_boxed(&self) -> Box<dyn Guard> {
        Box::new(Self(self.0.clone()))
    }

    fn check(&self, ctx: &GuardContext) -> bool {
        !self.0.check(ctx)
    }
}

/// Always returns true
#[derive(Clone)]
pub struct NoOpGuard;

impl Guard for NoOpGuard {
    fn check(&self, _ctx: &GuardContext) -> bool {
        true
    }

    fn clone_boxed(&self) -> Box<dyn Guard> {
        Box::new(Self)
    }
}

// pub struct AllGuard {
//     guards: Vec<Box<dyn Guard + 'static>>,
// }

// impl AllGuard {
//     pub fn new() -> Self {
//         Self { guards: vec![] }
//     }

//     pub fn add_guard(&mut self, guard: impl Guard + 'static) {
//         self.guards.push(Box::new(guard));
//     }
// }

// impl Guard for AllGuard {
//     fn check(&self, ctx: &GuardContext) -> bool {
//         if self.guards.is_empty() {
//             return true;
//         }
//         self.guards.iter().all(|g| g.check(ctx))
//     }

//     fn clone_boxed(&self) -> Box<dyn Guard> {
//         Box::new(Self {
//             guards: self.guards.iter().map(|g| (*g).clone_boxed()).collect(),
//         })
//     }
// }

// impl<A: Guard + 'static> FromIterator<A> for AllGuard {
//     fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
//         Self {
//             guards: iter
//                 .into_iter()
//                 .map(|i| Box::new(i) as Box<dyn Guard>)
//                 .collect(),
//         }
//     }
// }

// pub struct AnyGuard {
//     guards: Vec<Box<dyn Guard>>,
// }

// impl AnyGuard {
//     pub fn new() -> Self {
//         Self { guards: vec![] }
//     }

//     pub fn add_guard(&mut self, guard: impl Guard + 'static) {
//         self.guards.push(Box::new(guard));
//     }
// }

// impl Guard for AnyGuard {
//     fn check(&self, ctx: &GuardContext) -> bool {
//         if self.guards.is_empty() {
//             return true;
//         }
//         self.guards.iter().any(|g| g.check(ctx))
//     }
// }

// impl<A: Guard + 'static> FromIterator<A> for AnyGuard {
//     fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
//         Self {
//             guards: iter
//                 .into_iter()
//                 .map(|i| Box::new(i) as Box<dyn Guard>)
//                 .collect(),
//         }
//     }
// }

/// allows or forbids users based on their twitch ID
#[derive(Clone)]
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

    fn clone_boxed(&self) -> Box<dyn Guard> {
        Box::new(self.clone())
    }
}

/// allows or forbids channels based on their twitch ID
#[derive(Clone)]
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

    fn clone_boxed(&self) -> Box<dyn Guard> {
        Box::new(self.clone())
    }
}

/// returns true if the sender has any of the roles specified
#[derive(Clone, Copy)]
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

    fn clone_boxed(&self) -> Box<dyn Guard> {
        Box::new(*self)
    }
}
