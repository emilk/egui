use crate::{Frame, Id, Image, ImageSource, Response, Sense, TextStyle, Ui, Widget, WidgetText};
use ahash::HashMap;
use emath::{Align2, Rect, Vec2};
use epaint::{Color32, Galley};
use std::sync::Arc;

pub enum SizedAtomicKind<'a> {
    Text(Arc<Galley>),
    Image(Image<'a>, Vec2),
    Custom(Id, Vec2),
    Grow,
}

impl SizedAtomicKind<'_> {
    pub fn size(&self) -> Vec2 {
        match self {
            SizedAtomicKind::Text(galley) => galley.size(),
            SizedAtomicKind::Image(_, size) => *size,
            SizedAtomicKind::Custom(_, size) => *size,
            SizedAtomicKind::Grow => Vec2::ZERO,
        }
    }
}

/// AtomicLayout
pub struct WidgetLayout<'a> {
    pub atomics: Atomics<'a>,
    gap: Option<f32>,
    pub(crate) frame: Frame,
    pub(crate) sense: Sense,
    fallback_text_color: Option<Color32>,
}

impl<'a> WidgetLayout<'a> {
    pub fn new(atomics: impl IntoAtomics<'a>) -> Self {
        Self {
            atomics: atomics.into_atomics(),
            gap: None,
            frame: Frame::default(),
            sense: Sense::hover(),
            fallback_text_color: None,
        }
    }

    pub fn add(mut self, atomic: impl Into<Atomic<'a>>) -> Self {
        self.atomics.add(atomic.into());
        self
    }

    /// Default: `Spacing::icon_spacing`
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = Some(gap);
        self
    }

    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = frame;
        self
    }

    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    pub fn fallback_text_color(mut self, color: Color32) -> Self {
        self.fallback_text_color = Some(color);
        self
    }

    pub fn show(self, ui: &mut Ui) -> AtomicLayoutResponse {
        let fallback_text_color = self
            .fallback_text_color
            .unwrap_or_else(|| ui.style().visuals.text_color());
        let gap = self.gap.unwrap_or(ui.spacing().icon_spacing);

        let available_size = ui.available_size();
        let available_width = available_size.x;

        let mut desired_width = 0.0;
        let mut preferred_width = 0.0;

        let mut height: f32 = 0.0;

        let mut sized_items = Vec::new();

        let mut grow_count = 0;

        for (item) in self.atomics.0 {
            let (preferred_size, sized) = match item.kind {
                AtomicKind::Text(text) => {
                    let galley = text.into_galley(ui, None, available_width, TextStyle::Button);
                    (
                        galley.size(), // TODO
                        SizedAtomicKind::Text(galley),
                    )
                }
                AtomicKind::Image(image) => {
                    let size =
                        image.load_and_calc_size(ui, Vec2::min(available_size, Vec2::splat(16.0)));
                    let size = size.unwrap_or_default();
                    (size, SizedAtomicKind::Image(image, size))
                }
                AtomicKind::Custom(id, size) => (size, SizedAtomicKind::Custom(id, size)),
                AtomicKind::Grow => {
                    grow_count += 1;
                    (Vec2::ZERO, SizedAtomicKind::Grow)
                }
            };
            let size = sized.size();

            desired_width += size.x;
            preferred_width += preferred_size.x;

            height = height.max(size.y);

            sized_items.push(sized);
        }

        if sized_items.len() > 1 {
            let gap_space = gap * (sized_items.len() as f32 - 1.0);
            desired_width += gap_space;
            preferred_width += gap_space;
        }

        let margin = self.frame.total_margin();
        let content_size = Vec2::new(desired_width, height);
        let frame_size = content_size + margin.sum();

        let (rect, response) = ui.allocate_at_least(frame_size, self.sense);

        let mut response = AtomicLayoutResponse {
            response,
            custom_rects: HashMap::default(),
        };

        let content_rect = rect - margin;
        ui.painter().add(self.frame.paint(content_rect));

        let width_to_fill = content_rect.width();
        let extra_space = f32::max(width_to_fill - desired_width, 0.0);
        let grow_width = f32::max(extra_space / grow_count as f32, 0.0);

        let mut cursor = content_rect.left();

        for sized in sized_items {
            let size = sized.size();
            let width = match sized {
                SizedAtomicKind::Grow => grow_width,
                _ => size.x,
            };

            let frame = content_rect.with_min_x(cursor).with_max_x(cursor + width);
            cursor = frame.right() + gap;

            let align = Align2::CENTER_CENTER;
            let rect = align.align_size_within_rect(size, frame);

            match sized {
                SizedAtomicKind::Text(galley) => {
                    ui.painter().galley(rect.min, galley, fallback_text_color);
                }
                SizedAtomicKind::Image(image, _) => {
                    image.paint_at(ui, rect);
                }
                SizedAtomicKind::Custom(id, size) => {
                    response.custom_rects.insert(id, rect);
                }
                SizedAtomicKind::Grow => {}
            }
        }

        response
    }
}

