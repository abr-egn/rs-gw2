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
    vendor: Option<()>,
}

#[derive(Debug, Clone, Deserialize)]
struct Order {
    quantity: i32,
    unit_price: i32,
}

#[derive(Debug, Clone)]
struct Profit {
    id: i32,
    recipe: i32,
    sale_total: i32,
    craft_total: i32,
}

#[derive(Debug, Clone, Deserialize)]
struct Item {
    name: String,
    description: Option<String>,
    #[serde(rename = "type")]
    typ: String,
    level: i32,
    rarity: String,
    vendor_value: i32,
    game_types: Vec<String>,
    flags: Vec<String>,
    restrictions: Vec<String>,
    id: i32,
    chat_link: String,
    icon: String,
}

fn vendor(prices: &mut HashMap<i32, Price>, id: i32, price: i32) {
    prices.insert(id, Price {
        id: id,
        whitelisted: true,
        buys: Order { quantity: 0, unit_price: 0 },
        sells: Order { quantity: 1, unit_price: price },
        vendor: Some(()),
    });
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
    // Thermocatalytic Reagent
    vendor(&mut prices, 46747, 150);
    // Spool of Gossamer Thread
    vendor(&mut prices, 19790, 64);
    // Spool of Silk Thread
    vendor(&mut prices, 19791, 48);
    // Spool of Linen Thread
    vendor(&mut prices, 19793, 32);
    // Spool of Cotton Thread
    vendor(&mut prices, 19794, 24);
    // Spool of Wool Thread
    vendor(&mut prices, 19789, 16);
    // Spool of Jute Thread
    vendor(&mut prices, 19792, 8);
    let pid_vec: Vec<i32> = all_items.iter().cloned().collect();
    for ids in pid_vec.chunks(50) {
        let ps: Vec<Price> = client.fetch(false, &format!("commerce/prices?ids={}", ids_str(ids)))?;
        for p in ps {
            let mut to_insert = p.clone();
            if let Some(other) = prices.get(&p.id) {
                if other.sells.unit_price < to_insert.sells.unit_price {
                    to_insert = other.clone();
                }
            }
            prices.insert(p.id, to_insert);
        }
        print!(".");
        std::io::stdout().flush()?;
    }
    println!("");
    println!("retrieved prices: {}", prices.len());

    let mut profits = vec![];
    let mut profit_ids = HashSet::<i32>::new();
    'recipes: for (_, r) in &recipes {
        let sale_total = if let Some(p) = prices.get(&r.output_item_id) {
            p.buys.unit_price * r.output_item_count
        } else { continue };
        let sale_total = sale_total - (0.15 * (sale_total as f32).ceil()) as i32;
        let mut craft_total = 0;
        for i in &r.ingredients {
            if let Some(p) = prices.get(&i.item_id) {
                craft_total += p.sells.unit_price * i.count;
            } else { continue 'recipes };
        }
        if sale_total > craft_total {
            profits.push(Profit {
                id: r.output_item_id,
                recipe: r.id,
                sale_total, craft_total,
            });
            profit_ids.insert(r.output_item_id);
            for i in &r.ingredients {
                profit_ids.insert(i.item_id);
            }
        }
    }
    profits.sort_by(|b, a|
        (a.sale_total - a.craft_total).cmp(&(b.sale_total - b.craft_total))
    );
    println!("profits: {}", profits.len());

    let iids_vec: Vec<i32> = profit_ids.iter().cloned().collect();
    let mut items = HashMap::<i32, Item>::new();
    for ids in iids_vec.chunks(50) {
        let is: Vec<Item> = client.fetch(false, &format!("items?ids={}", ids_str(ids)))?;
        for i in is {
            items.insert(i.id, i);
        }
    }

    println!("");
    for p in profits {
        let item = items.get(&p.id).unwrap();
        println!("{}: {}", item.name, p.sale_total - p.craft_total);
        let r = recipes.get(&p.recipe).unwrap();
        let output_price = prices.get(&p.id).unwrap();
        println!("\tSale: {} = {} @{}", p.sale_total, r.output_item_count, output_price.buys.unit_price);
        for i in &r.ingredients {
            let ii = items.get(&i.item_id).unwrap();
            let ip = prices.get(&i.item_id).unwrap();
            let mut vendor = "";
            if ip.vendor.is_some() {
                vendor = " [vendor]";
            }
            println!("\t{}: {} = {} @{}{}", ii.name, i.count * ip.sells.unit_price, i.count, ip.sells.unit_price, vendor);
        }
        println!("\tTotal: {}", p.craft_total);
    }

    Ok(())
}
