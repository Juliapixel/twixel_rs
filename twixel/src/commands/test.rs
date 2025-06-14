use crate::util::db::TwixelUser;

pub async fn test(user: TwixelUser) -> String {
    format!(
        "you're user id: {}, added on: {}",
        user.id(),
        user.creation_ts()
    )
}