pub struct AtomicLayoutResponse {
    pub response: Response,
    pub custom_rects: HashMap<Id, Rect>,
}

// pub struct WLButton<'a> {
//     wl: WidgetLayout<'a>,
// }
//
// impl<'a> WLButton<'a> {
//     pub fn new(text: impl Into<WidgetText>) -> Self {
//         Self {
//             wl: WidgetLayout::new()
//                 .sense(Sense::click())
//                 .add(Item::default(), WidgetLayoutItemType::Text(text.into())),
//         }
//     }
//
//     pub fn image(image: impl Into<Image<'a>>) -> Self {
//         Self {
//             wl: WidgetLayout::new().sense(Sense::click()).add(
//                 Item::default(),
//                 WidgetLayoutItemType::Image(image.into().max_size(Vec2::splat(16.0))),
//             ),
//         }
//     }
//
//     pub fn image_and_text(image: impl Into<Image<'a>>, text: impl Into<WidgetText>) -> Self {
//         Self {
//             wl: WidgetLayout::new()
//                 .sense(Sense::click())
//                 .add(Item::default(), WidgetLayoutItemType::Image(image.into()))
//                 .add(Item::default(), WidgetLayoutItemType::Text(text.into())),
//         }
//     }
//
//     pub fn right_text(mut self, text: impl Into<WidgetText>) -> Self {
//         self.wl = self
//             .wl
//             .add(Item::default(), WidgetLayoutItemType::Grow)
//             .add(Item::default(), WidgetLayoutItemType::Text(text.into()));
//         self
//     }
// }
//
// impl<'a> Widget for WLButton<'a> {
//     fn ui(mut self, ui: &mut Ui) -> Response {
//         let response = ui.ctx().read_response(ui.next_auto_id());
//
//         let visuals = response.map_or(&ui.style().visuals.widgets.inactive, |response| {
//             ui.style().interact(&response)
//         });
//
//         self.wl.frame = self
//             .wl
//             .frame
//             .inner_margin(ui.style().spacing.button_padding)
//             .fill(visuals.bg_fill)
//             .stroke(visuals.bg_stroke)
//             .corner_radius(visuals.corner_radius);
//
//         self.wl.show(ui)
//     }
// }

pub enum AtomicKind<'a> {
    Text(WidgetText),
    Image(Image<'a>),
    Custom(Id, Vec2),
    Grow,
}

pub struct Atomic<'a> {
    size: Option<Vec2>,
    grow: bool,
    pub kind: AtomicKind<'a>,
}

pub fn a<'a>(i: impl Into<AtomicKind<'a>>) -> Atomic<'a> {
    Atomic {
        size: None,
        grow: false,
        kind: i.into(),
    }
}

impl Atomic<'_> {
    // pub fn size(mut self, size: Vec2) -> Self {
    //     self.size = Some(size);
    //     self
    // }
    //
    // pub fn grow(mut self, grow: bool) -> Self {
    //     self.grow = grow;
    //     self
    // }
}

