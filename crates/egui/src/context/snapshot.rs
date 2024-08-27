use ahash::{HashMap, HashMapExt};
use emath::TSTransform;
use epaint::text::FontDefinitions;
use serde::ser::{SerializeMap, SerializeSeq, SerializeTuple};

use crate::style::Style;
use crate::text_selection::LabelSelectionState;
use crate::{AreaState, Options, PlatformOutput, ViewportCommand};
use std::marker::PhantomData;
use std::sync::Arc;

use super::frame_state::FrameState;
use super::hit_test::WidgetHits;
use super::interaction::InteractionSnapshot;
use super::layers::{GraphicLayers, PaintList};
use super::memory::{Areas, Focus, InteractionState};
use super::{
    Id, IdMap, InputState, LayerId, Memory, Order, ViewportBuilder, ViewportClass, ViewportId,
    ViewportIdMap, ViewportState,
};

/// Tracks changes that occur to a [`Context`](super::Context) so that a
/// partial [`ContextSnapshot`] can be generated containing only information
/// that has changed since the previous snapshot was applied.
///
/// The [`ContextSnapshotDeltas::default()`] implementation returns an object
/// that will cause a full synchronization - the generated [`ContextSnapshot`]
/// will contain all of the context's data.
#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub struct ContextSnapshotDeltas {
    /// The number of times that the font definitions have changed.
    pub(super) font_definitions_count: u64,

    /// The number of frames that have elapsed.
    pub(super) frame_count: u64,

    /// The number of times that the style has changed.
    pub(super) style_count: u64,
}

impl Default for ContextSnapshotDeltas {
    fn default() -> Self {
        Self {
            font_definitions_count: u64::MAX,
            frame_count: u64::MAX,
            style_count: u64::MAX,
        }
    }
}

/// A borrowed version of a `ContextSnapshot`. May be serialized and
/// then deserialized to get an owned `ContextSnapshot`.
pub struct ContextSnapshotBorrow<'a> {
    /// The deltas describing the current context state.
    pub(super) deltas: &'a ContextSnapshotDeltas,

    /// The `ContextImpl::font_definitions` field.
    pub(super) font_definitions: Option<&'a FontDefinitions>,

    /// The `ContextImpl::memory` field.
    pub(super) memory: &'a Memory,

    /// The context's style.
    pub(super) style: Option<Arc<Style>>,

    /// The `ContextImpl::new_zoom_factor` field.
    pub(super) new_zoom_factor: &'a Option<f32>,

    /// The `ContextImpl::last_viewport` field.
    pub(super) last_viewport: &'a ViewportId,

    /// The `ContextImpl::viewports` field.
    pub(super) viewports: &'a ViewportIdMap<ViewportState>,
}

/// Holds the instantaneous state of a `Context`. May be used to synchronize
/// state between two separate contexts.
#[derive(Clone)]
pub struct ContextSnapshot {
    /// The deltas describing the current context state.
    pub(super) deltas: ContextSnapshotDeltas,

    /// The `ContextImpl::font_definitions` field.
    pub(super) font_definitions: Option<FontDefinitions>,

    /// The `ContextImpl::memory` field.
    pub(super) memory: MemorySnapshot,

    /// The `Memory::options` field.
    pub(super) options: OptionsSnapshot,

    /// The context's style.
    pub(super) style: Option<Arc<Style>>,

    /// The `ContextImpl::new_zoom_factor` field.
    pub(super) new_zoom_factor: Option<f32>,

    /// The `ContextImpl::last_viewport` field.
    pub(super) last_viewport: ViewportId,

    /// The `ContextImpl::viewports` field.
    pub(super) viewports: ViewportIdMap<ViewportStateSnapshot>,
}

impl ContextSnapshot {
    /// The number of fields that this struct has.
    const FIELDS: usize = 8;
}

/// Holds the instantaneous state of a `Memory` for synchronizing
/// between two separate contexts.
#[derive(Clone)]
pub(super) struct MemorySnapshot {
    /// The `LabelSelectionState` object stored in `Memory::data`
    pub label_selection_state: LabelSelectionState,

    /// The `Memory::new_font_definitions` field.
    pub new_font_definitions: Option<epaint::text::FontDefinitions>,

    /// The `Memory::viewport_id` field.
    pub viewport_id: ViewportId,

    /// The `Memory::popup` field.
    pub popup: Option<Id>,

    /// The `Memory::everything_is_visible` field.
    pub everything_is_visible: bool,

