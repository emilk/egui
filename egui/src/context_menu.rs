use super::{
    menu::{MenuResponse, MenuRoot},
    Response, Ui,
};

#[derive(Default)]
pub struct ContextMenuSystem {
    root: Option<MenuRoot>,
}

impl ContextMenuSystem {
    /// Show context menu root at element response.
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
    /// Show a menu at pointer if right-clicked response.
    /// Should be called from [`Context`] on a [`Response`]
    pub fn context_menu(&mut self, response: &Response, add_contents: impl FnOnce(&mut Ui)) {
        MenuRoot::context_click_interaction(response, &mut self.root, response.id);
        if let MenuResponse::Close = self.show(response, add_contents) {
            self.root = None
        }
    }
}
