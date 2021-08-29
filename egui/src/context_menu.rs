use super::{
    menu::{MenuResponse, MenuRoot},
    Response, Sense, Ui,
};

#[derive(Default)]
pub struct ContextMenuSystem {
    root: Option<MenuRoot>,
}

impl ContextMenuSystem {
    /// Sense if a context menu needs to be (re-)created or destroyed
    fn sense_click(&mut self, response: &Response) -> MenuResponse {
        let response = response.interact(Sense::click());
        let pointer = &response.ctx.input().pointer;
        if pointer.any_pressed() {
            if let Some(pos) = pointer.interact_pos() {
                let mut destroy = false;
                let mut in_old_menu = false;
                if let Some(root) = &mut self.root {
                    let menu_state = root.menu_state.read().unwrap();
                    in_old_menu = menu_state.area_contains(pos);
                    destroy = root.id == response.id;
                }
                if !in_old_menu {
                    let in_target = response.rect.contains(pos);
                    if in_target && pointer.secondary_down() {
                        return MenuResponse::Create(pos);
                    } else if (in_target && pointer.primary_down()) || destroy {
                        return MenuResponse::Close;
                    }
                }
            }
        }
        MenuResponse::Stay
    }
    /// Show context menu root.
    fn show(&mut self, response: &Response, add_contents: impl FnOnce(&mut Ui)) -> MenuResponse {
        if let Some(root) = &mut self.root {
            if root.id == response.id {
                let inner_response = root.show(&response.ctx, add_contents);
                let mut menu_state = root.menu_state.write().unwrap();
                menu_state.rect = inner_response.response.rect;

                if menu_state.response.is_close() {
                    return MenuResponse::Close;
                }
            }
        }
        MenuResponse::Stay
    }
    /// Show a menu if right-clicked
    /// Should be called from [`Context`] on a [`Response`]
    pub fn context_menu(&mut self, response: &Response, add_contents: impl FnOnce(&mut Ui)) {
        match self.sense_click(response) {
            MenuResponse::Create(pos) => {
                self.root = Some(MenuRoot::new(pos, response.id));
            }
            MenuResponse::Close => self.root = None,
            MenuResponse::Stay => {}
        };
        if let MenuResponse::Close = self.show(response, add_contents) {
            self.root = None
        }
    }
}
