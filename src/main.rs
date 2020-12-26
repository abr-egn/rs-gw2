use std::collections::{HashMap, HashSet};
use std::io::{Write, stdin};

#[macro_use]
mod error;

mod client;
mod cost;
mod index;

use crate::client::{Client, ItemId, Recipe, RecipeId};
use crate::cost::{Cost, Source};
use crate::error::Result;
use crate::index::{Index, RecipeSource};

#[derive(Debug, Clone)]
struct Profit {
    id: RecipeId,
    days: i32,
    sale: i32,
    value: i32,
    daily: HashSet<ItemId>,
    cost: Cost,
    mats_profit: Option<i32>,
}

impl Profit {
    fn per_day(&self) -> i32 {
        let d = std::cmp::max(1, self.days) as f32;
        ((self.value as f32) / d).floor() as i32
    }
}

const MIN_PROFIT: i32 = 5000;

fn main() -> Result<()> {
    let mut client = Client::new();
    let index = Index::new(&mut client, RecipeSource::Characters)?;

    let (flip_profits, bank_profits) = find_profits(&index);
    println!("flip profits: {}", flip_profits.len());
    println!("bank profits: {}", bank_profits.len());

    println!("");
    println!("=== Flip Profits ===");
    println!("");
    print_profits_min(&index, &flip_profits, MIN_PROFIT)?;
    println!("=== Bank Profits ===");
    println!("");
    print_profits_min(&index, &bank_profits, MIN_PROFIT)?;

    if false {
        command_loop(&index, &flip_profits)?;
    }

    Ok(())
}

fn find_profits(index: &Index) -> (Vec<Profit>, Vec<Profit>) {
    let mut flip_profits = vec![];
    let mut bank_profits = vec![];
    for r in index.recipes.values() {
        let item = if let Some(i) = index.items.get(&r.output_item_id) { i } else { continue };
        if item.description.as_ref().map_or(false, |d| d.contains("used to craft the legendary")) { continue }
        if item.name == "Guild Catapult" { continue }

        let sale = {
            let listings = if let Some(ls) = index.listings.get(&r.output_item_id) { ls } else { continue };
            let gross_sale = if let Ok(s) = listings.sale(r.output_item_count) { s } else { continue };
            gross_sale - ((0.15 * (gross_sale as f32)).ceil()) as i32
        };

        if let Some(p) = flip_profit(index, r, sale) { flip_profits.push(p); }
        if let Some(p) = bank_profit(index, r, sale) { bank_profits.push(p); }
    }
    flip_profits.sort_by(|b, a| { a.per_day().cmp(&b.per_day()) });
    bank_profits.sort_by(|b, a| { a.per_day().cmp(&b.per_day()) });
    (flip_profits, bank_profits)
}

fn flip_profit(index: &Index, r: &Recipe, sale: i32) -> Option<Profit> {
    let cost = if let Ok(c) = Cost::new(&index, &r.output_item_id, 1) { c } else { return None };
    if let Source::Auction = cost.source { return None }
    let daily = days(&cost);
    let mut days = 0;
    for d in daily.values() {
        days = std::cmp::max(days, *d);
    }
    
    if sale > cost.total {
        return Some(Profit {
            id: r.id,
            days,
            sale,
            value: sale - cost.total,
            daily: daily.keys().cloned().collect(),
            cost,
            mats_profit: None,
        });
    }
    None
}

fn bank_profit(index: &Index, r: &Recipe, sale: i32) -> Option<Profit> {
    let mut bank = index.materials.clone();
    let cost = if let Ok(c) = Cost::new_with_bank(&index, &r.output_item_id, 1, &mut bank) { c } else { return None };
    if let Source::Auction = cost.source { return None }
    let daily = days(&cost);
    let mut days = 0;
    for d in daily.values() {
        days = std::cmp::max(days, *d);
    }

    let used = bank_used(&cost);
    let mut used_profit = 0;
    for (id, count) in &used {
        if let Some(s) = index.listings.get(id).and_then(|l| l.sale(*count).ok()) {
            used_profit += s;
        }/* else if let Some(sc) = cost::special(index, id) {
            used_profit += sc*count;
        }*/
    }
    if sale > cost.total + used_profit {
        return Some(Profit {
            id: r.id,
            days,
            sale,
            value: sale - (cost.total + used_profit),
            daily: daily.keys().cloned().collect(),
            cost,
            mats_profit: Some(used_profit),
        });
    }
    None
}