    /// The `Memory::layer_transforms` field.
    pub layer_transforms: HashMap<LayerId, TSTransform>,

    /// The `Memory::areas` field.
    pub areas: ViewportIdMap<Areas>,

    /// The `Memory::interactions` field.
    pub interactions: ViewportIdMap<InteractionState>,

    /// The `Memory::focus` field.
    pub focus: ViewportIdMap<Focus>,
}

impl MemorySnapshot {
    /// The number of fields that this struct has.
    const FIELDS: usize = 9;
}

/// Holds the instantaneous state of an `Options` for synchronizing
/// between two separate contexts.
#[derive(Clone, Copy, serde::Deserialize)]
pub struct OptionsSnapshot {
    /// The `Options::everything_is_visible` field.
    pub zoom_factor: f32,

    /// The `Options::zoom_with_keyboard` field.
    pub zoom_with_keyboard: bool,

    /// The `Options::tessellation_options` field.
    pub tessellation_options: epaint::TessellationOptions,

    /// The `Options::repaint_on_widget_change` field.
    pub repaint_on_widget_change: bool,

    /// The `Options::screen_reader` field.
    pub screen_reader: bool,

    /// The `Options::preload_font_glyphs` field.
    pub preload_font_glyphs: bool,

    /// The `Options::warn_on_id_clash` field.
    pub warn_on_id_clash: bool,

    /// The `Options::line_scroll_speed` field.
    pub line_scroll_speed: f32,

    /// The `Options::scroll_zoom_speed` field.
    pub scroll_zoom_speed: f32,

    /// The `Options::reduce_texture_memory` field.
    pub reduce_texture_memory: bool,
}

impl OptionsSnapshot {
    /// The number of fields that this struct has.
    const FIELDS: usize = 10;
}

/// A serialized version of `epaint::text::TextWrapping`
/// which ensures that `max_rows` never overflows
/// when serializing across 32-bit or 64-bit architectures.
#[derive(serde::Deserialize, serde::Serialize)]
struct TextWrappingSnapshot {
    /// The `TextWrapping::max_width` field.
    pub max_width: f32,

    /// The `TextWrapping::max_rows` field.
    pub max_rows: u64,

    /// The `TextWrapping::break_anywhere` field.
    pub break_anywhere: bool,

    /// The `TextWrapping::overflow_character` field.
    pub overflow_character: Option<char>,
}

impl From<epaint::text::TextWrapping> for TextWrappingSnapshot {
    fn from(value: epaint::text::TextWrapping) -> Self {
        Self {
            max_width: value.max_width,
            max_rows: value.max_rows as u64,
            break_anywhere: value.break_anywhere,
            overflow_character: value.overflow_character,
        }
    }
}

impl From<TextWrappingSnapshot> for epaint::text::TextWrapping {
    fn from(value: TextWrappingSnapshot) -> Self {
        Self {
            max_width: value.max_width,
            max_rows: value.max_rows as usize,
            break_anywhere: value.break_anywhere,
            overflow_character: value.overflow_character,
        }
    }
}

/// Holds the instantaneous state of a `ViewportState` for synchronizing
/// between two separate contexts.
#[derive(Clone, Default)]
pub(super) struct ViewportStateSnapshot {
    /// The `ViewportState::class` field.
    pub class: ViewportClass,

    /// The `ViewportState::builder` field.
    pub builder: ViewportBuilder,

    /// The `ViewportState::input` field.
    pub input: InputState,

    /// The `ViewportState::this_frame` field.
    pub this_frame: FrameState,

    /// The `ViewportState::prev_frame` field.
    pub prev_frame: FrameState,

    /// The `ViewportState::used` field.
    pub used: bool,

    /// The `ViewportState::hits` field.
    pub hits: WidgetHits,

    /// The `ViewportState::interact_widgets` field.
    pub interact_widgets: InteractionSnapshot,

    /// The `ViewportState::graphics` field.
    pub graphics: GraphicLayers,

    /// The `ViewportState::output` field.
    pub output: PlatformOutput,

    /// The `ViewportState::commands` field.
    pub commands: Vec<ViewportCommand>,
}

impl ViewportStateSnapshot {
    /// The number of fields that this struct has.
    const FIELDS: usize = 11;
}

