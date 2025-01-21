bitflags::bitflags! {
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Copy, Default, Debug)]
    pub struct ChannelRoles: u8 {
        const Moderator = 1;
        const Vip = 1 << 1;
        const Subscriber = 1 << 2;
        const Broadcaster = 1 << 3;
    }
}

impl ChannelRoles {
    const PRIVILEGED_MASK: ChannelRoles = ChannelRoles::empty()
        .union(ChannelRoles::Moderator)
        .union(ChannelRoles::Vip)
        .union(ChannelRoles::Broadcaster);

    /// whether you have higher chat privileges in IRC
    pub fn is_privileged(&self) -> bool {
        self.intersects(Self::PRIVILEGED_MASK)
    }
}