fn bank_used(c: &Cost) -> HashMap<ItemId, i32> {
    let mut out = HashMap::new();
    bank_used_aux(&c.id, &c.source, &mut out);
    out
}

fn bank_used_aux(id: &ItemId, s: &Source, out: &mut HashMap<ItemId, i32>) {
    match s {
        Source::Bank { used, rest } => {
            *out.entry(*id).or_insert(0) += used;
            if let Some(r) = rest {
                bank_used_aux(id, r, out);
            }
        },
        Source::Recipe { ingredients, .. } => {
            for (id, c) in ingredients {
                bank_used_aux(id, &c.source, out);
            }
        },
        _ => ()
    }
}

fn print_profits_min(index: &Index, profits: &[Profit], min: i32) -> Result<()> {
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
    if let Some(mp) = p.mats_profit {
        println!("\tMats: {}", money(mp));
    }
    print_cost(&index, &cost, 1);
    let ingredients = shopping_ingredients(&index, &cost);
    let mut shop_cost = 0;
    println!("\tShopping:");
    for (id, count) in &ingredients {
        let cost = Cost::new(&index, id, *count)?;
        let item = index.items.get(id).unwrap();
        println!("\t\t{} : {} = {}{}", item.name, count, money(cost.total), cost.source.to_str());
        shop_cost += cost.total;
    }
    println!("\tTotal: {}", money(shop_cost));
    Ok(())
}

fn print_cost(index: &Index, cost: &Cost, indent: usize) {
    let ii = index.items.get(&cost.id).unwrap();
    let tabs: Vec<_> = std::iter::repeat("\t").take(indent).collect();
    let tabs = tabs.join("");
    let (quantity, total) = if let Source::Bank { used, .. } = cost.source {
        (used, 0)
    } else {
        (cost.quantity, cost.total)
    };
    println!("{}{} : {} = {}{}", tabs, ii.name, quantity, money(total), cost.source.to_str());
    match &cost.source {
        Source::Recipe { ingredients, .. } => {
            for ing in ingredients.values() {
                print_cost(index, ing, indent+1);
            }
        }
        Source::Bank { used, rest: Some(r) } => {
            let subcost = Cost { source: (**r).clone(), quantity: cost.quantity - used, ..*cost };
            print_cost(index, &subcost, indent);
        }
        _ => ()
    }
}

fn shopping_ingredients(index: &Index, cost: &Cost) -> HashMap<ItemId, i32> {
    let mut out = HashMap::new();
    for (id, count) in cost.base_ingredients() {
        let has = index.materials.get(&id).cloned().unwrap_or(0);
        if has < count {
            out.insert(id, count - has);
        }
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

fn command_loop(index: &Index, profits: &[Profit]) -> Result<()> {
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
            for p in profits {
                let r = index.recipes.get(&p.id).unwrap();
                if r.output_item_id == id {
                    print_profit(&index, p)?;
                }
            }
        }
        if line.starts_with("cost ") {
            let parts: Vec<_> = line.strip_prefix("cost ").unwrap().split(' ').collect();
            let id = match parts[0].parse::<i32>() {
                Err(e) => { println!("{}", e); continue },
                Ok(id) => ItemId(id),
            };
            let count = if parts.len() == 2 {
                match parts[1].parse::<i32>() {
                    Err(e) => { println!("{}", e); continue },
                    Ok(c) => c,
                }
            } else { 1 };
            let cost = match Cost::new(&index, &id, count) {
                Err(e) => { println!("{}", e); continue },
                Ok(c) => c,
            };
            print_cost(&index, &cost, 0);
        }
        if line.starts_with("min profit ") {
            let profit_str = line.strip_prefix("min profit ").unwrap();
            let profit = match profit_str.parse::<i32>() {
                Err(e) => { println!("{}", e); continue },
                Ok(n) => n,
            };
            print_profits_min(index, profits, profit)?;
        }
    }
    Ok(())
}