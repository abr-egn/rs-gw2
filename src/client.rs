use std::time::{Duration, Instant};

use reqwest::{self, StatusCode};
use serde::Deserialize;
use serde::de::{DeserializeOwned};

use crate::error::{Result};

pub struct Client {
    reqw: reqwest::blocking::Client,
    last: Instant,
}

static KEY: &str = "9E827119-71EE-774C-88AE-B4D1D4B30868205CD293-9A69-415C-A3E3-4CB1E184722C";

impl Client {
    pub fn new() -> Self {
        Client {
            reqw: reqwest::blocking::Client::new(),
            last: Instant::now(),
        }
    }

    pub fn characters(&mut self) -> Result<Vec<String>> {
        self.fetch(true, "characters")
    }

    pub fn character_recipes(&mut self, name: &str) -> Result<CharacterRecipes> {
        self.fetch(true, &format!("characters/{}/recipes", name))
    }

    pub fn recipes(&mut self, ids: &[RecipeId]) -> Result<Vec<Recipe>> {
        self.fetch(false, &format!("recipes?ids={}", ids_str(ids)))
    }

    pub fn prices(&mut self, ids: &[ItemId]) -> Result<Vec<Price>> {
        self.fetch(false, &format!("commerce/prices?ids={}", ids_str(ids)))
    }

    pub fn items(&mut self, ids: &[ItemId]) -> Result<Vec<Item>> {
        self.fetch(false, &format!("items?ids={}", ids_str(ids)))
    }

    pub fn materials(&mut self) -> Result<Vec<Material>> {
        self.fetch(true, "account/materials")
    }

    pub fn all_recipes(&mut self) -> Result<Vec<RecipeId>> {
        self.fetch(false, "recipes")
    }

    pub fn listings(&mut self, ids: &[ItemId]) -> Result<Vec<Listings>> {
        let mut out: Vec<Listings> = self.fetch(false, &format!("commerce/listings?ids={}", ids_str(ids)))?;
        for ls in &mut out {
            ls.buys.sort_by(|a, b| b.unit_price.cmp(&a.unit_price));
            ls.sells.sort_by(|a, b| a.unit_price.cmp(&b.unit_price));
        }
        Ok(out)
    }

    fn fetch<Out>(
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

#[derive(Debug, Clone, Deserialize)]
pub struct CharacterRecipes {
    pub recipes: Vec<RecipeId>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ingredient {
    pub item_id: ItemId,
    pub count: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Recipe {
    #[serde(rename = "type")]
    pub typ: String,
    pub output_item_id: ItemId,
    pub output_item_count: i32,
    pub min_rating: i32,
    pub time_to_craft_ms: i32,
    pub disciplines: Vec<String>,
    pub flags: Vec<String>,
    pub ingredients: Vec<Ingredient>,
    pub id: RecipeId,
    pub chat_link: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Price {
    pub id: ItemId,
    pub whitelisted: bool,
    pub buys: Order,
    pub sells: Order,
    pub vendor: Option<()>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Order {
    pub quantity: i32,
    pub unit_price: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub typ: String,
    pub level: i32,
    pub rarity: String,
    pub vendor_value: i32,
    pub game_types: Vec<String>,
    pub flags: Vec<String>,
    pub restrictions: Vec<String>,
    pub id: ItemId,
    pub chat_link: String,
    pub icon: String,
}

#[repr(transparent)]
#[serde(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct RecipeId(pub i32);

#[repr(transparent)]
#[serde(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct ItemId(pub i32);

trait AsId {
    fn as_id(&self) -> i32;
}

impl AsId for ItemId {
    fn as_id(&self) -> i32 { self.0 }
}

impl AsId for RecipeId {
    fn as_id(&self) -> i32 { self.0 }
}

fn ids_str<T: AsId>(ids: &[T]) -> String {
    let id_strs: Vec<String> = ids.iter().map(|id| format!("{}", id.as_id())).collect();
    id_strs.join(",")
}

#[derive(Debug, Clone, Deserialize)]
pub struct Material {
    pub id: ItemId,
    pub category: i32,
    pub binding: Option<String>,
    pub count: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Listings {
    pub id: ItemId,
    pub buys: Vec<Listing>,
    pub sells: Vec<Listing>,
}

impl Listings {
    pub fn cost(&self, quantity: i32) -> Result<i32> {
        let mut remaining = quantity;
        let mut cost = 0;
        for l in &self.sells {
            let bought = std::cmp::min(remaining, l.quantity);
            cost += bought * l.unit_price;
            remaining -= bought;
            if remaining == 0 {
                return Ok(cost)
            }
        }
        failed!("cost short {} of {}", remaining, self.id.0)
    }
    pub fn sale(&self, quantity: i32) -> Result<i32> {
        let mut remaining = quantity;
        let mut sale = 0;
        for l in &self.buys {
            let sold = std::cmp::min(remaining, l.quantity);
            sale += sold * l.unit_price;
            remaining -= sold;
            if remaining == 0 {
                return Ok(sale)
            }
        }
        failed!("sale short {} of {}", remaining, self.id.0)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Listing {
    pub listings: i32,
    pub unit_price: i32,
    pub quantity: i32,
}