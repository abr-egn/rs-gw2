use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::io::Write;
use std::time::{Duration, Instant};

use reqwest::{self, StatusCode};
use serde::{Deserialize};
use serde::de::DeserializeOwned;
//use serde::ser::Serialize;
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

    pub fn fetch<Out>(
        &mut self,
        auth: bool,
        path: &str,
    ) -> Result<Out>
    where
        Out: DeserializeOwned,
    {
        let since = Instant::now().duration_since(self.last);
        let tick = Duration::from_secs_f32(0.1);
        if since < tick {
            std::thread::sleep(tick - since);
        }
        self.last = Instant::now();

        let mut req = self.reqw.get(&format!("https://api.guildwars2.com/v2/{}", path))
            .query(&[("v", "latest")]);
        if auth {
            req = req.query(&[("access_token", KEY)]);
        }
        let req = req.build()?;
        let mut res = self.reqw.execute(req.try_clone().unwrap())?;
        if res.status() == StatusCode::TOO_MANY_REQUESTS {
            println!("\t429 sleep");
            std::thread::sleep(tick + tick);
            self.last = Instant::now();
            res = self.reqw.execute(req)?;
        }
        match res.status() {
            StatusCode::OK => (),
            StatusCode::PARTIAL_CONTENT => /*println!("[Partial {:?}]", res)*/ (),
            _ => failed!("{:?}", res),
        }

        Ok(res.json()?)
    }
}

fn ids_str(ids: &[i32]) -> String {
    let id_strs: Vec<String> = ids.iter().map(|id| format!("{}", id)).collect();
    id_strs.join(",")
}

#[derive(Debug, Clone, Deserialize)]
struct CharacterRecipes {
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

#[derive(Debug, Clone, Deserialize)]
struct Price {
    id: i32,
    whitelisted: bool,
    buys: Order,
    sells: Order,
}

#[derive(Debug, Clone, Deserialize)]
struct Order {
    quantity: i32,
    unit_price: i32,
}

fn main() -> Result<()> {
    let mut client = Client::new();

    let names: Vec<String> = client.fetch(true, "characters")?;
    println!("{:?}", names);
    let mut ids_by_char = HashMap::<&str, Vec<i32>>::new();
    let mut all_ids = HashSet::<i32>::new();
    for name in &names {
        let mut r: CharacterRecipes = client.fetch(true, &format!("characters/{}/recipes", name))?;
        println!("{}: {}", name, r.recipes.len());
        for id in &r.recipes {
            all_ids.insert(*id);
        }
        ids_by_char.entry(name).or_insert(vec![]).append(&mut r.recipes);
    }
    println!("known recipes: {}", all_ids.len());

    let mut recipes = HashMap::<i32, Recipe>::new();
    let id_vec: Vec<i32> = all_ids.iter().cloned().collect();
    for ids in id_vec.chunks(50) {
        let rs: Vec<Recipe> = client.fetch(false, &format!("recipes?ids={}", ids_str(ids)))?;
        for r in rs {
            recipes.insert(r.id, r);
        }
        print!(".");
        std::io::stdout().flush()?;
    }
    println!("");
    println!("retrieved recipes: {}", recipes.len());

    let mut all_items = HashSet::<i32>::new();
    for (&id, r) in &recipes {
        all_items.insert(id);
        for i in &r.ingredients {
            all_items.insert(i.item_id);
        }
    }
    println!("total items: {}", all_items.len());

    let mut prices = HashMap::<i32, Price>::new();
    let pid_vec: Vec<i32> = all_items.iter().cloned().collect();
    for ids in pid_vec.chunks(50) {
        let ps: Vec<Price> = client.fetch(false, &format!("commerce/prices?ids={}", ids_str(ids)))?;
        for p in ps {
            prices.insert(p.id, p);
        }
        print!(".");
        std::io::stdout().flush()?;
    }
    println!("");
    println!("retrieved prices: {}", prices.len());

    Ok(())
}