impl<'a> serde::Serialize for ContextSnapshotBorrow<'a> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_tuple(ContextSnapshot::FIELDS)?;
        seq.serialize_element(self.deltas)?;
        seq.serialize_element(&self.font_definitions)?;
        seq.serialize_element(&SnapshotSerialize(self.memory))?;
        seq.serialize_element(&SnapshotSerialize(&self.memory.options))?;
        seq.serialize_element(&self.style)?;
        seq.serialize_element(&self.new_zoom_factor)?;
        seq.serialize_element(&self.last_viewport)?;
        seq.serialize_element(&SnapshotSerialize(self.viewports))?;
        seq.end()
    }
}

/// Implements custom, snapshot-specific serialization logic for type `T`.
pub struct SnapshotSerialize<'a, T>(&'a T);

impl<'a> serde::Serialize for SnapshotSerialize<'a, Memory> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serialize_tuple = serializer.serialize_tuple(MemorySnapshot::FIELDS)?;
        serialize_tuple.serialize_element(
            &self
                .0
                .data
                .get_temp::<LabelSelectionState>(Id::new(self.0.viewport_id))
                .unwrap_or_default(),
        )?;
        serialize_tuple.serialize_element(&self.0.new_font_definitions)?;
        serialize_tuple.serialize_element(&self.0.viewport_id)?;
        serialize_tuple.serialize_element(&self.0.popup)?;
        serialize_tuple.serialize_element(&self.0.everything_is_visible)?;
        serialize_tuple.serialize_element(&self.0.layer_transforms)?;
        serialize_tuple.serialize_element(&SnapshotSerialize(&self.0.areas))?;
        serialize_tuple.serialize_element(&self.0.interactions)?;
        serialize_tuple.serialize_element(&self.0.focus)?;
        serialize_tuple.end()
    }
}

impl<'a> serde::Serialize for SnapshotSerialize<'a, Options> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serialize_tuple = serializer.serialize_tuple(OptionsSnapshot::FIELDS)?;
        serialize_tuple.serialize_element(&self.0.zoom_factor)?;
        serialize_tuple.serialize_element(&self.0.zoom_with_keyboard)?;
        serialize_tuple.serialize_element(&self.0.tessellation_options)?;
        serialize_tuple.serialize_element(&self.0.repaint_on_widget_change)?;
        serialize_tuple.serialize_element(&self.0.screen_reader)?;
        serialize_tuple.serialize_element(&self.0.preload_font_glyphs)?;
        serialize_tuple.serialize_element(&self.0.warn_on_id_clash)?;
        serialize_tuple.serialize_element(&self.0.line_scroll_speed)?;
        serialize_tuple.serialize_element(&self.0.scroll_zoom_speed)?;
        serialize_tuple.serialize_element(&self.0.reduce_texture_memory)?;
        serialize_tuple.end()
    }
}

impl<'a> serde::Serialize for SnapshotSerialize<'a, ViewportIdMap<Areas>> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serialize_map = serializer.serialize_map(Some(self.0.len()))?;
        for (id, state) in self.0 {
            serialize_map.serialize_entry(id, &SnapshotSerialize(state))?;
        }
        serialize_map.end()
    }
}

impl<'a> serde::Serialize for SnapshotSerialize<'a, Areas> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serialize_tuple = serializer.serialize_tuple(6)?;
        serialize_tuple.serialize_element(&SnapshotSerialize(&self.0.areas))?;
        serialize_tuple.serialize_element(&self.0.order)?;
        serialize_tuple.serialize_element(&self.0.visible_current_frame)?;
        serialize_tuple.serialize_element(&self.0.visible_last_frame)?;
        serialize_tuple.serialize_element(&self.0.wants_to_be_on_top)?;
        serialize_tuple.serialize_element(&self.0.sublayers)?;
        serialize_tuple.end()
    }
}

impl<'a> serde::Serialize for SnapshotSerialize<'a, IdMap<AreaState>> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serialize_map = serializer.serialize_map(Some(self.0.len()))?;
        for (id, state) in self.0 {
            serialize_map.serialize_entry(id, &SnapshotSerialize(state))?;
        }
        serialize_map.end()
    }
}

impl<'a> serde::Serialize for SnapshotSerialize<'a, AreaState> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serialize_tuple = serializer.serialize_tuple(5)?;
        serialize_tuple.serialize_element(&self.0.pivot_pos)?;
        serialize_tuple.serialize_element(&self.0.pivot)?;
        serialize_tuple.serialize_element(&self.0.size)?;
        serialize_tuple.serialize_element(&self.0.interactable)?;
        serialize_tuple.serialize_element(&self.0.last_became_visible_at)?;
        serialize_tuple.end()
    }
}

