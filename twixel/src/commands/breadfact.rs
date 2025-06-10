use futures::TryFutureExt;

pub async fn bread_fact() -> String {
    match reqwest::get("https://website-backend.w3champions.com/api/players/breadworms%232156/game-mode-stats?gateWay=20&season=21")
        .and_then(|r| r.json::<serde_json::Value>())
        .await
    {
        Ok(f) => f.get(0).unwrap().get("mmr").unwrap().to_string(),
        Err(e) => e.to_string(),
    }
}