pub trait AtomicExt<'a> {
    fn a_size(self, size: Vec2) -> Atomic<'a>;
    fn a_grow(self, grow: bool) -> Atomic<'a>;
}

impl<'a, T> AtomicExt<'a> for T
where
    T: Into<Atomic<'a>> + Sized,
{
    fn a_size(self, size: Vec2) -> Atomic<'a> {
        let mut atomic = self.into();
        atomic.size = Some(size);
        atomic
    }

    fn a_grow(self, grow: bool) -> Atomic<'a> {
        let mut atomic = self.into();
        atomic.grow = grow;
        atomic
    }
}

impl<'a, T> From<T> for Atomic<'a>
where
    T: Into<AtomicKind<'a>>,
{
    fn from(value: T) -> Self {
        Atomic {
            size: None,
            grow: false,
            kind: value.into(),
        }
    }
}

impl<'a> From<Image<'a>> for AtomicKind<'a> {
    fn from(value: Image<'a>) -> Self {
        AtomicKind::Image(value)
    }
}

// impl<'a> From<&str> for AtomicKind<'a> {
//     fn from(value: &str) -> Self {
//         AtomicKind::Text(value.into())
//     }
// }

impl<'a, T> From<T> for AtomicKind<'a>
where
    T: Into<WidgetText>,
{
    fn from(value: T) -> Self {
        AtomicKind::Text(value.into())
    }
}

pub struct Atomics<'a>(Vec<Atomic<'a>>);

impl<'a> Atomics<'a> {
    pub fn add(&mut self, atomic: impl Into<Atomic<'a>>) {
        self.0.push(atomic.into());
    }

    pub fn add_front(&mut self, atomic: impl Into<Atomic<'a>>) {
        self.0.insert(0, atomic.into());
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Atomic<'a>> {
        self.0.iter_mut()
    }

    pub fn text(&self) -> Option<String> {
        let mut string: Option<String> = None;
        for atomic in &self.0 {
            if let AtomicKind::Text(text) = &atomic.kind {
                if let Some(string) = &mut string {
                    string.push(' ');
                    string.push_str(text.text());
                } else {
                    string = Some(text.text().to_owned());
                }
            }
        }
        string
    }
}

impl<'a, T> IntoAtomics<'a> for T
where
    T: Into<Atomic<'a>>,
{
    fn collect(self, atomics: &mut Atomics<'a>) {
        atomics.add(self);
    }
}

pub trait IntoAtomics<'a> {
    fn collect(self, atomics: &mut Atomics<'a>);

    fn into_atomics(self) -> Atomics<'a>
    where
        Self: Sized,
    {
        let mut atomics = Atomics(Vec::new());
        self.collect(&mut atomics);
        atomics
    }
}

impl<'a> IntoAtomics<'a> for Atomics<'a> {
    fn collect(self, atomics: &mut Atomics<'a>) {
        atomics.0.extend(self.0);
    }
}

macro_rules! all_the_atomics {
    ($($T:ident),*) => {
        impl<'a, $($T),*> IntoAtomics<'a> for ($($T),*)
        where
            $($T: IntoAtomics<'a>),*
        {
            fn collect(self, atomics: &mut Atomics<'a>) {
                #[allow(non_snake_case)]
                let ($($T),*) = self;
                $($T.collect(atomics);)*
            }
        }
    };
}

all_the_atomics!();
all_the_atomics!(T0, T1);
all_the_atomics!(T0, T1, T2);
all_the_atomics!(T0, T1, T2, T3);
all_the_atomics!(T0, T1, T2, T3, T4);
all_the_atomics!(T0, T1, T2, T3, T4, T5);

// trait AtomicWidget {
//     fn show(&self, ui: &mut Ui) -> WidgetLayout;
// }

// TODO: This conflicts with the FnOnce Widget impl, is there some way around that?
// impl<T> Widget for T where T: AtomicWidget {
//     fn ui(self, ui: &mut Ui) -> Response {
//         ui.add(self)
//     }
// }

impl Widget for WidgetLayout<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}
