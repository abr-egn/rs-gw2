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
}

impl Source {
    pub fn to_str(&self) -> &'static str {
        match *self {
            Source::Unknown => " [UNKNOWN]",
            Source::Special => " [SPECIAL]",
            Source::Vendor => " [VENDOR]",
            _ => "",
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

impl Cost {
    pub fn new(index: &Index, id: &ItemId, quantity: i32) -> Result<Cost> {
        if let Some(value) = vendor(id) {
            return Ok(Cost {
                id: *id,
                source: Source::Vendor,
                quantity,
                total: quantity * value,
            })
        }
        if let Some(value) = special(index, id)? {
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
        for ing in &recipe.ingredients {
            let ing_cost = Cost::new(index, &ing.item_id, ing.count * runs)?;
            craft_total += ing_cost.total;
            ingredients.insert(ing.item_id, ing_cost);
        }
        if let Some(ls) = index.listings.get(id) {
            if let Ok(total) = ls.cost(quantity) {
                if total < craft_total {
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

fn vendor(id: &ItemId) -> Option<i32> {
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

fn special(index: &Index, id: &ItemId) -> Result<Option<i32>> {
    Ok(Some(match id.0 {
        // Obsidian Shard
        // 5 for 1 Guild Commendation daily at the Guild Trader
        // Guild Commendation ~= 50s
        19925 => 1000,
        // Charged Quartz Crystal
        // 25 Quartz Crystals at a place of power daily
        43772 => index.listings.get(&ItemId(43773)).unwrap().cost(25)?,
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

        _ => return Ok(None),
    }))
}