impl<'a> serde::Serialize for SnapshotSerialize<'a, ViewportState> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serialize_tuple = serializer.serialize_tuple(ViewportStateSnapshot::FIELDS)?;
        serialize_tuple.serialize_element(&self.0.class)?;
        serialize_tuple.serialize_element(&self.0.builder)?;
        serialize_tuple.serialize_element(&self.0.input)?;
        serialize_tuple.serialize_element(&self.0.this_frame)?;
        serialize_tuple.serialize_element(&self.0.prev_frame)?;
        serialize_tuple.serialize_element(&self.0.used)?;
        serialize_tuple.serialize_element(&self.0.hits)?;
        serialize_tuple.serialize_element(&self.0.interact_widgets)?;
        serialize_tuple.serialize_element(&SnapshotSerialize(&self.0.graphics))?;
        serialize_tuple.serialize_element(&self.0.output)?;
        serialize_tuple.serialize_element(&self.0.commands)?;
        serialize_tuple.end()
    }
}

impl<'a> serde::Serialize for SnapshotSerialize<'a, ViewportIdMap<ViewportState>> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serialize_map = serializer.serialize_map(Some(self.0.len()))?;
        for (id, state) in self.0 {
            serialize_map.serialize_entry(id, &SnapshotSerialize(state))?;
        }
        serialize_map.end()
    }
}

impl<'a> serde::Serialize for SnapshotSerialize<'a, GraphicLayers> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serialize_tuple = serializer.serialize_tuple(Order::COUNT)?;
        for layer in self.0.as_inner() {
            serialize_tuple.serialize_element(&SnapshotSerialize(layer))?;
        }
        serialize_tuple.end()
    }
}

impl<'a> serde::Serialize for SnapshotSerialize<'a, IdMap<PaintList>> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serialize_map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in self.0 {
            serialize_map.serialize_entry(key, &SnapshotSerialize(value))?;
        }
        serialize_map.end()
    }
}

impl<'a> serde::Serialize for SnapshotSerialize<'a, PaintList> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let inner = self.0.as_inner();
        let mut serialize_seq = serializer.serialize_seq(Some(inner.len()))?;
        for shape in inner {
            serialize_seq.serialize_element(&SnapshotSerialize(shape))?;
        }
        serialize_seq.end()
    }
}

impl<'a> serde::Serialize for SnapshotSerialize<'a, epaint::ClippedShape> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serialize_tuple = serializer.serialize_tuple(2)?;
        serialize_tuple.serialize_element(&self.0.clip_rect)?;
        serialize_tuple.serialize_element(&SnapshotSerialize(&self.0.shape))?;
        serialize_tuple.end()
    }
}

impl<'a> serde::Serialize for SnapshotSerialize<'a, epaint::Shape> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serialize_seq = serializer.serialize_tuple(2)?;

        match self.0 {
            epaint::Shape::Noop => {
                serialize_seq.serialize_element(&0u8)?;
                serialize_seq.serialize_element(&())?;
            }
            epaint::Shape::Vec(x) => {
                serialize_seq.serialize_element(&1u8)?;
                serialize_seq.serialize_element(&SnapshotSerialize(x))?;
            }
            epaint::Shape::Circle(x) => {
                serialize_seq.serialize_element(&2u8)?;
                serialize_seq.serialize_element(x)?;
            }
            epaint::Shape::Ellipse(x) => {
                serialize_seq.serialize_element(&3u8)?;
                serialize_seq.serialize_element(x)?;
            }
            epaint::Shape::LineSegment { points, stroke } => {
                serialize_seq.serialize_element(&4u8)?;
                serialize_seq.serialize_element(&(points, stroke))?;
            }
            epaint::Shape::Path(x) => {
                serialize_seq.serialize_element(&5u8)?;
                serialize_seq.serialize_element(x)?;
            }
            epaint::Shape::Rect(x) => {
                serialize_seq.serialize_element(&6u8)?;
                serialize_seq.serialize_element(x)?;
            }
            epaint::Shape::Text(x) => {
                serialize_seq.serialize_element(&7u8)?;
                serialize_seq.serialize_element(&SnapshotSerialize(x))?;
            }
            epaint::Shape::Mesh(x) => {
                serialize_seq.serialize_element(&8u8)?;
                serialize_seq.serialize_element(x)?;
            }
            epaint::Shape::QuadraticBezier(x) => {
                serialize_seq.serialize_element(&9u8)?;
                serialize_seq.serialize_element(x)?;
            }
            epaint::Shape::CubicBezier(x) => {
                serialize_seq.serialize_element(&10u8)?;
                serialize_seq.serialize_element(x)?;
            }
            epaint::Shape::Callback(_) => {
                return Err(serde::ser::Error::custom(
                    "Cannot serialize callback shapes.",
                ));
            }
        }

        serialize_seq.end()
    }
}

