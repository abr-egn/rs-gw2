use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{Write, stdin};

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
    days: i32,
    sale: i32,
    value: i32,
}

impl Profit {
    fn per_day(&self) -> i32 {
        let d = std::cmp::max(1, self.days) as f32;
        ((self.value as f32) / d).floor() as i32
    }
}

const UNKNOWN_COST: i32 = 0;
const MIN_PROFIT: i32 = 5000;

type Costs = HashMap<ItemId, Cost>;

fn main() -> Result<()> {
    let mut client = Client::new();
    let index = Index::new(&mut client)?;

    let mut costs: Costs = HashMap::new();
    
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
    // Milling Basin
    vendor(&mut costs, 76839, 56);

    // Obsidian Shard
    // 5 for 1 Guild Commendation daily at the Guild Trader
    // Guild Commendation ~= 50s
    special(&mut costs, 19925, 1000);
    // Charged Quartz Crystal
    // 25 Quartz Crystals at a place of power daily
    special(&mut costs, 43772, 25 * index.prices.get(&ItemId(43773)).unwrap().sells.unit_price);

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
        if cost.source == Source::Auction { continue }
        let price = if let Some(p) = index.prices.get(&r.output_item_id) { p } else { continue };
        let sale = price.buys.unit_price * r.output_item_count;
        let sale = sale - ((0.15 * (sale as f32)).ceil()) as i32;
        let mut max_days = 0;
        for d in days(&index, &costs, &r.output_item_id).values() {
            max_days = std::cmp::max(max_days, *d);
        }
        if sale > cost.value {
            profits.push(Profit {
                id: r.id,
                days: max_days,
                sale,
                value: sale - cost.value,
            });
            profit_ids.insert(r.output_item_id);
            for ing in &r.ingredients {
                profit_ids.insert(ing.item_id);
            }
        }
    }
    profits.sort_by(|b, a| { a.per_day().cmp(&b.per_day()) });
    println!("profits: {}", profits.len());

    println!("");
    for p in &profits {
        if p.value < MIN_PROFIT { break }
        print_profit(&index, &costs, &p);
    }

    let mut line = String::new();
    loop {
        print!("> ");
        std::io::stdout().flush()?;
        line.clear();
        stdin().read_line(&mut line)?;
        let line = line.trim();
        if line == "exit" { break }
        if line.starts_with("profit ") {
            let id_str = line.strip_prefix("profit ").unwrap();
            let id = match id_str.parse::<i32>() {
                Err(e) => { println!("{}", e); continue },
                Ok(id) => ItemId(id),
            };
            for p in &profits {
                let r = index.recipes.get(&p.id).unwrap();
                if r.output_item_id == id {
                    print_profit(&index, &costs, p);
                }
            }
        }
        if line.starts_with("cost ") {
            let id_str = line.strip_prefix("cost ").unwrap();
            let id = match id_str.parse::<i32>() {
                Err(e) => { println!("{}", e); continue },
                Ok(id) => ItemId(id),
            };
            if !costs.contains_key(&id) {
                println!("no cost entry");
                continue
            }
            print_cost(&index, &costs, &id, 0, 1);
        }
    }

    Ok(())
}

