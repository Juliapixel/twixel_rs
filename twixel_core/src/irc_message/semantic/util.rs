pub fn msg_from_param(param_str: &str) -> &str {
    let Some((_colon, text)) = param_str.split_at_checked(1) else {
        return "";
    };

    if text.starts_with('\u{0001}') && text.ends_with('\u{0001}') && text.len() > 1 {
        &text[1..(text.len() - 1)]
    } else {
        text
    }
}