impl<'a> serde::Serialize for SnapshotSerialize<'a, Vec<epaint::Shape>> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serialize_seq = serializer.serialize_seq(Some(self.0.len()))?;
        for shape in self.0 {
            serialize_seq.serialize_element(&SnapshotSerialize(shape))?;
        }
        serialize_seq.end()
    }
}

impl<'a> serde::Serialize for SnapshotSerialize<'a, epaint::TextShape> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serialize_tuple = serializer.serialize_tuple(14)?;
        serialize_tuple.serialize_element(&self.0.pos)?;

        serialize_tuple.serialize_element(&self.0.galley.job.text)?;
        serialize_tuple.serialize_element(&self.0.galley.job.sections)?;
        serialize_tuple
            .serialize_element(&TextWrappingSnapshot::from(self.0.galley.job.wrap.clone()))?;
        serialize_tuple.serialize_element(&self.0.galley.job.first_row_min_height)?;
        serialize_tuple.serialize_element(&self.0.galley.job.break_on_newline)?;
        serialize_tuple.serialize_element(&self.0.galley.job.halign)?;
        serialize_tuple.serialize_element(&self.0.galley.job.justify)?;
        serialize_tuple
            .serialize_element(&self.0.galley.job.round_output_size_to_nearest_ui_point)?;

        serialize_tuple.serialize_element(&self.0.underline)?;
        serialize_tuple.serialize_element(&self.0.fallback_color)?;
        serialize_tuple.serialize_element(&self.0.override_text_color)?;
        serialize_tuple.serialize_element(&self.0.opacity_factor)?;
        serialize_tuple.serialize_element(&self.0.angle)?;
        serialize_tuple.end()
    }
}

/// Implements custom, snapshot-specific deserialization logic for type `T`.
pub struct SnapshotDeserialize<T>(T);

/// Implements a deserialization visitor for type `T`.
pub struct SnapshotDeserializeVisitor<T>(PhantomData<T>);

impl<T> Default for SnapshotDeserializeVisitor<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<'de> serde::de::Deserialize<'de> for ContextSnapshot {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(deserializer
            .deserialize_tuple(Self::FIELDS, SnapshotDeserializeVisitor::<Self>::default())?
            .0)
    }
}

impl<'de> serde::de::Visitor<'de> for SnapshotDeserializeVisitor<ContextSnapshot> {
    type Value = SnapshotDeserialize<ContextSnapshot>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a sequence of tuple values")
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let deltas = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;

        let font_definitions = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;

        let memory = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;

        let options = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(3, &self))?;

        let style = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(4, &self))?;

        let new_zoom_factor = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(5, &self))?;

        let last_viewport = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(6, &self))?;

        let viewports = seq
            .next_element::<SnapshotDeserialize<ViewportIdMap<ViewportStateSnapshot>>>()?
            .ok_or_else(|| serde::de::Error::invalid_length(7, &self))?
            .0;

        Ok(SnapshotDeserialize(ContextSnapshot {
            deltas,
            font_definitions,
            memory,
            options,
            style,
            new_zoom_factor,
            last_viewport,
            viewports,
        }))
    }
}

impl<'de> serde::de::Deserialize<'de> for MemorySnapshot {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(deserializer
            .deserialize_tuple(Self::FIELDS, SnapshotDeserializeVisitor::<Self>::default())?
            .0)
    }
}

impl<'de> serde::de::Visitor<'de> for SnapshotDeserializeVisitor<MemorySnapshot> {
    type Value = SnapshotDeserialize<MemorySnapshot>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a sequence of tuple values")
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let label_selection_state = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        let new_font_definitions = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
        let viewport_id = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;
        let popup = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(3, &self))?;
        let everything_is_visible = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(4, &self))?;
        let layer_transforms = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(5, &self))?;
        let areas = seq
            .next_element::<SnapshotDeserialize<ViewportIdMap<Areas>>>()?
            .ok_or_else(|| serde::de::Error::invalid_length(6, &self))?
            .0;
        let interactions = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(7, &self))?;
        let focus = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(8, &self))?;

        Ok(SnapshotDeserialize(MemorySnapshot {
            label_selection_state,
            new_font_definitions,
            viewport_id,
            popup,
            everything_is_visible,
            layer_transforms,
            areas,
            interactions,
            focus,
        }))
    }
}

