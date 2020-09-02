use std::collections::{HashMap, HashSet};

#[macro_use]
mod error;

mod client;
mod index;

use crate::client::{Client, Item, ItemId, RecipeId};
use crate::error::Result;
use crate::index::Index;

#[derive(Debug, Clone)]
struct Profit {
    id: ItemId,
    recipe: RecipeId,
    sale_total: i32,
    craft_total: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Source {
    Auction,
    Vendor,
    Unknown,
}

fn main() -> Result<()> {
    let mut client = Client::new();
    let index = Index::new(&mut client)?;

    let mut profits = vec![];
    let mut profit_ids = HashSet::<ItemId>::new();
    'recipes: for (_, r) in &index.recipes {
        let sale_total = if let Some(p) = index.prices.get(&r.output_item_id) {
            p.buys.unit_price * r.output_item_count
        } else { continue };
        let sale_total = sale_total - (0.15 * (sale_total as f32).ceil()) as i32;
        let mut craft_total = 0;
        for i in &r.ingredients {
            if let Some(p) = index.prices.get(&i.item_id) {
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

    let iids_vec: Vec<ItemId> = profit_ids.iter().cloned().collect();
    let mut items = HashMap::<ItemId, Item>::new();
    for ids in iids_vec.chunks(50) {
        let is: Vec<Item> = client.items(ids)?;
        for i in is {
            items.insert(i.id, i);
        }
    }

    println!("");
    for p in profits {
        let item = items.get(&p.id).unwrap();
        println!("{}: {}", item.name, p.sale_total - p.craft_total);
        let r = index.recipes.get(&p.recipe).unwrap();
        let output_price = index.prices.get(&p.id).unwrap();
        println!("\tSale: {} = {} @{}", p.sale_total, r.output_item_count, output_price.buys.unit_price);
        for i in &r.ingredients {
            let ii = items.get(&i.item_id).unwrap();
            let ip = index.prices.get(&i.item_id).unwrap();
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
