use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro128PlusPlus as GameRng;
use shipyard::{
    AllStoragesViewMut, EntitiesView, EntityId, Get, IntoIter, IntoWithId, UniqueView,
    UniqueViewMut, View, ViewMut,
};
use std::hash::Hasher;
use wyhash::WyHash;

use crate::{
    components::{
        Asleep, BlocksTile, CombatBonus, CombatStats, Coord, Equipment, Experience,
        GivesExperience, HurtBy, Name, Tally,
    },
    magicnum,
    map::Map,
    message::Messages,
    player::{PlayerAlive, PlayerId},
    saveload, spawn, GameSeed, TurnCount,
};

pub fn melee_attack(
    (attacker, defender): (EntityId, EntityId),
    entities: EntitiesView,
    mut msgs: UniqueViewMut<Messages>,
    game_seed: UniqueView<GameSeed>,
    turn_count: UniqueView<TurnCount>,
    mut combat_stats: ViewMut<CombatStats>,
    mut hurt_bys: ViewMut<HurtBy>,
    mut tallies: ViewMut<Tally>,
    (asleeps, combat_bonuses, equipments, names, coords): (
        View<Asleep>,
        View<CombatBonus>,
        View<Equipment>,
        View<Name>,
        View<Coord>,
    ),
) {
    let att_name = &names.get(attacker).unwrap().0;
    let def_name = &names.get(defender).unwrap().0;
    let mut rng = {
        let mut hasher = WyHash::with_seed(magicnum::MELEE_ATTACK);
        hasher.write_u64(game_seed.0);
        hasher.write_u64(turn_count.0);
        if let Ok(attacker_coord) = coords.get(attacker) {
            hasher.write_i32(attacker_coord.0.x);
            hasher.write_i32(attacker_coord.0.y);
        }
        if let Ok(defender_coord) = coords.get(defender) {
            hasher.write_i32(defender_coord.0.x);
            hasher.write_i32(defender_coord.0.y);
        }
        GameRng::seed_from_u64(hasher.finish())
    };

    if !asleeps.contains(defender) && rng.gen_ratio(1, 10) {
        msgs.add(format!("{} misses {}.", att_name, def_name));
        return;
    }

    let attack_value = combat_stats.get(attacker).unwrap().attack
        + equipments.get(attacker).map_or(0.0, |equip| {
            equip
                .weapon
                .iter()
                .chain(equip.armor.iter())
                .filter_map(|&e| combat_bonuses.get(e).ok())
                .map(|b| b.attack)
                .sum()
        });
    let defense_value = combat_stats.get(defender).unwrap().defense
        + equipments.get(defender).map_or(0.0, |equip| {
            equip
                .weapon
                .iter()
                .chain(equip.armor.iter())
                .filter_map(|&e| combat_bonuses.get(e).ok())
                .map(|b| b.defense)
                .sum()
        });
    // Attack is twice defense most of the time.
    let mut damage = if attack_value >= defense_value * 2.0 {
        attack_value - defense_value
    } else {
        attack_value * (0.25 + (0.125 * attack_value / defense_value.max(1.0)).min(0.25))
    };

    // Fluctuate damage by a random amount.
    let mut suffix = '!';
    if rng.gen() {
        if rng.gen() {
            damage *= 1.5;
            suffix = '‼';
        } else {
            damage *= 0.5;
            suffix = '.';
        }
    }

    // Randomly round to nearest integer, e.g. 3.1 damage has a 10% chance to round to 4.
    let damage = damage.trunc() as i32
        + if rng.gen::<f32>() < damage.fract() {
            1
        } else {
            0
        };

    if damage > 0 {
        (&mut combat_stats).get(defender).unwrap().hp -= damage;
        entities.add_component(defender, &mut hurt_bys, HurtBy::Someone(attacker));
        if let Ok(att_tally) = (&mut tallies).get(attacker) {
            att_tally.damage_dealt += damage.max(0) as u64;
        }
        if let Ok(def_tally) = (&mut tallies).get(defender) {
            def_tally.damage_taken += damage.max(0) as u64;
        }
        msgs.add(format!(
            "{} hits {} for {} hp{}",
            att_name, def_name, damage, suffix
        ));
    } else {
        msgs.add(format!(
            "{} hits {}, but does no damage.",
            att_name, def_name
        ));
    }
}

/// Check for dead entities, do any special handling for them and delete them.
pub fn handle_dead_entities(mut all_storages: AllStoragesViewMut) {
    loop {
        let mut entities = [EntityId::dead(); 10];
        let mut num_entities = 0;

        // Fill buffer with dead entities.
        all_storages.run(|combat_stats: View<CombatStats>| {
            for ((id, _), entity) in combat_stats
                .iter()
                .with_id()
                .into_iter()
                .filter(|(_, stats)| stats.hp <= 0)
                .zip(entities.iter_mut())
            {
                *entity = id;
                num_entities += 1;
            }
        });

        for &entity in entities.iter().take(num_entities) {
            all_storages.run(|mut msgs: UniqueViewMut<Messages>, names: View<Name>| {
                msgs.add(format!("{} dies!", &names.get(entity).unwrap().0));
            });

            all_storages.run(
                |mut exps: ViewMut<Experience>,
                 gives_exps: View<GivesExperience>,
                 hurt_bys: View<HurtBy>,
                 mut tallies: ViewMut<Tally>| {
                    if let Ok(&HurtBy::Someone(receiver)) = hurt_bys.get(entity) {
                        // Credit kill to whoever last hurt this entity.
                        if let Ok(receiver_tally) = (&mut tallies).get(receiver) {
                            receiver_tally.kills += 1;
                        }

                        // Give experience to whoever last hurt this entity.
                        if let Ok(receiver_exp) = (&mut exps).get(receiver) {
                            if let Ok(gives_exp) = gives_exps.get(entity) {
                                receiver_exp.exp += gives_exp.0;
                            }
                        }
                    }
                },
            );

            if entity == all_storages.borrow::<UniqueView<PlayerId>>().unwrap().0 {
                // The player has died.
                all_storages.run(
                    |mut msgs: UniqueViewMut<Messages>,
                     mut player_alive: UniqueViewMut<PlayerAlive>| {
                        msgs.add("Press SPACE to continue...".into());
                        player_alive.0 = false;
                    },
                );

                saveload::delete_save_file();

                // Don't handle any more dead entities.
                num_entities = 0;
                break;
            } else {
                // Remove dead entity from the map.
                all_storages.run(
                    |mut map: UniqueViewMut<Map>,
                     blocks_tile: View<BlocksTile>,
                     coords: View<Coord>| {
                        map.remove_entity(
                            entity,
                            coords.get(entity).unwrap().0.into(),
                            blocks_tile.contains(entity),
                        );
                    },
                );

                // Delete the dead entity.
                spawn::despawn_entity(&mut all_storages, entity);
            }
        }

        if num_entities == 0 {
            break;
        }
    }
}

/// Clear all HurtBy components off of all entities.
pub fn clear_hurt_bys(mut hurt_bys: ViewMut<HurtBy>) {
    hurt_bys.clear();
}