impl<'de> serde::de::Deserialize<'de> for SnapshotDeserialize<ViewportIdMap<Areas>> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(SnapshotDeserializeVisitor::<ViewportIdMap<Areas>>::default())
    }
}

impl<'de> serde::de::Visitor<'de> for SnapshotDeserializeVisitor<ViewportIdMap<Areas>> {
    type Value = SnapshotDeserialize<ViewportIdMap<Areas>>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a map of viewport state snapshots")
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut result = ViewportIdMap::with_capacity(map.size_hint().unwrap_or_default());
        while let Some((key, value)) = map.next_entry::<ViewportId, SnapshotDeserialize<Areas>>()? {
            result.insert(key, value.0);
        }
        Ok(SnapshotDeserialize(result))
    }
}

impl<'de> serde::de::Deserialize<'de> for SnapshotDeserialize<Areas> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_tuple(6, SnapshotDeserializeVisitor::<Areas>::default())
    }
}

impl<'de> serde::de::Visitor<'de> for SnapshotDeserializeVisitor<Areas> {
    type Value = SnapshotDeserialize<Areas>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a sequence of tuple values")
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let areas = seq
            .next_element::<SnapshotDeserialize<IdMap<AreaState>>>()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?
            .0;
        let order = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
        let visible_current_frame = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;
        let visible_last_frame = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(3, &self))?;
        let wants_to_be_on_top = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(4, &self))?;
        let sublayers = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(5, &self))?;

        Ok(SnapshotDeserialize(Areas {
            areas,
            order,
            visible_current_frame,
            visible_last_frame,
            wants_to_be_on_top,
            sublayers,
        }))
    }
}

impl<'de> serde::de::Deserialize<'de> for SnapshotDeserialize<IdMap<AreaState>> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(SnapshotDeserializeVisitor::<IdMap<AreaState>>::default())
    }
}

impl<'de> serde::de::Visitor<'de> for SnapshotDeserializeVisitor<IdMap<AreaState>> {
    type Value = SnapshotDeserialize<IdMap<AreaState>>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a map of area states")
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut result = IdMap::with_capacity(map.size_hint().unwrap_or_default());
        while let Some(key) = map.next_key::<Id>()? {
            result.insert(key, map.next_value::<SnapshotDeserialize<AreaState>>()?.0);
        }
        Ok(SnapshotDeserialize(result))
    }
}

impl<'de> serde::de::Deserialize<'de> for SnapshotDeserialize<AreaState> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_tuple(5, SnapshotDeserializeVisitor::<AreaState>::default())
    }
}

impl<'de> serde::de::Visitor<'de> for SnapshotDeserializeVisitor<AreaState> {
    type Value = SnapshotDeserialize<AreaState>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a sequence of tuple values")
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let pivot_pos = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        let pivot = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
        let size = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;
        let interactable = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(3, &self))?;
        let last_became_visible_at = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(4, &self))?;

        Ok(SnapshotDeserialize(AreaState {
            pivot_pos,
            pivot,
            size,
            interactable,
            last_became_visible_at,
        }))
    }
}

impl<'de> serde::de::Deserialize<'de>
    for SnapshotDeserialize<ViewportIdMap<ViewportStateSnapshot>>
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(SnapshotDeserializeVisitor::<
            ViewportIdMap<ViewportStateSnapshot>,
        >::default())
    }
}

impl<'de> serde::de::Visitor<'de>
    for SnapshotDeserializeVisitor<ViewportIdMap<ViewportStateSnapshot>>
{
    type Value = SnapshotDeserialize<ViewportIdMap<ViewportStateSnapshot>>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a map of viewport state snapshots")
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut result = ViewportIdMap::with_capacity(map.size_hint().unwrap_or_default());
        while let Some((key, value)) =
            map.next_entry::<ViewportId, SnapshotDeserialize<ViewportStateSnapshot>>()?
        {
            result.insert(key, value.0);
        }
        Ok(SnapshotDeserialize(result))
    }
}

impl<'de> serde::de::Deserialize<'de> for SnapshotDeserialize<ViewportStateSnapshot> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_tuple(
            ViewportStateSnapshot::FIELDS,
            SnapshotDeserializeVisitor::<ViewportStateSnapshot>::default(),
        )
    }
}

