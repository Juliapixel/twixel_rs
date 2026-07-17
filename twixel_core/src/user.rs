bitflags::bitflags! {
    /// Bitflags indicating a user's roles in a channel
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Copy, Default, Debug)]
    pub struct ChannelRoles: u8 {
        /// Whether the user is a moderator
        const Moderator = 1;
        /// Whether the user is a VIP
        const Vip = 1 << 1;
        /// Whether the user is a subscriber
        const Subscriber = 1 << 2;
        /// Whether the user is the broadcaster
        const Broadcaster = 1 << 3;
        /// Whether the user is the lead moderator
        const LeadModerator = 1 << 4;
    }
}

impl ChannelRoles {
    const PRIVILEGED_MASK: ChannelRoles = ChannelRoles::empty()
        .union(ChannelRoles::Moderator)
        .union(ChannelRoles::LeadModerator)
        .union(ChannelRoles::Vip)
        .union(ChannelRoles::Broadcaster);

    /// `true` if the user has higher chat privileges in IRC
    pub fn is_privileged(&self) -> bool {
        self.intersects(Self::PRIVILEGED_MASK)
    }
}
