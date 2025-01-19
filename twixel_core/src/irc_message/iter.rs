use memchr::memchr;

use crate::IrcMessage;

use super::error::IrcMessageParseError;

pub struct IrcMessageParseIter<'a> {
    pos: usize,
    inner: &'a str,
}

impl<'a> IrcMessageParseIter<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            inner: text,
            pos: 0,
        }
    }
}

impl<'a> Iterator for IrcMessageParseIter<'a> {
    type Item = Result<IrcMessage<'a>, IrcMessageParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = memchr(b'\n', &self.inner.as_bytes()[self.pos..])?;
        let parsed = self.inner[self.pos..=(self.pos + next)].parse::<IrcMessage<'_>>();
        self.pos += next + 1;
        Some(parsed)
    }
}

pub struct BadgeIter<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> BadgeIter<'a> {
    pub fn new(src: &'a str) -> Self {
        Self { src, pos: 0 }
    }
}

impl<'a> Iterator for BadgeIter<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let cur_slice = &self.src[self.pos..];
        let boundary = match memchr::memchr(b',', cur_slice.as_bytes()) {
            Some(f) => f,
            None => cur_slice.len(),
        };
        let single_badge = &cur_slice[..boundary];
        let slash_pos =
            memchr::memchr(b'/', single_badge.as_bytes()).expect("badge did not containt a \"/\"");
        self.pos += boundary + 1;

        single_badge
            .split_at_checked(slash_pos)
            .map(|(k, v)| (k, v.split_at(1).1))
    }
}

#[test]
fn badge_iter() {
    const TEST_BADGES: &str = "subscriber/3000,mod/1,vip/1";
    let mut iter = BadgeIter::new(TEST_BADGES);
    assert_eq!(iter.next().unwrap(), ("subscriber", "3000"));
    assert_eq!(iter.next().unwrap(), ("mod", "1"));
    assert_eq!(iter.next().unwrap(), ("vip", "1"));
}