impl<'de> serde::de::Visitor<'de> for SnapshotDeserializeVisitor<ViewportStateSnapshot> {
    type Value = SnapshotDeserialize<ViewportStateSnapshot>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a sequence of tuple values")
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let class = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        let builder = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
        let input = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;
        let this_frame = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(3, &self))?;
        let prev_frame = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(4, &self))?;
        let used = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(5, &self))?;
        let hits = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(6, &self))?;
        let interact_widgets = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(7, &self))?;
        let graphics = seq
            .next_element::<SnapshotDeserialize<GraphicLayers>>()?
            .ok_or_else(|| serde::de::Error::invalid_length(8, &self))?
            .0;
        let output = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(9, &self))?;
        let commands = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(10, &self))?;

        Ok(SnapshotDeserialize(ViewportStateSnapshot {
            class,
            builder,
            input,
            this_frame,
            prev_frame,
            used,
            hits,
            interact_widgets,
            graphics,
            output,
            commands,
        }))
    }
}

impl<'de> serde::de::Deserialize<'de> for SnapshotDeserialize<GraphicLayers> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_tuple(
            Order::COUNT,
            SnapshotDeserializeVisitor::<GraphicLayers>::default(),
        )
    }
}

impl<'de> serde::de::Visitor<'de> for SnapshotDeserializeVisitor<GraphicLayers> {
    type Value = SnapshotDeserialize<GraphicLayers>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a sequence of tuple values")
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut raw_layers = std::array::from_fn::<_, { Order::COUNT }, _>(|i| {
            Some(
                seq.next_element::<SnapshotDeserialize<IdMap<PaintList>>>()
                    .and_then(|x| x.ok_or_else(|| serde::de::Error::invalid_length(i, &self))),
            )
        });

        for i in &mut raw_layers {
            if i.as_ref().expect("Failed to get layer.").is_err() {
                return Err(std::mem::take(i)
                    .expect("Failed to get layer.")
                    .err()
                    .expect("Failed to get error."));
            }
        }

        Ok(SnapshotDeserialize(GraphicLayers::from_inner(
            std::array::from_fn(|i| {
                std::mem::take(&mut raw_layers[i])
                    .and_then(|x| x.ok())
                    .expect("Failed to get layer.")
                    .0
            }),
        )))
    }
}

impl<'de> serde::de::Deserialize<'de> for SnapshotDeserialize<IdMap<PaintList>> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(SnapshotDeserializeVisitor::<IdMap<PaintList>>::default())
    }
}

impl<'de> serde::de::Visitor<'de> for SnapshotDeserializeVisitor<IdMap<PaintList>> {
    type Value = SnapshotDeserialize<IdMap<PaintList>>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a map of paint lists")
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut result = IdMap::with_capacity(map.size_hint().unwrap_or_default());
        while let Some(key) = map.next_key::<Id>()? {
            result.insert(key, map.next_value::<SnapshotDeserialize<PaintList>>()?.0);
        }
        Ok(SnapshotDeserialize(result))
    }
}

impl<'de> serde::de::Deserialize<'de> for SnapshotDeserialize<PaintList> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_seq(SnapshotDeserializeVisitor::<PaintList>::default())
    }
}

impl<'de> serde::de::Visitor<'de> for SnapshotDeserializeVisitor<PaintList> {
    type Value = SnapshotDeserialize<PaintList>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a list of clipped rects")
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut result = Vec::with_capacity(seq.size_hint().unwrap_or_default());
        while let Some(element) = seq.next_element::<SnapshotDeserialize<epaint::ClippedShape>>()? {
            result.push(element.0);
        }
        Ok(SnapshotDeserialize(PaintList::from_inner(result)))
    }
}

impl<'de> serde::de::Deserialize<'de> for SnapshotDeserialize<epaint::ClippedShape> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_tuple(
            2,
            SnapshotDeserializeVisitor::<epaint::ClippedShape>::default(),
        )
    }
}

impl<'de> serde::de::Visitor<'de> for SnapshotDeserializeVisitor<epaint::ClippedShape> {
    type Value = SnapshotDeserialize<epaint::ClippedShape>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a tuple of size and shape values")
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let clip_rect = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;

        let shape = seq
            .next_element::<SnapshotDeserialize<epaint::Shape>>()?
            .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?
            .0;

        Ok(SnapshotDeserialize(epaint::ClippedShape {
            clip_rect,
            shape,
        }))
    }
}

impl<'de> serde::de::Deserialize<'de> for SnapshotDeserialize<epaint::Shape> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_tuple(2, SnapshotDeserializeVisitor::<epaint::Shape>::default())
    }
}

