use shipyard::{UniqueView, World};

use crate::{
    gamekey::{self, GameKey},
    gamesym::GameSym,
    ui::{self, Options},
};
use ruggle::{util::Size, InputBuffer, InputEvent, KeyMods, TileGrid, Tileset};

use super::{ModeControl, ModeResult, ModeUpdate};

const YES_STR: &str = "[ Yes ]";
const NO_STR: &str = "[ No ]";

pub enum YesNoDialogModeResult {
    AppQuit,
    Yes,
    No,
}

pub struct YesNoDialogMode {
    prompt: String,
    yes_selected: bool,
}

impl From<bool> for YesNoDialogModeResult {
    fn from(yes: bool) -> Self {
        if yes {
            Self::Yes
        } else {
            Self::No
        }
    }
}

/// A yes-or-no dialog box with a prompt that shows up in the center of the screen.
impl YesNoDialogMode {
    pub fn new(prompt: String, yes_default: bool) -> Self {
        Self {
            prompt,
            yes_selected: yes_default,
        }
    }

    pub fn prepare_grids(
        &self,
        world: &World,
        grids: &mut Vec<TileGrid<GameSym>>,
        tilesets: &[Tileset<GameSym>],
        window_size: Size,
    ) {
        let text_zoom = world.borrow::<UniqueView<Options>>().text_zoom;
        let new_grid_size = Size {
            w: 4 + self.prompt.len().max(YES_STR.len() + NO_STR.len() + 2) as u32,
            h: 7,
        };

        if !grids.is_empty() {
            grids[0].resize(new_grid_size);
        } else {
            grids.push(TileGrid::new(new_grid_size, tilesets, 0));
            grids[0].view.clear_color = None;
        }

        grids[0].view_centered(tilesets, text_zoom, (0, 0).into(), window_size);
        grids[0].view.zoom = text_zoom;
    }

    pub fn update(
        &mut self,
        _world: &World,
        inputs: &mut InputBuffer,
        _pop_result: &Option<ModeResult>,
    ) -> (ModeControl, ModeUpdate) {
        inputs.prepare_input();

        if let Some(InputEvent::AppQuit) = inputs.get_input() {
            return (
                ModeControl::Pop(YesNoDialogModeResult::AppQuit.into()),
                ModeUpdate::Immediate,
            );
        } else if let Some(InputEvent::Press(keycode)) = inputs.get_input() {
            match gamekey::from_keycode(keycode, inputs.get_mods(KeyMods::SHIFT)) {
                GameKey::Left => self.yes_selected = true,
                GameKey::Right => self.yes_selected = false,
                GameKey::Confirm => {
                    return (
                        ModeControl::Pop(YesNoDialogModeResult::from(self.yes_selected).into()),
                        ModeUpdate::Immediate,
                    )
                }
                GameKey::Cancel => {
                    return (
                        ModeControl::Pop(YesNoDialogModeResult::No.into()),
                        ModeUpdate::Immediate,
                    )
                }
                _ => {}
            }
        }

        (ModeControl::Stay, ModeUpdate::WaitForEvent)
    }

    pub fn draw(&self, _world: &World, grids: &mut [TileGrid<GameSym>], active: bool) {
        let grid = &mut grids[0];
        let yes_x = grid.width() as i32 - (YES_STR.len() + NO_STR.len() + 4) as i32;
        let no_x = grid.width() as i32 - NO_STR.len() as i32 - 2;

        grid.view.color_mod = if active {
            ui::color::WHITE
        } else {
            ui::color::GRAY
        };

        grid.set_draw_fg(ui::color::WHITE);
        grid.set_draw_bg(ui::color::BLACK);
        grid.draw_box((0, 0), (grid.width(), grid.height()));
        grid.print((2, 2), &self.prompt);

        grid.set_draw_bg(ui::color::SELECTED_BG);
        grid.print_color((yes_x, 4), false, self.yes_selected, YES_STR);
        grid.print_color((no_x, 4), false, !self.yes_selected, NO_STR);
    }
}
