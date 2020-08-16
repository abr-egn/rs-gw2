use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::time::{Duration, Instant};

use reqwest::{self, StatusCode};
use serde::{Deserialize};
use serde::de::DeserializeOwned;
//use serde_json::Value;

static KEY: &str = "9E827119-71EE-774C-88AE-B4D1D4B30868205CD293-9A69-415C-A3E3-4CB1E184722C";

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
struct Failed(String);

impl fmt::Display for Failed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Failed({})", self.0)
    }
}

impl Error for Failed {}

macro_rules! failed {
    ($($arg:expr),*) => {
        return Err(Box::new(Failed(format!($($arg),*))))
    };
}

struct Client {
    reqw: reqwest::blocking::Client,
    last: Instant,
}

impl Client {
    pub fn new() -> Self {
        Client {
            reqw: reqwest::blocking::Client::new(),
            last: Instant::now(),
        }
    }

    pub fn fetch<T: DeserializeOwned>(
        &mut self,
        auth: bool,
        path: &str,
    ) -> Result<T> {
        let since = Instant::now().duration_since(self.last);
        let tick = Duration::from_secs_f32(0.1);
        if since < tick {
            //println!("throttle: {}", (tick - since).as_secs_f32());
            std::thread::sleep(tick - since);
        }
        self.last = Instant::now();

        let mut req = self.reqw.get(&format!("https://api.guildwars2.com/v2/{}", path));
        if auth {
            req = req.query(&[("access_token", KEY)]);
        }
        let res = req.send()?;
        //println!("{:?}", res);
        if res.status() != StatusCode::OK {
            failed!("{:?}", res)
        }

        Ok(res.json()?)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Recipes {
    recipes: Vec<i32>,
}

#[derive(Debug, Clone, Deserialize)]
struct Ingredient {
    item_id: i32,
    count: i32,
}

#[derive(Debug, Clone, Deserialize)]
struct Recipe {
    #[serde(rename = "type")]
    typ: String,
    output_item_id: i32,
    output_item_count: i32,
    min_rating: i32,
    time_to_craft_ms: i32,
    disciplines: Vec<String>,
    flags: Vec<String>,
    ingredients: Vec<Ingredient>,
    id: i32,
    chat_link: String,
}

fn main() -> Result<()> {
    let mut client = Client::new();

    let names: Vec<String> = client.fetch(true, "characters")?;
    println!("{:?}", names);
    let mut ids_by_char = HashMap::<&str, Vec<i32>>::new();
    let mut all_ids = HashSet::<i32>::new();
    for name in &names {
        let mut r: Recipes = client.fetch(true, &format!("characters/{}/recipes", name))?;
        println!("{}: {}", name, r.recipes.len());
        for id in &r.recipes {
            all_ids.insert(*id);
        }
        ids_by_char.entry(name).or_insert(vec![]).append(&mut r.recipes);
    }
    println!("{}", all_ids.len());
    Ok(())
}