impl<'de> serde::de::Visitor<'de> for SnapshotDeserializeVisitor<epaint::Shape> {
    type Value = SnapshotDeserialize<epaint::Shape>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a tuple containing discriminant and value")
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let discriminant = seq
            .next_element::<u8>()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;

        let result = match discriminant {
            0 => seq.next_element::<()>()?.map(|_| epaint::Shape::Noop),
            1 => seq
                .next_element::<SnapshotDeserialize<Vec<epaint::Shape>>>()?
                .map(|x| epaint::Shape::Vec(x.0)),
            2 => seq
                .next_element::<epaint::CircleShape>()?
                .map(epaint::Shape::Circle),
            3 => seq
                .next_element::<epaint::EllipseShape>()?
                .map(epaint::Shape::Ellipse),
            4 => seq
                .next_element::<([emath::Pos2; 2], epaint::PathStroke)>()?
                .map(|(points, stroke)| epaint::Shape::LineSegment { points, stroke }),
            5 => seq
                .next_element::<epaint::PathShape>()?
                .map(epaint::Shape::Path),
            6 => seq
                .next_element::<epaint::RectShape>()?
                .map(epaint::Shape::Rect),
            7 => seq
                .next_element::<SnapshotDeserialize<epaint::TextShape>>()?
                .map(|x| epaint::Shape::Text(x.0)),
            8 => seq.next_element::<epaint::Mesh>()?.map(epaint::Shape::Mesh),
            9 => seq
                .next_element::<epaint::QuadraticBezierShape>()?
                .map(epaint::Shape::QuadraticBezier),
            10 => seq
                .next_element::<epaint::CubicBezierShape>()?
                .map(epaint::Shape::CubicBezier),
            _ => return Err(serde::de::Error::custom("invalid shape type")),
        }
        .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;

        Ok(SnapshotDeserialize(result))
    }
}

impl<'de> serde::de::Deserialize<'de> for SnapshotDeserialize<Vec<epaint::Shape>> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_seq(SnapshotDeserializeVisitor::<Vec<epaint::Shape>>::default())
    }
}

impl<'de> serde::de::Visitor<'de> for SnapshotDeserializeVisitor<Vec<epaint::Shape>> {
    type Value = SnapshotDeserialize<Vec<epaint::Shape>>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a list of clipped rects")
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut result = Vec::with_capacity(seq.size_hint().unwrap_or_default());
        while let Some(element) = seq.next_element::<SnapshotDeserialize<epaint::Shape>>()? {
            result.push(element.0);
        }
        Ok(SnapshotDeserialize(result))
    }
}

impl<'de> serde::de::Deserialize<'de> for SnapshotDeserialize<epaint::TextShape> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_tuple(
            14,
            SnapshotDeserializeVisitor::<epaint::TextShape>::default(),
        )
    }
}

impl<'de> serde::de::Visitor<'de> for SnapshotDeserializeVisitor<epaint::TextShape> {
    type Value = SnapshotDeserialize<epaint::TextShape>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "a tuple of values")
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let pos = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;

        let text = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;

        let sections = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;

        let wrap = seq
            .next_element::<TextWrappingSnapshot>()?
            .ok_or_else(|| serde::de::Error::invalid_length(3, &self))?;

        let first_row_min_height = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(4, &self))?;

        let break_on_newline = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(5, &self))?;

        let halign = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(6, &self))?;

        let justify = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(7, &self))?;

        let round_output_size_to_nearest_ui_point = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(8, &self))?;

        let job = Arc::new(epaint::text::LayoutJob {
            text,
            sections,
            wrap: wrap.into(),
            first_row_min_height,
            break_on_newline,
            halign,
            justify,
            round_output_size_to_nearest_ui_point,
        });

        let underline = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;

        let fallback_color = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(3, &self))?;

        let override_text_color = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(4, &self))?;

        let opacity_factor = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(5, &self))?;

        let angle = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(6, &self))?;

        let galley = Arc::new(epaint::Galley {
            job,
            rows: Vec::new(),
            elided: false,
            rect: emath::Rect::ZERO,
            mesh_bounds: emath::Rect::ZERO,
            num_indices: 0,
            num_vertices: 0,
            pixels_per_point: 0.0,
        });

        Ok(SnapshotDeserialize(epaint::TextShape {
            pos,
            galley,
            underline,
            fallback_color,
            override_text_color,
            opacity_factor,
            angle,
        }))
    }
}
