use std::collections::{HashMap, HashSet, VecDeque};

#[macro_use]
mod error;

mod client;
mod index;

use crate::client::{Client, ItemId, RecipeId};
use crate::error::Result;
use crate::index::Index;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Source {
    Vendor,
    Recipe(RecipeId),
    Auction,
    AccountBound,
    Unknown,
}

#[derive(Debug, Clone)]
struct Cost {
    id: ItemId,
    source: Source,
    value: i32,
}

#[derive(Debug, Clone)]
struct Profit {
    id: RecipeId,
    value: i32,
}

fn main() -> Result<()> {
    let mut client = Client::new();
    let index = Index::new(&mut client)?;

    let mut costs: HashMap<ItemId, Cost> = HashMap::new();
    // Thermocatalytic Reagent
    vendor(&mut costs, 46747, 150);
    // Spool of Gossamer Thread
    vendor(&mut costs, 19790, 64);
    // Spool of Silk Thread
    vendor(&mut costs, 19791, 48);
    // Spool of Linen Thread
    vendor(&mut costs, 19793, 32);
    // Spool of Cotton Thread
    vendor(&mut costs, 19794, 24);
    // Spool of Wool Thread
    vendor(&mut costs, 19789, 16);
    // Spool of Jute Thread
    vendor(&mut costs, 19792, 8);

    let mut queue: VecDeque<ItemId> = index.all_items.iter().cloned().collect();
    'queue: while let Some(iid) = queue.pop_front() {
        if costs.contains_key(&iid) { continue }
        match index.recipes_by_item.get(&iid) {
            None => {
                let (source, value) = if let Some(price) = index.prices.get(&iid) {
                    (Source::Auction, price.sells.unit_price)
                } else {
                    let items = client.items(&[iid])?;
                    let item = &items[0];
                    if item.flags.iter().any(|f| f == "AccountBound") {
                        (Source::AccountBound, 0)
                    } else {
                        (Source::Unknown, 0)
                    }
                };
                costs.insert(iid, Cost { id: iid, source, value });
            }
            Some(recipe) => {
                let mut craft_total = 0;
                for ing in &recipe.ingredients {
                    match costs.get(&ing.item_id) {
                        None => {
                            queue.push_back(iid);
                            continue 'queue;
                        }
                        Some(cost) => {
                            craft_total += cost.value * ing.count;
                        }
                    }
                }
                let (source, value) = if let Some(price) = index.prices.get(&iid) {
                    if price.sells.unit_price < craft_total {
                        (Source::Auction, price.sells.unit_price)
                    } else {
                        (Source::Recipe(recipe.id), craft_total)
                    }
                } else {
                    (Source::Recipe(recipe.id), craft_total)
                };
                costs.insert(iid, Cost { id: iid, source, value });
            }
        }
    }
    println!("costs: {}", costs.len());
    let mut unknown = vec![];
    for (id, cost) in &costs {
        if cost.source == Source::Unknown {
            unknown.push(*id);
        }
    }
    println!("unknown: {}", unknown.len());

    if !unknown.is_empty() {
        let items = client.items(&unknown)?;
        for item in items {
            println!("\t{}", item.name);
        }
    }

    let mut profits = vec![];
    for r in index.recipes.values() {
        let cost = if let Some(c) = costs.get(&r.output_item_id) { c } else { continue };
        let price = if let Some(p) = index.prices.get(&r.output_item_id) { p } else { continue };
        let sale = price.buys.unit_price * r.output_item_count;
        let sale = sale - (0.15 * (sale as f32).ceil()) as i32;
        if sale > cost.value {
            profits.push(Profit {
                id: r.id,
                value: sale - cost.value,
            })
        }
    }
    profits.sort_by(|b, a|a.value.cmp(&b.value));
    println!("profits: {}", profits.len());

    /*
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
        let output_price = index.costs.get(&p.id).unwrap();
        println!("\tSale: {} = {} @{}", p.sale_total, r.output_item_count, output_price.buys.unit_price);
        for i in &r.ingredients {
            let ii = items.get(&i.item_id).unwrap();
            let ip = index.costs.get(&i.item_id).unwrap();
            let mut vendor = "";
            if ip.vendor.is_some() {
                vendor = " [vendor]";
            }
            println!("\t{}: {} = {} @{}{}", ii.name, i.count * ip.sells.unit_price, i.count, ip.sells.unit_price, vendor);
        }
        println!("\tTotal: {}", p.craft_total);
    }
    */

    Ok(())
}

fn vendor(costs: &mut HashMap<ItemId, Cost>, id: i32, price: i32) {
    costs.insert(ItemId(id), Cost {
        id: ItemId(id),
        source: Source::Vendor,
        value: price,
    });
}