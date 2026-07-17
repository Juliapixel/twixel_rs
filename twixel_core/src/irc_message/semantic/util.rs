pub(crate) fn msg_from_param(param_str: &str) -> &str {
    if param_str.starts_with("\u{0001}ACTION ") && param_str.ends_with('\u{0001}') {
        &param_str[("\u{0001}ACTION ".len())..(param_str.len() - 1)]
    } else {
        param_str
    }
}
