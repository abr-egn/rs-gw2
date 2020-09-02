use std::collections::{HashMap, HashSet, VecDeque};

#[macro_use]
mod error;

mod client;
mod index;

use crate::client::{Client, ItemId, Recipe, RecipeId};
use crate::error::Result;
use crate::index::Index;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Source {
    Vendor,
    Recipe(RecipeId),
    Auction,
    Unknown,
    Special,
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

const UNKNOWN_COST: i32 = 100000;

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

    let mut queue: VecDeque<ItemId> = index.items.keys().cloned().collect();
    'queue: while let Some(iid) = queue.pop_front() {
        if costs.contains_key(&iid) { continue }
        match index.recipes_by_item.get(&iid) {
            None => {
                let (source, value) = if let Some(price) = index.prices.get(&iid) {
                    (Source::Auction, price.sells.unit_price)
                } else {
                    (Source::Unknown, UNKNOWN_COST)
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

    let mut profits = vec![];
    let mut profit_ids = HashSet::new();
    for r in index.recipes.values() {
        let cost = if let Some(c) = costs.get(&r.output_item_id) { c } else { continue };
        let price = if let Some(p) = index.prices.get(&r.output_item_id) { p } else { continue };
        let sale = price.buys.unit_price * r.output_item_count;
        let sale = sale - (0.15 * (sale as f32).ceil()) as i32;
        if sale > cost.value {
            profits.push(Profit {
                id: r.id,
                value: sale - cost.value,
            });
            profit_ids.insert(r.output_item_id);
            for ing in &r.ingredients {
                profit_ids.insert(ing.item_id);
            }
        }
    }
    profits.sort_by(|b, a|a.value.cmp(&b.value));
    println!("profits: {}", profits.len());

    println!("");
    for p in profits {
        if p.value < 10000 { break }
        let recipe = index.recipes.get(&p.id).unwrap();
        let item = index.items.get(&recipe.output_item_id).unwrap();
        let cost = costs.get(&item.id).unwrap();
        println!("{}: {}", item.name, p.value);
        let output_price = index.prices.get(&item.id).unwrap();
        println!("\tSale: {} = {} @{}", recipe.output_item_count * output_price.buys.unit_price, recipe.output_item_count, output_price.buys.unit_price);
        print_ingredients(&index, &costs, &recipe, 1);
        println!("\tCost: {}", cost.value);
    }

    Ok(())
}

fn vendor(costs: &mut HashMap<ItemId, Cost>, id: i32, price: i32) {
    costs.insert(ItemId(id), Cost {
        id: ItemId(id),
        source: Source::Vendor,
        value: price,
    });
}

fn special(costs: &mut HashMap<ItemId, Cost>, id: i32, price: i32) {
    costs.insert(ItemId(id), Cost {
        id: ItemId(id),
        source: Source::Special,
        value: price,
    });
}

fn print_ingredients(index: &Index, costs: &HashMap<ItemId, Cost>, recipe: &Recipe, indent: usize) {
    for i in &recipe.ingredients {
        let ii = index.items.get(&i.item_id).expect(&format!("no item for {:?}", i.item_id));
        let ic = costs.get(&i.item_id).expect(&format!("no cost for {}", ii.name));
        let tabs: Vec<_> = std::iter::repeat("\t").take(indent).collect();
        let tabs = tabs.join("");
        println!("{}{}: {} = {} @{} [{:?}]", tabs, ii.name, i.count * ic.value, i.count, ic.value, ic.source);
        if let Source::Recipe(id) = ic.source {
            let r = index.recipes.get(&id).unwrap();
            print_ingredients(index, costs, r, indent+1);
        }
    }
}