fn print_profit(index: &Index, costs: &Costs, p: &Profit) {
    let recipe = index.recipes.get(&p.id).unwrap();
    let item = index.items.get(&recipe.output_item_id).unwrap();
    let cost = costs.get(&item.id).unwrap();
    println!("{} : {} ({} over {} days)", item.name, money(p.per_day()), money(p.value), p.days);
    let output_price = index.prices.get(&item.id).unwrap();
    println!("\tSale: {} = {} @ {}", money(p.sale), recipe.output_item_count, money(output_price.buys.unit_price));
    print_costs(&index, &costs, &recipe, 1, 1);
    println!("\tCost: {}", money(cost.value));
    let mut materials = index.materials.clone();
    let ingredients = all_ingredients(&index, &costs, &mut materials, &recipe.output_item_id, 1);
    let mut shop_cost = 0;
    for (id, count) in &ingredients {
        let c = costs.get(id).unwrap();
        shop_cost += count * c.value;
    }
    println!("\tShopping: {}", money(shop_cost));
    for (id, count) in &ingredients {
        let item = index.items.get(id).unwrap();
        let c = costs.get(id).unwrap();
        println!("\t\t{} : {} @ {} = {}", item.name, count, money(c.value), money(count * c.value));
    }
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

fn print_costs(index: &Index, costs: &Costs, recipe: &Recipe, indent: usize, count: i32) {
    for i in &recipe.ingredients {
        let total = i.count * count;
        print_cost(index, costs, &i.item_id, indent, total);
    }
}

fn print_cost(index: &Index, costs: &Costs, id: &ItemId, indent: usize, count: i32) {
    let ic = costs.get(id).unwrap();
    let ii = index.items.get(id).unwrap();
    let source = match ic.source {
        Source::Unknown => " [UNKNOWN]",
        Source::Special => " [SPECIAL]",
        _ => "",
    };
    let tabs: Vec<_> = std::iter::repeat("\t").take(indent).collect();
    let tabs = tabs.join("");
    println!("{}{} : {} @ {} = {}{}", tabs, ii.name, count, money(ic.value), money(count * ic.value), source);
    if let Source::Recipe(id) = ic.source {
        let r = index.recipes.get(&id).unwrap();
        print_costs(index, costs, r, indent+1, count);
    }
}

fn all_ingredients(index: &Index, costs: &HashMap<ItemId, Cost>, materials: &mut HashMap<ItemId, i32>, id: &ItemId, count: i32) -> HashMap<ItemId, i32> {
    let mut out = HashMap::new();
    let has = materials.get(id).cloned().unwrap_or(0);
    let used = std::cmp::min(count, has);
    if used > 0 {
        *materials.get_mut(id).unwrap() -= used;
    }
    let needed = count - used;
    if needed <= 0 { return HashMap::new(); }
    if let Source::Recipe(rid) = costs.get(id).unwrap().source {
        let recipe = index.recipes.get(&rid).unwrap();
        for ing in &recipe.ingredients {
            for (id, count) in all_ingredients(index, costs, materials, &ing.item_id, needed * ing.count) {
                *out.entry(id).or_insert(0) += count;
            }
        }
    } else {
        out.insert(*id, needed);
    }
    out
}

fn money(amount: i32) -> String {
    let mut out = String::new();
    if amount >= 10000 {
        out.push_str(&format!("{}g ", amount / 10000));
    }
    if amount >= 100 {
        out.push_str(&format!("{}s ", (amount / 100) % 100));
    }
    out.push_str(&format!("{}c", amount % 100));
    out
}

fn is_daily(id: &ItemId) -> bool {
    match id.0 {
        // Charged Quartz Crystal
        43772 => true,
        // Glob of Elder Spirit Residue
        46744 => true,
        // Lump of Mithrillium
        46742 => true,
        // Spool of Silk Weaving Thread
        46740 => true,
        // Spool of Thick Elonian Cord
        46745 => true,
        _ => false,
    }
}

fn days(index: &Index, costs: &Costs, id: &ItemId) -> HashMap<ItemId, i32> {
    if is_daily(id) {
        let mut out = HashMap::new();
        out.insert(*id, 1);
        return out;
    }
    let cost = costs.get(id).unwrap();
    let rid = if let Source::Recipe(rid) = cost.source { rid } else { return HashMap::new() };
    let recipe = index.recipes.get(&rid).unwrap();
    let mut out = HashMap::new();
    for ing in &recipe.ingredients {
        for (id, count) in days(index, costs, &ing.item_id) {
            *out.entry(id).or_insert(0) += count * ing.count;
        }
    }
    out
}