use std::collections::{HashMap, HashSet};
use std::io::Write;

use crate::client::{CharacterRecipes, Client, ItemId, Price, Recipe, RecipeId};
use crate::error::Result;

pub struct Index {
    pub recipes: HashMap<RecipeId, Recipe>,
    pub recipes_by_item: HashMap<ItemId, Recipe>,
    pub all_items: HashSet<ItemId>,
    pub prices: HashMap<ItemId, Price>,
}

impl Index {
    pub fn new(client: &mut Client) -> Result<Index> {
        let names: Vec<String> = client.characters()?;
        println!("{:?}", names);
        let mut all_ids = HashSet::<RecipeId>::new();
        for name in &names {
            let r: CharacterRecipes = client.character_recipes(name)?;
            println!("{}: {}", name, r.recipes.len());
            for id in &r.recipes {
                all_ids.insert(*id);
            }
        }
        println!("known recipes: {}", all_ids.len());

        let mut recipes = HashMap::new();
        let mut recipes_by_item = HashMap::new();
        let id_vec: Vec<RecipeId> = all_ids.iter().cloned().collect();
        for ids in id_vec.chunks(50) {
            let rs: Vec<Recipe> = client.recipes(ids)?;
            for r in rs {
                recipes.insert(r.id, r.clone());
                recipes_by_item.insert(r.output_item_id, r);
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
    
        let pid_vec: Vec<ItemId> = all_items.iter().cloned().collect();
        for ids in pid_vec.chunks(50) {
            let ps: Vec<Price> = client.prices(ids)?;
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

        Ok(Index{recipes, recipes_by_item, all_items, prices})
    }
}