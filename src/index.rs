use std::collections::{HashMap, HashSet};
use std::io::Write;

use crate::client::{CharacterRecipes, Client, Item, ItemId, Price, Recipe, RecipeId};
use crate::error::Result;

pub struct Index {
    pub recipes: HashMap<RecipeId, Recipe>,
    pub recipes_by_item: HashMap<ItemId, Recipe>,
    pub items: HashMap<ItemId, Item>,
    pub prices: HashMap<ItemId, Price>,
    pub materials: HashMap<ItemId, i32>,
}

impl Index {
    pub fn new(client: &mut Client, by_character: bool) -> Result<Index> {
        let all_ids;
        if by_character {
            let names: Vec<String> = client.characters()?;
            println!("{:?}", names);
            let mut id_set = HashSet::<RecipeId>::new();
            for name in &names {
                let r: CharacterRecipes = client.character_recipes(name)?;
                println!("{}: {}", name, r.recipes.len());
                for id in &r.recipes {
                    id_set.insert(*id);
                }
            }
            all_ids = id_set.iter().cloned().collect();
        } else {
            all_ids = client.all_recipes()?;
        }
        println!("known recipes: {}", all_ids.len());

        let mut recipes = HashMap::new();
        let mut recipes_by_item = HashMap::new();
        for ids in all_ids.chunks(50) {
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
 
        let mut items = HashMap::new();
        let id_vec: Vec<_> = all_items.iter().cloned().collect();
        for ids in id_vec.chunks(50) {
            let is = client.items(&ids)?;
            for i in is {
                items.insert(i.id, i);
            }
            print!(".");
            std::io::stdout().flush()?;
        }
        println!("");
        println!("retrieved items: {}", items.len());
        
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

        let mut materials = HashMap::new();
        let ms = client.materials()?;
        println!("materials: {}", ms.len());
        for m in ms {
            materials.insert(m.id, m.count);
        }

        Ok(Index{recipes, recipes_by_item, items, prices, materials})
    }
}