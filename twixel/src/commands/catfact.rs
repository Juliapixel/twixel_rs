use futures::TryFutureExt;

#[derive(serde::Deserialize)]
struct CatFact {
    fact: String,
}

pub async fn cat_fact() -> String {
    match reqwest::get("https://catfact.ninja/fact")
        .and_then(|r| r.json::<CatFact>())
        .await
    {
        Ok(f) => f.fact,
        Err(e) => e.to_string(),
    }
}
