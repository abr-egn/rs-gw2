use reqwest::{self, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::Value;

static KEY: &str = "9E827119-71EE-774C-88AE-B4D1D4B30868205CD293-9A69-415C-A3E3-4CB1E184722C";

fn fetch<T: DeserializeOwned>(client: &reqwest::blocking::Client, path: &str) -> T {
    let res = client.get(&format!("https://api.guildwars2.com/v2/{}?access_token={}", path, KEY))
        .send().unwrap();
    if res.status() != StatusCode::OK {
        panic!("{:?}", res);
    }
    res.json().unwrap()
}

fn main() {
    let client = reqwest::blocking::Client::new();
    println!("{}", fetch::<Value>(&client, "characters/Vidhara/recipes"));
}
