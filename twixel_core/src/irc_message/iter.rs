use std::iter::Map;

use memchr::{memchr, Memchr};

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
