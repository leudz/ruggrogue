use shipyard::{
    EntitiesView, EntityId, Get, IntoIter, IntoWithId, Unique, UniqueView, UniqueViewMut, View,
    ViewMut, World,
};
use std::{cmp::Reverse, collections::BinaryHeap};

use crate::{
    components::{BlocksTile, Coord, FieldOfView, Monster},
    damage, item,
    map::Map,
    player::{self, PlayerId},
};

#[derive(Unique)]
pub struct MonsterTurns(BinaryHeap<(Reverse<i32>, EntityId)>);

impl MonsterTurns {
    pub fn new() -> MonsterTurns {
        MonsterTurns(BinaryHeap::new())
    }
}

impl Default for MonsterTurns {
    fn default() -> Self {
        Self::new()
    }
}

pub fn enqueue_monster_turns(
    mut monster_turns: UniqueViewMut<MonsterTurns>,
    player_id: UniqueView<PlayerId>,
    coords: View<Coord>,
    monsters: View<Monster>,
) {
    let player_coord = coords.get(player_id.0).unwrap();

    for (id, (_, coord)) in (&monsters, &coords).iter().with_id() {
        // Monsters close to the player get their turns first.
        monster_turns
            .0
            .push((Reverse(coord.dist(player_coord)), id));
    }
}

fn do_turn_for_one_monster(world: &World, monster: EntityId) {
    if item::is_asleep(world, monster) {
        item::handle_sleep_turn(world, monster);
    } else if world.run_with_data(player::can_see_player, monster) {
        let mut map = world.borrow::<UniqueViewMut<Map>>().unwrap();
        let player_id = world.borrow::<UniqueView<PlayerId>>().unwrap();
        let (player_pos, pos): ((i32, i32), (i32, i32)) = {
            let coords = world.borrow::<View<Coord>>().unwrap();
            (
                coords.get(player_id.0).unwrap().0.into(),
                coords.get(monster).unwrap().0.into(),
            )
        };

        if let Some(step) = ruggrogue::find_path(&*map, pos, player_pos, 4, true).nth(1) {
            if step == player_pos {
                world.run_with_data(damage::melee_attack, (monster, player_id.0));
            } else {
                let blocks = world.borrow::<View<BlocksTile>>().unwrap();
                let mut coords = world.borrow::<ViewMut<Coord>>().unwrap();
                let mut fovs = world.borrow::<ViewMut<FieldOfView>>().unwrap();

                map.move_entity(monster, pos, step, blocks.contains(monster));
                (&mut coords).get(monster).unwrap().0 = step.into();
                (&mut fovs).get(monster).unwrap().dirty = true;
            }
        }
    }
}

pub fn do_monster_turns(world: &World) {
    let (entities, mut monster_turns) = world
        .borrow::<(EntitiesView, UniqueViewMut<MonsterTurns>)>()
        .unwrap();

    while let Some((_, monster)) = monster_turns.0.pop() {
        if entities.is_alive(monster) {
            do_turn_for_one_monster(world, monster);
        }
    }
}
