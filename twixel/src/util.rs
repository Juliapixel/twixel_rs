use unicode_segmentation::UnicodeSegmentation;

pub mod db;

/// returns a &str that is at most `limit` bytes long
pub fn limit_str(value: &str, limit: usize) -> &str {
    let boundary = value
        .char_indices()
        .map(|(idx, c)| idx + c.len_utf8())
        .take_while(|end| *end <= limit)
        .last()
        .unwrap_or(0);
    if boundary == value.len() {
        value
    } else {
        value.split_at(boundary).0
    }
}

/// returns a &str that is at most `limit` chars long
pub fn limit_str_chars(value: &str, limit: usize) -> &str {
    let boundary = value
        .char_indices()
        .take(limit)
        .last()
        .map(|l| l.0 + l.1.len_utf8())
        .unwrap_or(0);
    if boundary == value.len() {
        value
    } else {
        value.split_at(boundary).0
    }
}

/// returns a &str that is at most `limit` chars long but maintains grapheme boundaries
pub fn limit_str_at_graphemes(value: &str, limit: usize) -> &str {
    let char_count = std::cell::RefCell::new(0);
    let boundary = value
        .grapheme_indices(true)
        .inspect(|(_i, g)| *char_count.borrow_mut() += g.chars().count())
        .take_while(|_| *char_count.borrow() <= limit)
        .last()
        .map(|(i, g)| i + g.len())
        .unwrap_or(0);
    if boundary == value.len() {
        value
    } else {
        value.split_at(boundary).0
    }
}

/// prevents message output from running commands over twitch IRC
pub fn sanitize_output(out: &mut String) {
    if out.starts_with('.') || out.starts_with('/') {
        out.insert(0, '\u{e0000}');
    }
}

#[test]
fn limiting() {
    // 5 bytes, 5 chars
    const ASCII: &str = "hello";
    // 7 bytes, 5 chars
    const MIXED: &str = "helĺó";
    // 13 bytes, 4 chars
    const EMOJI: &str = "🧞‍♀️";
    // le sanity check
    assert_eq!(EMOJI.chars().count(), 4);

    // on limit
    assert_eq!(limit_str(ASCII, 5), ASCII);
    assert_eq!(limit_str_chars(ASCII, 5), ASCII);
    assert_eq!(limit_str_at_graphemes(ASCII, 5), ASCII);
    // under limit
    assert_eq!(limit_str(ASCII, 6), ASCII);
    assert_eq!(limit_str_chars(ASCII, 6), ASCII);
    assert_eq!(limit_str_at_graphemes(ASCII, 6), ASCII);

    // on limit
    assert_eq!(limit_str(MIXED, 7), MIXED);
    assert_eq!(limit_str_chars(MIXED, 6), MIXED);
    assert_eq!(limit_str_at_graphemes(MIXED, 6), MIXED);
    // under limit
    assert_eq!(limit_str(MIXED, 8), MIXED);
    assert_eq!(limit_str_chars(MIXED, 7), MIXED);
    assert_eq!(limit_str_at_graphemes(MIXED, 7), MIXED);

    // on limit
    assert_eq!(limit_str(EMOJI, 13), EMOJI);
    assert_eq!(limit_str_chars(EMOJI, 4), EMOJI);
    assert_eq!(limit_str_at_graphemes(EMOJI, 4), EMOJI);
    // under limit
    assert_eq!(limit_str(EMOJI, 14), EMOJI);
    assert_eq!(limit_str_chars(EMOJI, 5), EMOJI);
    assert_eq!(limit_str_at_graphemes(EMOJI, 5), EMOJI);

    // should split the string
    assert_eq!(limit_str(ASCII, 4), "hell");
    assert_eq!(limit_str_chars(ASCII, 4), "hell");
    assert_eq!(limit_str_at_graphemes(ASCII, 4), "hell");

    assert_eq!(limit_str(MIXED, 4), "hel");
    assert_eq!(limit_str_chars(MIXED, 4), "helĺ");
    assert_eq!(limit_str_at_graphemes(MIXED, 4), "helĺ");

    assert_eq!(limit_str(EMOJI, 4), "🧞");
    assert_eq!(limit_str_chars(EMOJI, 3).chars().count(), 3);
    assert_eq!(limit_str_at_graphemes(EMOJI, 3), "");
}
