use std::collections::{HashMap, HashSet};
use std::io::{Write, stdin};

#[macro_use]
mod error;

mod client;
mod cost;
mod index;

use crate::client::{Client, ItemId, RecipeId};
use crate::cost::{Cost, Source};
use crate::error::Result;
use crate::index::Index;

#[derive(Debug, Clone)]
struct Profit {
    id: RecipeId,
    days: i32,
    sale: i32,
    value: i32,
    daily: HashSet<ItemId>,
    cost: Cost,
}

impl Profit {
    fn per_day(&self) -> i32 {
        let d = std::cmp::max(1, self.days) as f32;
        ((self.value as f32) / d).floor() as i32
    }
}

const MIN_PROFIT: i32 = 10000;

fn main() -> Result<()> {
    let mut client = Client::new();
    let index = Index::new(&mut client, true)?;

    let mut profits = vec![];
    let mut profit_ids = HashSet::new();
    for r in index.recipes.values() {
        let item = if let Some(i) = index.items.get(&r.output_item_id) { i } else { continue };
        if item.description.as_ref().map_or(false, |d| d.contains("used to craft the legendary")) { continue }
        if item.name == "Guild Catapult" { continue }

        let cost = if let Ok(c) = Cost::new(&index, &r.output_item_id, 1) { c } else { continue };
        if let Source::Auction = cost.source { continue }
        let listings = if let Some(ls) = index.listings.get(&r.output_item_id) { ls } else { continue };
        let sale = if let Ok(s) = listings.sale(r.output_item_count) { s } else { continue };
        let sale = sale - ((0.15 * (sale as f32)).ceil()) as i32;

        let daily = days(&cost);
        let mut max_days = 0;
        for d in daily.values() {
            max_days = std::cmp::max(max_days, *d);
        }

        if sale > cost.total {
            profits.push(Profit {
                id: r.id,
                days: max_days,
                sale,
                value: sale - cost.total,
                daily: daily.keys().cloned().collect(),
                cost,
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
    print_profits_min(&index, &profits, MIN_PROFIT)?;

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
                    print_profit(&index, p)?;
                }
            }
        }
        if line.starts_with("min profit ") {
            let profit_str = line.strip_prefix("min profit ").unwrap();
            let profit = match profit_str.parse::<i32>() {
                Err(e) => { println!("{}", e); continue },
                Ok(n) => n,
            };
            print_profits_min(&index, &profits, profit)?;
        }
    }

    Ok(())
}

fn print_profits_min(index: &Index, profits: &Vec<Profit>, min: i32) -> Result<()> {
    let mut daily_used = HashSet::new();
    'profits: for p in profits {
        if p.per_day() < min { break }
        let recipe = index.recipes.get(&p.id).unwrap();
        let item = index.items.get(&recipe.output_item_id).unwrap();
        if p.days > 1 {
            println!("(skip: {} : {} [{} days])\n", item.name, money(p.per_day()), p.days);
            continue
        }
        for d in &p.daily {
            if !daily_used.insert(d) {
                let used = index.items.get(d).unwrap();
                println!("(skip: {} : {} [{}])\n", item.name, money(p.per_day()), used.name);
                continue 'profits
            }
        }
        print_profit(&index, &p)?;
        println!("");
    }
    Ok(())
}

fn print_profit(index: &Index, p: &Profit) -> Result<()> {
    let recipe = index.recipes.get(&p.id).unwrap();
    let item = index.items.get(&recipe.output_item_id).unwrap();
    let cost = &p.cost;
    println!("{} : {} ({} over {} days)", item.name, money(p.per_day()), money(p.value), p.days);
    let output_price = index.listings.get(&item.id).unwrap().sale(1).unwrap();
    println!("\tSale: {} = {} @ {}", money(p.sale), recipe.output_item_count, money(output_price));
    println!("\tCost: {}", money(cost.total));
    print_cost(&index, &cost, 1);
    let mut materials = index.materials.clone();
    let ingredients = shopping_ingredients(&index, &cost, &mut materials);
    let mut shop_cost = 0;
    println!("\tShopping:");
    for (id, count) in &ingredients {
        let cost = Cost::new(&index, id, *count)?;
        let item = index.items.get(id).unwrap();
        println!("\t\t{} : {} = {}", item.name, count, money(cost.total));
        shop_cost += cost.total;
    }
    println!("\tTotal: {}", money(shop_cost));
    Ok(())
}

fn print_cost(index: &Index, cost: &Cost, indent: usize) {
    let ii = index.items.get(&cost.id).unwrap();
    let tabs: Vec<_> = std::iter::repeat("\t").take(indent).collect();
    let tabs = tabs.join("");
    println!("{}{} : {} = {}{}", tabs, ii.name, cost.quantity, money(cost.total), cost.source.to_str());
    if let Source::Recipe { ingredients, .. } = &cost.source {
        for ing in ingredients.values() {
            print_cost(index, ing, indent+1);
        }
    }
}

fn shopping_ingredients(index: &Index, cost: &Cost, materials: &mut HashMap<ItemId, i32>) -> HashMap<ItemId, i32> {
    let mut out = HashMap::new();
    let has = materials.get(&cost.id).cloned().unwrap_or(0);
    let used = std::cmp::min(cost.quantity, has);
    if used > 0 {
        *materials.get_mut(&cost.id).unwrap() -= used;
    }
    let needed = cost.quantity - used;
    if needed <= 0 { return HashMap::new(); }
    if let Source::Recipe { ingredients, .. } = &cost.source {
        for ing in ingredients.values() {
            for (id, count) in shopping_ingredients(index, ing, materials) {
                *out.entry(id).or_insert(0) += count;
            }
        }
    } else {
        out.insert(cost.id, needed);
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

fn days(cost: &Cost) -> HashMap<ItemId, i32> {
    if is_daily(&cost.id) {
        let mut out = HashMap::new();
        out.insert(cost.id, cost.quantity);
        return out;
    }
    let ingredients = if let Source::Recipe { ingredients, .. } = &cost.source { ingredients } else { return HashMap::new() };
    let mut out = HashMap::new();
    for ing in ingredients.values() {
        for (id, count) in days(ing) {
            *out.entry(id).or_insert(0) += count;
        }
    }
    out
}