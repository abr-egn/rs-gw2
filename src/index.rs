use std::collections::{HashMap, HashSet};
use std::io::Write;

use serde::{Deserialize};

use crate::client::Client;
use crate::error::Result;

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

pub struct Index {
    pub recipes: HashMap<RecipeId, Recipe>,
    pub prices: HashMap<ItemId, Price>,
}

impl Index {
    pub fn new(client: &mut Client) -> Result<Index> {
        let names: Vec<String> = client.fetch(true, "characters")?;
        println!("{:?}", names);
        let mut all_ids = HashSet::<RecipeId>::new();
        for name in &names {
            let r: CharacterRecipes = client.fetch(true, &format!("characters/{}/recipes", name))?;
            println!("{}: {}", name, r.recipes.len());
            for id in &r.recipes {
                all_ids.insert(*id);
            }
        }
        println!("known recipes: {}", all_ids.len());

        let mut recipes = HashMap::new();
        let id_vec: Vec<RecipeId> = all_ids.iter().cloned().collect();
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

        let mut all_items = HashSet::<ItemId>::new();
        for (_, r) in &recipes {
            all_items.insert(r.output_item_id);
            for i in &r.ingredients {
                all_items.insert(i.item_id);
            }
        }
        println!("total items: {}", all_items.len());
    
        let mut prices = HashMap::<ItemId, Price>::new();
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
    
        let pid_vec: Vec<ItemId> = all_items.iter().cloned().collect();
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

        Ok(Index{recipes, prices})
    }
}

fn vendor(prices: &mut HashMap<ItemId, Price>, id: i32, price: i32) {
    prices.insert(ItemId(id), Price {
        id: ItemId(id),
        whitelisted: true,
        buys: Order { quantity: 0, unit_price: 0 },
        sells: Order { quantity: 1, unit_price: price },
        vendor: Some(()),
    });
}

pub trait AsId {
    fn as_id(&self) -> i32;
}

impl AsId for ItemId {
    fn as_id(&self) -> i32 { self.0 }
}

impl AsId for RecipeId {
    fn as_id(&self) -> i32 { self.0 }
}

pub fn ids_str<T: AsId>(ids: &[T]) -> String {
    let id_strs: Vec<String> = ids.iter().map(|id| format!("{}", id.as_id())).collect();
    id_strs.join(",")
}