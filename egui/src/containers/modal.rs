//! Show modal dialog.


use crate::*;

// ----------------------------------------------------------------------------


/// Common modal state 
/// 
/// > A modal dialog is a dialog that appears on top of the main content and moves the system into a special mode requiring user interaction. This dialog disables the main content until the user explicitly interacts with the modal dialog.  
/// > – [Modal & Nonmodal Dialogs: When (& When Not) to Use Them](https://www.nngroup.com/articles/modal-nonmodal-dialog/)
/// For this implementation, the above suggests copying the common state approach from [`MonoState`]
#[derive(Clone, Debug, Default)]
pub(crate) struct ModalMonoState {
    /// The optional id the modal took focus from 
    previous_focused_id_opt: Option<Id>,
    /// The id of the last modal shown to enforce modality 
    last_modal_id_opt: Option<Id>,
}

impl ModalMonoState {
  /// The id source of the default modal
  pub const DEFAULT_MODAL_ID_SOURCE: &'static str = "__default_modal";

  /// Construct an id for the default modal 
  pub fn get_default_modal_id() -> Id {
      Id::new(Self::DEFAULT_MODAL_ID_SOURCE)
  }
  /// Construct an interceptor color  for the default modal 
  pub fn get_default_modal_interceptor_color() -> Color32 {
      Color32::from_rgba_unmultiplied(
        0, 0, 0, 144
      )
  }
}

// ----------------------------------------------------------------------------

/// Relinquish control of the modal. If the modal is showing, this must be called to show a new modal.
/// 
/// - No id is required to relinquish modal control – to prevent the application from entering a state where it's irrevocably stuck in a mode. In other words, the application must therefore track/manage when/whether it's allowable to relinquish control.
/// - Does nothing if the modal was not showing.
/// - If some id-bearing widget was previously focused, this returns the id. 
pub fn relinquish_modal(ctx: &CtxRef) -> Option<Id> {
    let last_modal_id_opt: Option<Id> = ctx.memory()
        .data_temp
        .get_or_default::<ModalMonoState>()
        .last_modal_id_opt;
    
    ctx.memory()
        .data_temp
        .get_mut_or_default::<ModalMonoState>();
    // try to determine whether the modal can be shown 
    let is_modal_controlled = last_modal_id_opt.is_some();
    is_modal_controlled.then(|| {
        // modal control has been obtained 
        let previous_focused_id_opt: Option<Id> = ctx.memory()
            .data_temp
            .get_mut_or_default::<ModalMonoState>()
            .previous_focused_id_opt
            .take();
        let _  = ctx.memory()
            .data_temp
            .get_mut_or_default::<ModalMonoState>()
            .last_modal_id_opt
            .take();
        previous_focused_id_opt
    }).flatten()
}
/// Show a modal dialog that intercepts interaction with other ui elements whilst visible.
///
/// - The returned inner response includes the result of the provided contents ui function as well as the response from clicking the interaction interceptor.
/// - Returns `None` if a modal is already showing.
///
/// ```
/// # let mut ctx = egui::CtxRef::default();
/// # ctx.begin_frame(Default::default());
/// # let ctx = &ctx;
///   let id_0 = egui::Id::new("my_0th_modal");
///   let id_1 = egui::Id::new("my_1st_modal");
///   let r_opt =  egui::modal::show_custom_modal(
///       ctx, 
///       id_0,
///       None, 
///       |ui| {
///           ui.label("This is a modal dialog");
///   });
///   assert_eq!(r_opt, Some(()), "A modal dialog with an id may show once");
///   let r_opt =  egui::modal::show_custom_modal(
///       ctx, 
///       id_0,
///       None, 
///       |ui| {
///           ui.label("This is the same (by id) modal dialog");
///   });
///    assert_eq!(r_opt, Some(()), "A modal dialog with an id may show again/update");
///   let r_opt = egui::modal::show_custom_modal(
///       ctx, 
///       id_1,
///       None, 
///       |ui| {
///           ui.label("This wants to be a modal dialog, yet shall produce nary a ui ere the grotesque and catastrophic violation of some invariant. ");
///   });
///   assert_eq!(r_opt, None, "A modal dialog may not appear whilst another has control");
///   egui::modal::relinquish_modal(ctx);
/// let r_opt = egui::modal::show_custom_modal(
///       ctx, 
///       id_1,
///       None, 
///       |ui| {
///           ui.label("This wants to be a modal dialog, and its dreams are fulfilled.");
///   });
///   assert_eq!(r_opt, Some(()), "A modal dialog may appear after another has relinquished control");
/// ```
pub fn show_custom_modal<R>(
    ctx: &CtxRef, 
    id: Id,
    background_color_opt: Option<Color32>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    use containers::*;
    // Clone some context state
    let previous_focused_id_opt: Option<Id> = ctx.memory().focus();
    let last_modal_id_opt = ctx.memory()
        .data_temp
        .get_or_default::<ModalMonoState>()
        .last_modal_id_opt;
    
    // Enforce modality
    let have_modal_control = last_modal_id_opt.is_none() 
        || last_modal_id_opt == Some(id)
        || last_modal_id_opt == Some(ModalMonoState::get_default_modal_id());
    if have_modal_control{
        ctx.memory()
            .data_temp
            .get_mut_or_default::<ModalMonoState>()
            .last_modal_id_opt.replace(id);
        ctx.memory()
            .data_temp
            .get_mut_or_default::<ModalMonoState>()
            .previous_focused_id_opt = previous_focused_id_opt;
        // show the modal taking up the whole screen 
        let InnerResponse {
          inner, ..
        } = Area::new(id)
            .interactable(true)
            .fixed_pos(Pos2::ZERO)
            // .order(Order::Foreground)
            .show(ctx, |ui| {
                let background_color = background_color_opt
                    .unwrap_or_else(ModalMonoState::get_default_modal_interceptor_color);
                let interceptor_rect = ui.ctx().input().screen_rect(); 
                // create an empty interaction interceptor 
                // for some reason, using Sense::click() instead of Sense::hover()
                // seems to intercept not only clicks to the unoccupied areas but also to the user-provided ui               
                ui.allocate_response(
                    interceptor_rect.size(), 
                    Sense::hover() 
                );
                let InnerResponse{
                    inner: user_ui_inner, ..
                } = ui.allocate_ui_at_rect(
                    interceptor_rect,
                    |ui| {
                        // create a customizable visual indicator signifying to the user that this is a modal mode
                        ui
                            .painter()
                            .add(Shape::rect_filled(
                                interceptor_rect,
                                0.0,
                                background_color
                            ));
                        add_contents(ui)
                    }
                );
                user_ui_inner
            }); 
        Some(inner) 
    } else {
      None
    }
}
