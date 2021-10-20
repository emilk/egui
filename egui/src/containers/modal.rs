//! Show modal dialog.


use crate::*;

// ----------------------------------------------------------------------------


/// Common modal state 
/// 
/// > A modal dialog is a dialog that appears on top of the main content and moves the system into a special mode requiring user interaction. This dialog disables the main content until the user explicitly interacts with the modal dialog.  
/// > – [Modal & Nonmodal Dialogs: When (& When Not) to Use Them](https://www.nngroup.com/articles/modal-nonmodal-dialog/)
/// For this implementation, the above suggests copying the common state approach from [MonoState]
#[derive(Clone, Debug, Default)]
pub(crate) struct ModalMonoState {
    /// The optional id the modal took focus from 
    previous_focused_id_opt: Option<Id>,
    /// The id of the last modal shown to enforce modality 
    last_modal_id_opt: Option<Id>,
}

impl ModalMonoState {
  //  the modal showing? 
  pub fn is_modal_showing(&self) -> bool { self.last_modal_id_opt.is_some() }  
}

// ----------------------------------------------------------------------------

/// Show a modal dialog that intercepts interaction with other ui elements whilst visible.
///
/// - Clicking away from the caller-provided modal ui optionally dismisses modal.
/// - The modal can also be dismissed using a custom close key (the default is [Key::Esc])
/// - To dismiss the modal manually, call the [relinquish_modal] function. 
/// - Returns `None` if a modal is already showing.
///
/// ```
/// # let mut ctx = egui::CtxRef::default();
/// # ctx.begin_frame(Default::default());
/// # let ctx = &ctx;
///   let id = egui::Id::new("my_first_modal");
///   let r_opt =  egui::modal::show_modal(
///       ctx, 
///       id,
///       true,
///       None, 
///       None, 
///       |ui| {
///           ui.label("This is a modal dialog");
///   });
///   assert_eq!(r_opt, Some(()));
///   let id = egui::Id::new("my_attempted_second_modal");
///   let r_opt = egui::modal::show_modal(
///       ctx, 
///       id,
///       true,
///       None, 
///       None, 
///       |ui| {
///           ui.label("This wants to be a modal dialog");
///   });
///   assert_eq!(r_opt, None);
///   egui::modal::relinquish_modal(ctx);
/// 
/// ```
pub fn show_modal<R>(
    ctx: &CtxRef, 
    id: Id,
    click_away_dismisses: bool,
    background_color_opt: Option<Color32>,
    close_key_opt: Option<Key>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    use containers::*;
    let memory = &mut ctx.memory();
    // Clone some context state
    let previous_focused_id_opt: Option<Id> = memory.focus();
    let modal_mono_state_ref = memory
        .data_temp
        .get_mut_or_default::<ModalMonoState>();
    // Enforce modality
    let is_modal_showing = modal_mono_state_ref.is_modal_showing();
    let have_modal_control = !is_modal_showing
      || (is_modal_showing && modal_mono_state_ref.last_modal_id_opt == Some(id));
    if have_modal_control{
        modal_mono_state_ref
            .last_modal_id_opt.replace(id);
        modal_mono_state_ref
            .previous_focused_id_opt = previous_focused_id_opt.clone();
        drop(modal_mono_state_ref);
        drop(memory);
        // show the modal taking up the whole screen 
        let InnerResponse {
          inner, mut response, ..
        } = Area::new(id)
            .interactable(true)
            .fixed_pos(Pos2::ZERO)
            .order(Order::Foreground)
            .show(ctx, |ui| {
                let background_color = background_color_opt
                  .unwrap_or(Color32::from_rgba_unmultiplied(
                      0, 0, 0, 144
                  ));
                ui
                    .painter()
                    .add(Shape::rect_filled(
                        ui.ctx().input().screen_rect,
                        0.0,
                        background_color
                    ));
                // user-provided contents
                add_contents(ui)
            }); 
        response = response.interact(Sense::click());
        let close_key = close_key_opt.unwrap_or(Key::Escape);
        if (response.clicked() && click_away_dismisses) 
            || ctx.input().key_pressed(close_key) 
        {
            relinquish_modal(ctx);
        }
        Some(inner) 
    } else {
      None
    }
}

/// Relinquish control of the modal. If the modal is showing, this must be called to show a new modal.
/// 
/// - Does nothing if the modal was not showing.
/// - If some id-bearing widget was previously focused, this returns the id. 
pub fn relinquish_modal(ctx: &CtxRef) -> Option<Id> {
    let mut memory = ctx.memory();
    let modal_mono_state_ref = memory
        .data_temp
        .get_mut_or_default::<ModalMonoState>();
    // try to determine whether the modal can be shown 
    let have_modal_control = modal_mono_state_ref
        .is_modal_showing();
    have_modal_control.then(|| {
        // modal control has been obtained 
        let previous_focused_id_opt = modal_mono_state_ref
            .previous_focused_id_opt
            .take();
        let _  = modal_mono_state_ref
            .last_modal_id_opt
            .take();
        previous_focused_id_opt
    }).flatten()
}