//! Рецепты крафта. Часть требует станции рядом: верстак, печь, котёл, наковальню.

use crate::blocks::*;
use crate::inventory::Inventory;
use crate::items::{self, ARMOR_MATS, Item, Mat, SLOTS, TOOL_KINDS, TOOL_MATS, ToolKind};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Station {
    Hand,
    Workbench,
    Furnace,
    Cauldron,
    Anvil,
}

impl Station {
    pub fn name(self) -> &'static str {
        match self {
            Station::Hand => "в руках",
            Station::Workbench => "верстак",
            Station::Furnace => "печь",
            Station::Cauldron => "котёл",
            Station::Anvil => "наковальня",
        }
    }
    pub fn block(self) -> Option<u16> {
        match self {
            Station::Hand => None,
            Station::Workbench => Some(WORKBENCH),
            Station::Furnace => Some(FURNACE),
            Station::Cauldron => Some(CAULDRON),
            Station::Anvil => Some(ANVIL),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Recipe {
    pub out: (Item, u16),
    pub ins: Vec<(Item, u16)>,
    pub station: Station,
}

impl Recipe {
    pub fn can_craft(&self, inv: &Inventory) -> bool {
        self.ins.iter().all(|(it, n)| inv.count(*it) >= *n as u32)
    }
}

fn mat_item(m: Mat) -> Item {
    match m {
        Mat::Wood => Item::Block(OAK_PLANKS),
        Mat::Leather => Item::Thing(items::LEATHER),
        Mat::Fur => Item::Thing(items::FUR),
        Mat::Stone => Item::Block(COBBLE),
        Mat::Copper => Item::Thing(items::COPPER_INGOT),
        Mat::Iron => Item::Thing(items::IRON_INGOT),
        Mat::Gold => Item::Thing(items::GOLD_INGOT),
        Mat::Ruby => Item::Thing(items::RUBY),
        Mat::Sapphire => Item::Thing(items::SAPPHIRE),
        Mat::Malachite => Item::Thing(items::MALACHITE),
        Mat::Crystal => Item::Thing(items::CRYSTAL),
        Mat::Heaven => Item::Thing(items::HEAVEN_INGOT),
    }
}

fn build() -> Vec<Recipe> {
    use Station::*;
    let b = Item::Block;
    let t = Item::Thing;
    let mut v: Vec<Recipe> = Vec::new();
    let mut r = |out: (Item, u16), ins: &[(Item, u16)], st: Station| v.push(Recipe { out, ins: ins.to_vec(), station: st });

    // дерево
    for log in [OAK_LOG, SPRUCE_LOG, BIRCH_LOG, SAKURA_LOG, PALM_LOG, LIGHT_LOG] {
        let planks = if log == LIGHT_LOG { LIGHT_PLANKS } else { log + 1 };
        r((b(planks), 4), &[(b(log), 1)], Hand);
    }
    for p in [SPRUCE_LOG + 1, BIRCH_LOG + 1, SAKURA_LOG + 1, PALM_LOG + 1, LIGHT_PLANKS, BAMBOO_BLOCK] {
        r((b(OAK_PLANKS), 1), &[(b(p), 1)], Hand);
    }
    r((t(items::STICK), 4), &[(b(OAK_PLANKS), 2)], Hand);
    r((b(WORKBENCH), 1), &[(b(OAK_PLANKS), 4)], Hand);
    r((b(TORCH), 4), &[(t(items::COAL), 1), (t(items::STICK), 1)], Hand);
    r((b(FURNACE), 1), &[(b(COBBLE), 8)], Workbench);
    r((b(CHEST), 1), &[(b(OAK_PLANKS), 8)], Workbench);
    r((b(CRATE), 1), &[(b(OAK_PLANKS), 6), (t(items::STICK), 2)], Workbench);
    r((b(BARREL), 1), &[(b(OAK_PLANKS), 6), (t(items::IRON_INGOT), 1)], Workbench);
    r((b(LADDER), 3), &[(t(items::STICK), 7)], Workbench);
    r((b(LATTICE), 4), &[(t(items::STICK), 6)], Workbench);
    r((b(BEAM), 2), &[(b(OAK_LOG), 1), (t(items::STICK), 1)], Workbench);
    r((b(BOOKSHELF), 1), &[(b(OAK_PLANKS), 6), (t(items::BOOK), 3)], Workbench);
    r((b(TIMBER_FRAME), 4), &[(b(OAK_PLANKS), 2), (b(WHITEWASH), 2)], Workbench);
    r((b(CAULDRON), 1), &[(t(items::IRON_INGOT), 7)], Workbench);
    r((b(ANVIL), 1), &[(b(IRON_BLOCK), 3), (t(items::IRON_INGOT), 4)], Workbench);
    r((b(IRON_BARS), 16), &[(t(items::IRON_INGOT), 6)], Workbench);
    r((b(LANTERN), 1), &[(t(items::IRON_INGOT), 4), (b(TORCH), 1)], Workbench);
    r((b(CANDLE), 2), &[(t(items::WAX), 1), (t(items::THREAD), 1)], Hand);
    r((b(ROPE), 4), &[(t(items::THREAD), 4)], Hand);
    r((t(items::PAPER), 3), &[(t(items::REED), 3)], Hand);
    r((t(items::BOOK), 1), &[(t(items::PAPER), 3), (t(items::LEATHER), 1)], Hand);
    r((t(items::BOWL), 4), &[(b(OAK_PLANKS), 3)], Hand);
    r((b(BEEHIVE), 1), &[(b(OAK_PLANKS), 6), (t(items::HONEYCOMB), 3)], Workbench);
    r((b(HONEY_BLOCK), 1), &[(t(items::HONEY_JAR), 4)], Hand);
    r((b(HAY), 1), &[(t(items::WHEAT), 9)], Hand);
    r((b(THATCH), 2), &[(t(items::WHEAT), 6)], Hand);
    r((b(FUR_BLOCK), 1), &[(t(items::FUR), 4)], Hand);
    r((b(BONE_BLOCK), 1), &[(t(items::BONE), 9)], Hand);
    r((b(JELLY_BLOCK), 1), &[(t(items::SLIME), 9)], Hand);
    r((b(FEATHER_BLOCK), 1), &[(t(items::FEATHER), 9)], Hand);
    r((b(SALT_BLOCK), 1), &[(t(items::SALT), 9)], Hand);
    r((b(AMBER_BLOCK), 1), &[(t(items::AMBER), 9)], Hand);
    r((t(items::SUGAR), 2), &[(t(items::REED), 1)], Hand);
    r((t(items::FLOUR), 1), &[(t(items::WHEAT), 2)], Hand);
    r((b(WOOL0), 1), &[(t(items::THREAD), 4)], Hand);

    // камень и кирпичи
    r((b(STONE), 1), &[(b(COBBLE), 1)], Furnace);
    r((b(GLASS), 1), &[(b(SAND), 1)], Furnace);
    r((t(items::BOTTLE), 3), &[(b(GLASS), 3)], Workbench);
    r((t(items::BRICK), 1), &[(t(items::CLAY_BALL), 1)], Furnace);
    r((b(BRICKS), 1), &[(t(items::BRICK), 4)], Hand);
    r((b(STONE_BRICKS), 4), &[(b(STONE), 4)], Workbench);
    r((b(CHISELED_STONE), 1), &[(b(STONE_BRICKS), 2)], Workbench);
    r((b(CRACKED_STONE_BRICKS), 1), &[(b(STONE_BRICKS), 1)], Furnace);
    r((b(MOSSY_STONE_BRICKS), 1), &[(b(STONE_BRICKS), 1), (b(MOSS_BLOCK), 1)], Hand);
    r((b(MOSSY_COBBLE), 1), &[(b(COBBLE), 1), (b(MOSS_BLOCK), 1)], Hand);
    r((b(SANDSTONE), 1), &[(b(SAND), 4)], Hand);
    r((b(RED_SANDSTONE), 1), &[(b(RED_SAND), 4)], Hand);
    r((b(SMOOTH_SANDSTONE), 1), &[(b(SANDSTONE), 1)], Furnace);
    r((b(SANDSTONE_BRICKS), 4), &[(b(SANDSTONE), 4)], Workbench);
    r((b(CHISELED_SANDSTONE), 1), &[(b(SANDSTONE_BRICKS), 2)], Workbench);
    r((b(CUT_RED_SANDSTONE), 4), &[(b(RED_SANDSTONE), 4)], Workbench);
    r((b(POLISHED_GRANITE), 4), &[(b(GRANITE), 4)], Workbench);
    r((b(POLISHED_MARBLE), 4), &[(b(MARBLE), 4)], Workbench);
    r((b(MARBLE_PILLAR), 2), &[(b(POLISHED_MARBLE), 2)], Workbench);
    r((b(POLISHED_BASALT), 4), &[(b(BASALT), 4)], Workbench);
    r((b(SLATE_BRICKS), 4), &[(b(SLATE), 4)], Workbench);
    r((b(SLATE_TILES), 4), &[(b(SLATE_BRICKS), 4)], Workbench);
    r((b(TUFF_BRICKS), 4), &[(b(TUFF), 4)], Workbench);
    r((b(OBSIDIAN_BRICKS), 4), &[(b(OBSIDIAN), 4)], Workbench);
    r((b(ICE_BRICKS), 4), &[(b(PACKED_ICE), 4)], Workbench);
    r((b(PACKED_ICE), 1), &[(b(ICE), 4)], Hand);
    r((b(SNOW_BRICKS), 4), &[(b(SNOW_BLOCK), 4)], Hand);
    r((b(DARK_BRICKS), 4), &[(b(BRICKS), 4), (t(items::COAL), 1)], Workbench);
    r((b(ADOBE), 4), &[(b(MUD), 2), (t(items::WHEAT), 2)], Workbench);
    r((b(MUD), 1), &[(b(DIRT), 1), (b(CLAY), 1)], Hand);
    r((b(PAVING), 4), &[(b(COBBLE), 2), (b(GRAVEL), 2)], Workbench);
    r((b(WHITEWASH), 4), &[(b(STONE), 4), (t(items::DYE0), 1)], Workbench);
    r((b(QUARTZ), 1), &[(b(MARBLE), 1), (t(items::SALT), 1)], Furnace);
    r((b(QUARTZ_PILLAR), 2), &[(b(QUARTZ), 2)], Workbench);
    r((b(MOSAIC), 4), &[(b(TERRACOTTA0 + 10), 1), (b(TERRACOTTA0 + 1), 1), (b(TERRACOTTA0 + 5), 1), (b(TERRACOTTA0), 1)], Workbench);
    r((b(SEA_STONE), 4), &[(b(STONE), 4), (b(CORAL_BLUE), 1)], Workbench);
    r((b(SEA_LANTERN), 1), &[(b(SEA_STONE), 4), (b(GLOWSTONE), 1)], Workbench);
    r((b(GOLD_TILES), 4), &[(t(items::GOLD_INGOT), 4), (b(STONE), 1)], Workbench);
    r((b(CLAY), 1), &[(t(items::CLAY_BALL), 4)], Hand);

    // руды и металлы
    r((t(items::IRON_INGOT), 1), &[(b(IRON_ORE), 1)], Furnace);
    r((t(items::COPPER_INGOT), 1), &[(b(COPPER_ORE), 1)], Furnace);
    r((t(items::GOLD_INGOT), 1), &[(b(GOLD_ORE), 1)], Furnace);
    r((t(items::HEAVEN_INGOT), 1), &[(t(items::GOLD_INGOT), 1), (t(items::HEAVEN_DUST), 4)], Anvil);
    let storage = [
        (COAL_BLOCK, items::COAL),
        (IRON_BLOCK, items::IRON_INGOT),
        (COPPER_BLOCK, items::COPPER_INGOT),
        (GOLD_BLOCK, items::GOLD_INGOT),
        (RUBY_BLOCK, items::RUBY),
        (CRYSTAL_BLOCK, items::CRYSTAL),
        (SAPPHIRE_BLOCK, items::SAPPHIRE),
        (MALACHITE_BLOCK, items::MALACHITE),
    ];
    for (blk, it) in storage {
        r((b(blk), 1), &[(t(it), 9)], Workbench);
        r((t(it), 9), &[(b(blk), 1)], Hand);
    }

    // инструменты и оружие
    for k in TOOL_KINDS {
        let (mats, sticks) = match k {
            ToolKind::Sword => (2, 1),
            ToolKind::Pickaxe => (3, 2),
            ToolKind::Axe => (3, 2),
            ToolKind::Shovel => (1, 2),
            ToolKind::Dagger => (1, 1),
            ToolKind::Spear => (1, 3),
            ToolKind::Mace => (4, 1),
            ToolKind::Katana => (3, 1),
            ToolKind::Battleaxe => (5, 2),
            ToolKind::Warhammer => (6, 2),
            ToolKind::Scythe => (3, 3),
            ToolKind::Halberd => (4, 3),
            ToolKind::Rapier => (2, 1),
        };
        for m in TOOL_MATS {
            let st = if matches!(m, Mat::Wood | Mat::Stone) { Workbench } else { Anvil };
            let st = if st == Anvil && matches!(m, Mat::Copper | Mat::Iron) { Workbench } else { st };
            r((Item::Tool(k, m), 1), &[(mat_item(m), mats), (t(items::STICK), sticks)], st);
        }
    }
    // броня
    for s in SLOTS {
        let n = [5, 8, 7, 4][s as usize];
        for m in ARMOR_MATS {
            let st = if matches!(m, Mat::Leather | Mat::Fur | Mat::Copper | Mat::Iron) { Workbench } else { Anvil };
            r((Item::Armor(s, m), 1), &[(mat_item(m), n)], st);
        }
    }

    // готовка
    let cook = [
        (items::RAW_BEEF, items::STEAK),
        (items::RAW_CHICKEN, items::ROAST_CHICKEN),
        (items::RAW_MUTTON, items::ROAST_MUTTON),
        (items::RAW_FISH, items::COOKED_FISH),
        (items::POTATO, items::BAKED_POTATO),
        (items::CARROT, items::BAKED_CARROT),
    ];
    for (a, c) in cook {
        r((t(c), 1), &[(t(a), 1), (t(items::COAL), 1)], Furnace);
    }
    r((t(items::BREAD), 1), &[(t(items::FLOUR), 3)], Furnace);
    r((t(items::COOKIE), 8), &[(t(items::FLOUR), 2), (t(items::COCOA), 1)], Furnace);
    r((t(items::GINGERBREAD), 4), &[(t(items::FLOUR), 2), (t(items::HONEY_JAR), 1)], Furnace);
    r((t(items::PUMPKIN_PIE), 1), &[(b(PUMPKIN), 1), (t(items::SUGAR), 1), (t(items::EGG), 1)], Furnace);
    r((t(items::CAKE), 6), &[(t(items::FLOUR), 3), (t(items::SUGAR), 2), (t(items::EGG), 1), (t(items::MILK), 1)], Workbench);
    r((t(items::CHEESE), 2), &[(t(items::MILK), 1), (t(items::SALT), 1)], Hand);
    r((t(items::PANCAKES), 2), &[(t(items::FLOUR), 1), (t(items::EGG), 1), (t(items::MILK), 1)], Furnace);
    r((t(items::SWEET_ROLL), 2), &[(t(items::FLOUR), 2), (t(items::JAM), 1)], Furnace);
    r((t(items::JAM), 1), &[(t(items::BERRIES), 4), (t(items::SUGAR), 1), (t(items::BOTTLE), 1)], Cauldron);
    r((t(items::HONEY_JAR), 1), &[(t(items::HONEYCOMB), 2), (t(items::BOTTLE), 1)], Hand);
    r((t(items::WAX), 2), &[(t(items::HONEYCOMB), 1)], Furnace);
    r((t(items::MUSHROOM_SOUP), 1), &[(b(RED_MUSHROOM), 1), (b(BROWN_MUSHROOM), 1), (t(items::BOWL), 1)], Hand);
    r((t(items::STEW), 1), &[(t(items::STEAK), 1), (t(items::BAKED_POTATO), 1), (t(items::CARROT), 1), (t(items::BOWL), 1)], Cauldron);
    r((t(items::PORRIDGE), 1), &[(t(items::WHEAT), 3), (t(items::MILK), 1), (t(items::BOWL), 1)], Cauldron);
    r((t(items::SUSHI), 2), &[(t(items::SALMON), 1), (t(items::WHEAT), 1), (t(items::SALT), 1)], Hand);
    r((t(items::KEBAB), 2), &[(t(items::STEAK), 1), (t(items::ROAST_MUTTON), 1), (t(items::STICK), 1)], Furnace);
    r((t(items::GOLDEN_APPLE), 1), &[(t(items::APPLE), 1), (t(items::GOLD_INGOT), 4)], Anvil);
    r((t(items::MORS), 2), &[(t(items::BERRIES), 3), (t(items::SUGAR), 1), (t(items::BOTTLE), 1)], Cauldron);
    r((t(items::ICE_CREAM), 2), &[(t(items::MILK), 1), (t(items::SUGAR), 1), (b(SNOW_BLOCK), 1)], Hand);
    r((t(items::NECTAR), 1), &[(b(HEAVEN_LILY), 3), (t(items::BOTTLE), 1)], Cauldron);
    r((t(items::AMBROSIA), 1), &[(t(items::NECTAR), 1), (t(items::GOLDEN_APPLE), 1), (t(items::HEAVEN_DUST), 2)], Cauldron);
    r((t(items::SALT), 2), &[(b(SALT_BLOCK), 1)], Hand);

    // красители и цветные блоки
    let flower_dye: [(u16, usize); 12] = [
        (DAISY, 0), (POPPY, 5), (CORNFLOWER, 12), (DANDELION, 7), (TULIP_RED, 5), (TULIP_ORANGE, 6),
        (TULIP_PINK, 15), (LAVENDER, 13), (BLUEBELL, 11), (SUNFLOWER, 7), (ROSE_BUSH, 14), (DESERT_ROSE, 15),
    ];
    for (f, d) in flower_dye {
        r((t(items::DYE0 + d as u16), 2), &[(b(f), 1)], Hand);
    }
    r((t(items::DYE0 + 3), 2), &[(t(items::COAL), 1)], Hand);
    r((t(items::DYE0 + 4), 2), &[(t(items::COCOA), 1)], Hand);
    r((t(items::DYE0 + 0), 3), &[(t(items::BONE), 1)], Hand);
    r((t(items::DYE0 + 9), 2), &[(b(CACTUS), 1)], Furnace);
    r((t(items::DYE0 + 8), 2), &[(t(items::DYE0 + 9), 1), (t(items::DYE0), 1)], Hand);
    r((t(items::DYE0 + 10), 2), &[(t(items::DYE0 + 9), 1), (t(items::DYE0 + 12), 1)], Hand);
    r((t(items::DYE0 + 1), 2), &[(t(items::DYE0 + 2), 1), (t(items::DYE0), 1)], Hand);
    r((t(items::DYE0 + 2), 2), &[(t(items::DYE0 + 3), 1), (t(items::DYE0), 1)], Hand);
    for i in 0..16u16 {
        let dye = t(items::DYE0 + i as u16);
        r((b(WOOL0 + i), 1), &[(b(WOOL0), 1), (dye, 1)], Hand);
        r((b(CONCRETE0 + i), 8), &[(b(SAND), 4), (b(GRAVEL), 4), (dye, 1)], Workbench);
        r((b(TERRACOTTA0 + i), 8), &[(b(CLAY), 8), (dye, 1)], Furnace);
    }
    let glass_cols: [(u16, usize); 4] = [(GLASS_RED, 5), (GLASS_BLUE, 12), (GLASS_GREEN, 9), (GLASS_YELLOW, 7)];
    for (g, d) in glass_cols {
        r((b(g), 8), &[(b(GLASS), 8), (t(items::DYE0 + d as u16), 1)], Workbench);
    }
    let mut gi = 0u16;
    for i in 0..16u16 {
        if matches!(i, 5 | 7 | 9 | 12) {
            continue;
        }
        r((b(GLASS_EXTRA0 + gi), 8), &[(b(GLASS), 8), (t(items::DYE0 + i), 1)], Workbench);
        gi += 1;
    }
    for (k, i) in [0u16, 5, 6, 7, 8, 11, 13, 15].iter().enumerate() {
        r((b(LAMP0 + k as u16), 2), &[(b(GLOWSTONE), 1), (b(GLASS), 2), (t(items::DYE0 + i), 1)], Workbench);
    }

    // Рай
    r((b(HEAVEN_BRICKS), 4), &[(b(HEAVEN_STONE), 4)], Workbench);
    r((b(HALO_STONE), 2), &[(b(HEAVEN_STONE), 2), (t(items::HEAVEN_DUST), 1)], Workbench);
    r((b(SUN_BLOCK), 1), &[(b(GLOWSTONE), 1), (t(items::HEAVEN_DUST), 4)], Anvil);
    r((b(CELESTIAL_GLASS), 4), &[(b(GLASS), 4), (t(items::HEAVEN_DUST), 1)], Workbench);
    r((b(STARDUST_BLOCK), 1), &[(t(items::HEAVEN_DUST), 9)], Hand);
    r((b(RAINBOW_BLOCK), 4), &[(b(CLOUD), 4), (t(items::DYE0 + 5), 1), (t(items::DYE0 + 7), 1), (t(items::DYE0 + 12), 1)], Workbench);
    r((b(PORTAL_FRAME), 1), &[(t(items::GOLD_INGOT), 4), (b(OBSIDIAN), 1)], Anvil);
    r((b(PORTAL), 1), &[(t(items::HEAVEN_DUST), 8), (t(items::CRYSTAL), 1)], Anvil);
    r((b(GLOWSTONE), 1), &[(t(items::HEAVEN_DUST), 2), (t(items::COAL), 2)], Furnace);

    // зелья: бутылка + особые ингредиенты в котле
    let pot_ing: [&[(Item, u16)]; 10] = [
        &[(t(items::APPLE), 2), (t(items::GOLD_INGOT), 1)],
        &[(t(items::HONEY_JAR), 1), (t(items::BERRIES), 2)],
        &[(t(items::SUGAR), 3)],
        &[(t(items::FEATHER), 2), (t(items::SLIME), 1)],
        &[(t(items::RAW_BEEF), 1), (t(items::RUBY), 1)],
        &[(t(items::CARROT), 2), (b(GLOW_MUSHROOM), 1)],
        &[(b(MAGMA), 1), (t(items::SALT), 1)],
        &[(t(items::BREAD), 2), (t(items::MILK), 1)],
        &[(t(items::FEATHER), 4)],
        &[(t(items::SAPPHIRE), 1), (t(items::SUGAR), 1)],
    ];
    for (i, ing) in pot_ing.iter().enumerate() {
        let mut ins = vec![(t(items::BOTTLE), 1)];
        ins.extend_from_slice(ing);
        r((t(items::POTION0 + i as u16), 1), &ins, Cauldron);
    }
    // ===== v0.9 =====
    use items as it;
    for log in [MAPLE_LOG, EBONY_LOG, WILLOW_LOG, ACACIA_LOG, MANGROVE_LOG, CEDAR_LOG] {
        r((b(log + 1), 4), &[(b(log), 1)], Hand);
        r((b(OAK_PLANKS), 1), &[(b(log + 1), 1)], Hand);
    }
    for (tile, src) in [(STONE_TILES, STONE), (GRANITE_TILES, POLISHED_GRANITE), (MARBLE_TILES, POLISHED_MARBLE), (BASALT_TILES, POLISHED_BASALT), (TUFF_TILES, TUFF_BRICKS), (OBSIDIAN_TILES, OBSIDIAN_BRICKS)] {
        r((b(tile), 4), &[(b(src), 4)], Workbench);
    }
    for (ch, src) in [(CHISELED_GRANITE, POLISHED_GRANITE), (CHISELED_MARBLE, POLISHED_MARBLE), (CHISELED_BASALT, POLISHED_BASALT), (CHISELED_SLATE, SLATE_BRICKS), (CHISELED_TUFF, TUFF_BRICKS), (CHISELED_OBSIDIAN, OBSIDIAN_BRICKS)] {
        r((b(ch), 1), &[(b(src), 2)], Workbench);
    }
    r((b(GRANITE_BRICKS), 4), &[(b(POLISHED_GRANITE), 4)], Workbench);
    r((b(MARBLE_BRICKS), 4), &[(b(POLISHED_MARBLE), 4)], Workbench);
    r((b(BASALT_BRICKS), 4), &[(b(POLISHED_BASALT), 4)], Workbench);
    r((b(SMOOTH_STONE), 1), &[(b(STONE), 1)], Furnace);
    r((b(POLISHED_SLATE), 4), &[(b(SLATE), 4)], Workbench);
    r((b(POLISHED_TUFF), 4), &[(b(TUFF), 4)], Workbench);
    r((b(POLISHED_OBSIDIAN), 4), &[(b(OBSIDIAN), 4), (t(it::SALT), 1)], Anvil);
    for i in 0..16u16 {
        let dye = t(items::DYE0 + i);
        r((b(GLAZED0 + i), 1), &[(b(TERRACOTTA0 + i), 1)], Furnace);
        r((b(PAINTED_PLANKS0 + i), 8), &[(b(OAK_PLANKS), 8), (dye, 1)], Workbench);
    }
    for (k, i) in [1u16, 2, 3, 4, 9, 10, 12, 14].iter().enumerate() {
        r((b(LAMP_EXTRA0 + k as u16), 2), &[(b(GLOWSTONE), 1), (b(GLASS), 2), (t(items::DYE0 + i), 1)], Workbench);
    }
    let metal_src = [COPPER_BLOCK, IRON_BLOCK, GOLD_BLOCK, RUBY_BLOCK, SAPPHIRE_BLOCK, MALACHITE_BLOCK, CRYSTAL_BLOCK, STARDUST_BLOCK];
    for (k, src) in metal_src.iter().enumerate() {
        r((b(CHISELED_METAL0 + k as u16), 4), &[(b(*src), 4)], Anvil);
        r((b(METAL_PILLAR0 + k as u16), 2), &[(b(CHISELED_METAL0 + k as u16), 2)], Anvil);
    }
    r((b(COPPER_GRATE), 8), &[(t(it::COPPER_INGOT), 6)], Workbench);
    r((b(GOLD_GRATE), 8), &[(t(it::GOLD_INGOT), 6)], Anvil);
    r((b(HEAVEN_GRATE), 8), &[(t(it::HEAVEN_INGOT), 6)], Anvil);
    // природа
    r((b(BLACK_SAND), 4), &[(b(SAND), 4), (t(it::COAL), 1)], Hand);
    r((b(ROOTED_DIRT), 2), &[(b(DIRT), 2), (t(it::STICK), 1)], Hand);
    r((b(CRACKED_EARTH), 1), &[(b(DIRT), 1)], Furnace);
    r((b(GLOW_MOSS), 2), &[(b(MOSS_BLOCK), 2), (b(GLOW_MUSHROOM), 1)], Hand);
    r((b(MYCELIUM), 2), &[(b(DIRT), 2), (b(BROWN_MUSHROOM), 1), (b(RED_MUSHROOM), 1)], Hand);
    r((b(CALCITE), 2), &[(b(STONE), 1), (t(it::BONE), 1)], Workbench);
    r((b(LIMESTONE), 4), &[(b(SANDSTONE), 2), (b(CALCITE), 2)], Workbench);
    r((b(CHALK), 4), &[(b(CALCITE), 2), (t(it::SALT), 1)], Hand);
    r((b(DRIPSTONE), 2), &[(b(TUFF), 1), (b(CLAY), 1)], Workbench);
    r((b(SNOW_PACKED), 1), &[(b(SNOW_BLOCK), 2)], Hand);
    r((b(KELP_BLOCK), 1), &[(b(REEDS), 6), (t(it::SALT), 1)], Furnace);
    r((b(CORAL_PINK), 1), &[(t(it::CORAL_SHARD), 4), (t(items::DYE0 + 15), 1)], Hand);
    r((b(CORAL_PURPLE), 1), &[(t(it::CORAL_SHARD), 4), (t(items::DYE0 + 13), 1)], Hand);
    for (f, d) in [(ORCHID, 14u16), (PEONY, 15), (THISTLE, 13), (FIREFLOWER, 6)] {
        r((t(items::DYE0 + d), 2), &[(b(f), 1)], Hand);
    }
    // декор
    r((b(PARQUET), 4), &[(b(OAK_PLANKS), 2), (b(EBONY_LOG + 1), 2)], Workbench);
    r((b(CHECKER_TILES), 4), &[(b(QUARTZ), 2), (b(POLISHED_BASALT), 2)], Workbench);
    r((b(APPLE_CRATE), 1), &[(b(CRATE), 1), (t(it::APPLE), 9)], Hand);
    r((b(CARROT_CRATE), 1), &[(b(CRATE), 1), (t(it::CARROT), 9)], Hand);
    r((b(POTATO_CRATE), 1), &[(b(CRATE), 1), (t(it::POTATO), 9)], Hand);
    r((b(FISH_BARREL), 1), &[(b(BARREL), 1), (t(it::RAW_FISH), 9)], Hand);
    r((t(it::APPLE), 9), &[(b(APPLE_CRATE), 1)], Hand);
    r((t(it::CARROT), 9), &[(b(CARROT_CRATE), 1)], Hand);
    r((t(it::POTATO), 9), &[(b(POTATO_CRATE), 1)], Hand);
    r((t(it::RAW_FISH), 9), &[(b(FISH_BARREL), 1)], Hand);
    r((b(JAR_SHELF), 1), &[(b(OAK_PLANKS), 6), (t(it::BOTTLE), 3)], Workbench);
    r((b(SCROLL_SHELF), 1), &[(b(OAK_PLANKS), 6), (t(it::SCROLL), 3)], Workbench);
    r((b(SAFE), 1), &[(b(IRON_BLOCK), 2), (t(it::GEAR), 2)], Anvil);
    r((b(CLOCK_BLOCK), 1), &[(b(OAK_PLANKS), 4), (t(it::GEAR), 1), (t(it::GOLD_INGOT), 1)], Workbench);
    r((b(MAP_BLOCK), 1), &[(t(it::PAPER), 8), (t(it::INK), 1)], Workbench);
    r((b(STAINED_WINDOW), 4), &[(b(GLASS_RED), 1), (b(GLASS_BLUE), 1), (b(GLASS_YELLOW), 1), (b(GLASS_GREEN), 1)], Workbench);
    r((b(PAPER_LANTERN), 2), &[(t(it::PAPER), 4), (b(CANDLE), 1), (t(it::STICK), 2)], Workbench);
    r((b(WEAPON_RACK), 1), &[(b(OAK_PLANKS), 4), (t(it::IRON_INGOT), 3)], Workbench);
    r((b(TAPESTRY), 1), &[(t(it::CLOTH), 4), (t(items::DYE0 + 7), 1)], Workbench);
    r((b(RUG_BLOCK), 2), &[(t(it::CLOTH), 3), (t(it::THREAD), 2)], Workbench);
    // материалы
    r((t(it::STEEL_INGOT), 1), &[(t(it::IRON_INGOT), 1), (t(it::COAL), 2)], Furnace);
    r((t(it::BRONZE_INGOT), 2), &[(t(it::COPPER_INGOT), 1), (t(it::GOLD_INGOT), 1)], Furnace);
    r((t(it::SILK), 2), &[(t(it::THREAD), 3)], Workbench);
    r((t(it::CLOTH), 2), &[(t(it::THREAD), 4), (b(WOOL0), 1)], Workbench);
    r((t(it::STRAP), 2), &[(t(it::LEATHER), 1)], Hand);
    r((t(it::GEAR), 2), &[(t(it::STEEL_INGOT), 1)], Anvil);
    r((t(it::INK), 1), &[(t(it::BOTTLE), 1), (t(items::DYE0 + 3), 1)], Hand);
    r((t(it::SCROLL), 1), &[(t(it::PAPER), 2), (t(it::INK), 1)], Hand);
    r((t(it::GUNPOWDER), 2), &[(t(it::CHARCOAL), 1), (t(it::SALT), 1), (t(it::FLINT), 1)], Workbench);
    r((t(it::CHARCOAL), 1), &[(b(OAK_LOG), 1)], Furnace);
    r((t(it::OBSIDIAN_SHARD), 4), &[(b(OBSIDIAN), 1)], Anvil);
    r((t(it::GEM_DUST), 3), &[(t(it::RUBY), 1), (t(it::SAPPHIRE), 1), (t(it::MALACHITE), 1)], Anvil);
    r((t(it::PEARL), 1), &[(t(it::CORAL_SHARD), 3), (t(it::SALT), 1)], Cauldron);
    // еда
    r((t(it::PIZZA), 2), &[(t(it::FLOUR), 2), (t(it::CHEESE), 1), (t(it::STEAK), 1)], Furnace);
    r((t(it::DONUT), 4), &[(t(it::FLOUR), 2), (t(it::SUGAR), 1), (t(it::EGG), 1)], Furnace);
    r((t(it::BURGER), 1), &[(t(it::BREAD), 1), (t(it::STEAK), 1), (t(it::CHEESE), 1)], Hand);
    r((t(it::CHOCOLATE), 2), &[(t(it::COCOA), 2), (t(it::SUGAR), 1), (t(it::MILK), 1)], Cauldron);
    r((t(it::CANDY), 4), &[(t(it::SUGAR), 2), (t(items::DYE0 + 5), 1)], Cauldron);
    r((t(it::CARAMEL_APPLE), 1), &[(t(it::APPLE), 1), (t(it::SUGAR), 2), (t(it::STICK), 1)], Cauldron);
    r((t(it::BORSCHT), 1), &[(t(it::STEAK), 1), (t(it::POTATO), 1), (t(it::CARROT), 1), (t(items::DYE0 + 5), 1), (t(it::BOWL), 1)], Cauldron);
    r((t(it::SHCHI), 1), &[(t(it::POTATO), 1), (t(it::CARROT), 1), (b(FERN), 2), (t(it::BOWL), 1)], Cauldron);
    r((t(it::PELMENI), 1), &[(t(it::FLOUR), 2), (t(it::RAW_BEEF), 1), (t(it::BOWL), 1)], Cauldron);
    r((t(it::PILAF), 1), &[(t(it::WHEAT), 3), (t(it::RAW_MUTTON), 1), (t(it::CARROT), 1), (t(it::BOWL), 1)], Cauldron);
    r((t(it::OMELETTE), 1), &[(t(it::EGG), 2), (t(it::MILK), 1)], Furnace);
    r((t(it::BLINI_JAM), 2), &[(t(it::PANCAKES), 1), (t(it::JAM), 1)], Hand);
    r((t(it::SYRNIKI), 3), &[(t(it::CHEESE), 1), (t(it::FLOUR), 1), (t(it::EGG), 1)], Furnace);
    r((t(it::SHAWARMA), 1), &[(t(it::BREAD), 1), (t(it::ROAST_CHICKEN), 1), (t(it::CARROT), 1)], Hand);
    r((t(it::FRIED_MUSHROOMS), 1), &[(b(BROWN_MUSHROOM), 2), (t(it::STICK), 1)], Furnace);
    r((t(it::KOMPOT), 2), &[(t(it::CHERRIES), 2), (t(it::PLUM), 1), (t(it::BOTTLE), 1)], Cauldron);
    r((t(it::KVASS), 2), &[(t(it::BREAD), 1), (t(it::SUGAR), 1), (t(it::BOTTLE), 1)], Cauldron);
    r((t(it::TEA), 1), &[(b(LAVENDER), 1), (t(it::BOTTLE), 1)], Cauldron);
    r((t(it::COCOA_DRINK), 1), &[(t(it::COCOA), 1), (t(it::MILK), 1), (t(it::SUGAR), 1)], Cauldron);
    r((t(it::HONEY_CAKE), 4), &[(t(it::FLOUR), 3), (t(it::HONEY_JAR), 1), (t(it::EGG), 1)], Furnace);
    r((t(it::HEAVEN_PIE), 1), &[(t(it::FLOUR), 2), (t(it::STARFRUIT), 1), (t(it::HEAVEN_DUST), 2)], Furnace);
    r((t(it::STAR_COOKIE), 4), &[(t(it::FLOUR), 1), (t(it::STARFRUIT), 1)], Furnace);
    r((t(it::CHERRIES), 4), &[(b(SAKURA_LEAVES + 0), 4)], Hand);
    r((t(it::GRAPES), 2), &[(b(WILLOW_LEAVES), 4)], Hand);
    r((t(it::LEMON), 2), &[(b(ACACIA_LEAVES), 4)], Hand);
    v
}

static RECIPES: OnceLock<Vec<Recipe>> = OnceLock::new();

pub fn recipes() -> &'static Vec<Recipe> {
    RECIPES.get_or_init(build)
}

/// Выполнить рецепт: списать ингредиенты и выдать результат.
pub fn craft(r: &Recipe, inv: &mut Inventory) -> bool {
    if !r.can_craft(inv) {
        return false;
    }
    for (it, n) in &r.ins {
        inv.remove(*it, *n);
    }
    inv.add(r.out.0, r.out.1);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn many_recipes_and_crafting_works() {
        assert!(recipes().len() > 200, "{}", recipes().len());
        let mut inv = Inventory::new();
        inv.add(Item::Block(OAK_LOG), 2);
        let planks = recipes().iter().find(|r| r.out.0 == Item::Block(OAK_PLANKS) && r.ins[0].0 == Item::Block(OAK_LOG)).unwrap();
        assert!(craft(planks, &mut inv));
        assert_eq!(inv.count(Item::Block(OAK_PLANKS)), 4);
        // у всех предметов-результатов есть имя
        for r in recipes() {
            assert!(!r.out.0.name().is_empty());
        }
    }
}

