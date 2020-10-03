use std::collections::HashMap;

use crate::client::{ItemId, RecipeId};
use crate::error::Result;
use crate::index::Index;

#[derive(Debug, Clone)]
pub enum Source {
    Vendor,
    Recipe {
        id: RecipeId,
        ingredients: HashMap<ItemId, Cost>,
    },
    Auction,
    Unknown,
    Special,
    Bank {
        used: i32,
        rest: Option<Box<Source>>,
    },
}

impl Source {
    pub fn to_str(&self) -> String {
        match *self {
            Source::Unknown => " [UNKNOWN]".into(),
            Source::Special => " [SPECIAL]".into(),
            Source::Vendor => " [VENDOR]".into(),
            Source::Bank { used, rest: Some(ref rest) } => {
                format!(" [{} BANK +{}]", used, rest.to_str())
            },
            _ => "".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Cost {
    pub id: ItemId,
    pub source: Source,
    pub quantity: i32,
    pub total: i32,
}

fn base_ingredients_aux(id: &ItemId, source: &Source, quantity: i32) -> HashMap<ItemId, i32> {
    let mut out = HashMap::new();
    match source {
        Source::Recipe { ingredients, .. } => {
            for ing in ingredients.values() {
                for (id, count) in ing.base_ingredients() {
                    *out.entry(id).or_insert(0) += count;
                }
            }
        }
        Source::Bank { used, rest: Some(r) } => {
            out.insert(*id, *used);
            for (id, count) in base_ingredients_aux(id, r, quantity - used) {
                *out.entry(id).or_insert(0) += count;
            }
        },
        _ => { out.insert(*id, quantity); },
    }

    out
}

impl Cost {
    pub fn base_ingredients(&self) -> HashMap<ItemId, i32> {
        base_ingredients_aux(&self.id, &self.source, self.quantity)
    }

    pub fn new(index: &Index, id: &ItemId, quantity: i32) -> Result<Cost> {
        Cost::new_with_bank(index, id, quantity, &mut HashMap::new())
    }

    pub fn new_with_bank(index: &Index, id: &ItemId, quantity: i32, bank: &mut HashMap<ItemId, i32>) -> Result<Cost> {
        if let Some(count) = bank.get(id).cloned() {
            if count > 0 {
                let used = std::cmp::min(quantity, count);
                let remaining = quantity - used;
                bank.insert(*id, count - used);
                return Ok(if remaining == 0 {
                    Cost {
                        id: *id,
                        source: Source::Bank { used, rest: None },
                        quantity,
                        total: 0,
                    }
                } else {
                    let rest = Cost::new_with_bank(index, id, remaining, bank)?;
                    Cost {
                        id: *id,
                        source: Source::Bank { used, rest: Some(Box::new(rest.source)) },
                        quantity,
                        total: rest.total,
                    }
                })
            }
        }
        if let Some(value) = vendor(id) {
            return Ok(Cost {
                id: *id,
                source: Source::Vendor,
                quantity,
                total: quantity * value,
            })
        }
        if let Some(value) = special(index, id) {
            return Ok(Cost {
                id: *id,
                source: Source::Special,
                quantity,
                total: quantity * value,
            })
        }
        let recipe = match index.recipes_by_item.get(id) {
            None => {
                return Ok(index.listings.get(id)
                    .and_then(|ls| ls.cost(quantity).ok())
                    .map(|total| Cost {
                        id: *id,
                        source: Source::Auction,
                        quantity,
                        total,
                    })
                    .unwrap_or(Cost {
                        id: *id,
                        source: Source::Unknown,
                        quantity,
                        total: 0,
                    }))
            }
            Some(r) => r,
        };
        let runs = ((quantity as f32) / (recipe.output_item_count as f32)).ceil() as i32;
        let mut craft_total = 0;
        let mut ingredients = HashMap::new();
        // Snapshot the bank before computing crafted cost so it can be set back
        // to this if auctioning is cheaper.
        let old_bank = bank.clone();
        for ing in &recipe.ingredients {
            let ing_cost = Cost::new_with_bank(index, &ing.item_id, ing.count * runs, bank)?;
            craft_total += ing_cost.total;
            ingredients.insert(ing.item_id, ing_cost);
        }
        if let Some(ls) = index.listings.get(id) {
            if let Ok(total) = ls.cost(quantity) {
                if total < craft_total {
                    *bank = old_bank;
                    return Ok(Cost {
                        id: *id,
                        source: Source::Auction,
                        quantity,
                        total,
                    })
                }
            }
        }
        Ok(Cost {
            id: *id,
            source: Source::Recipe {
                id: recipe.id,
                ingredients,
            },
            quantity,
            total: craft_total,
        })
    }
}

pub fn vendor(id: &ItemId) -> Option<i32> {
    Some(match id.0 {
        // Thermocatalytic Reagent
        46747 => 150,
        // Spool of Gossamer Thread
        19790 => 64,
        // Spool of Silk Thread
        19791 => 48,
        // Spool of Linen Thread
        19793 => 32,
        // Spool of Cotton Thread
        19794 => 24,
        // Spool of Wool Thread
        19789 => 16,
        // Spool of Jute Thread
        19792 => 8,
        // Milling Basin
        76839 => 56,
        // Lump of Tin
        19704 => 8,
        // Lump of Coal
        19750 => 16,
        // Lump of Primordium
        19924 => 48,
        _ => return None,
    })
}

pub fn special(index: &Index, id: &ItemId) -> Option<i32> {
    Some(match id.0 {
        // Obsidian Shard
        // 5 for 1 Guild Commendation daily at the Guild Trader
        // Guild Commendation ~= 50s
        19925 => 1000,
        // Charged Quartz Crystal
        // 25 Quartz Crystals at a place of power daily
        43772 => index.listings.get(&ItemId(43773)).unwrap().cost(25).unwrap(),
        // Plaguedoctor's Orichalcum-Imbued Inscription
        // 2500 Volatile Magic + 50 Inscribed Shard ~= 3500 Volatile Magic
        // https://gw2lunchbox.com/IstanShipments.html puts VM at ~40s per 250 (Trophy Shipment)
        // for ~16c per 1 Volatile Magic
        // * 3500 = 56000
        87809 => 2*56000,
        // Plaguedoctor's Intricate Gossamer Insignia
        // 1250 Volatile Magic + 25 Inscribed Shard ~= 1750 Volatile Magic
        // ~= 28000c
        88011 => 2*28000,
        // Branded Mass: 20 Volatile Magic ~= 320c
        89537 => 320,
        // Exquisite Serpentite Jewel
        // It's a hassle to get - dwarven catacombs puzzle area daily chest.
        89696 => 100000,
        // Bottle of Airship Oil
        // Handwave
        69434 => 1000,
        // Pile of Auric Dust
        // Handwave
        69432 => 1000,
        // Ley Line Spark
        // Handwave
        69392 => 1000,
        // Dungeon widgets
        _ if index.offerings.contains(id) => 1000000,

        _ => return None,
    })
}