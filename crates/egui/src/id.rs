// TODO(emilk): have separate types `PositionId` and `UniqueId`. ?

use epaint::text::TextWrapMode;
use epaint::Color32;
use std::num::NonZeroU64;

/// egui tracks widgets frame-to-frame using [`Id`]s.
///
/// For instance, if you start dragging a slider one frame, egui stores
/// the sliders [`Id`] as the current active id so that next frame when
/// you move the mouse the same slider changes, even if the mouse has
/// moved outside the slider.
///
/// For some widgets [`Id`]s are also used to persist some state about the
/// widgets, such as Window position or whether not a collapsing header region is open.
///
/// This implies that the [`Id`]s must be unique.
///
/// For simple things like sliders and buttons that don't have any memory and
/// doesn't move we can use the location of the widget as a source of identity.
/// For instance, a slider only needs a unique and persistent ID while you are
/// dragging the slider. As long as it is still while moving, that is fine.
///
/// For things that need to persist state even after moving (windows, collapsing headers)
/// the location of the widgets is obviously not good enough. For instance,
/// a collapsing region needs to remember whether or not it is open even
/// if the layout next frame is different and the collapsing is not lower down
/// on the screen.
///
/// Then there are widgets that need no identifiers at all, like labels,
/// because they have no state nor are interacted with.
///
/// This is niche-optimized to that `Option<Id>` is the same size as `Id`.
#[derive(Clone, Copy, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Id(NonZeroU64);

impl nohash_hasher::IsEnabled for Id {}

pub trait AsId: std::hash::Hash + std::fmt::Debug {}

impl<T: std::hash::Hash + std::fmt::Debug> AsId for T {}

impl Id {
    /// A special [`Id`], in particular as a key to [`crate::Memory::data`]
    /// for when there is no particular widget to attach the data.
    ///
    /// The null [`Id`] is still a valid id to use in all circumstances,
    /// though obviously it will lead to a lot of collisions if you do use it!
    pub const NULL: Self = Self(NonZeroU64::MAX);

    #[inline]
    const fn from_hash(hash: u64) -> Self {
        if let Some(nonzero) = NonZeroU64::new(hash) {
            Self(nonzero)
        } else {
            Self(NonZeroU64::MIN) // The hash was exactly zero (very bad luck)
        }
    }

    /// Generate a new [`Id`] by hashing some source (e.g. a string or integer).
    pub fn new<T: AsId>(source: T) -> Self {
        let id = Self::from_hash(ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(&source));

        #[cfg(debug_assertions)]
        id_source::maybe_insert(id, source, None);

        id
    }

    /// Generate a new [`Id`] by hashing the parent [`Id`] and the given argument.
    pub fn with(self, child: impl AsId) -> Self {
        use std::hash::{BuildHasher as _, Hasher as _};
        let mut hasher = ahash::RandomState::with_seeds(1, 2, 3, 4).build_hasher();
        hasher.write_u64(self.0.get());
        child.hash(&mut hasher);
        let id = Self::from_hash(hasher.finish());

        #[cfg(debug_assertions)]
        id_source::maybe_insert(id, &child, Some(self));

        id
    }

    /// Short and readable summary
    pub fn short_debug_format(&self) -> String {
        format!("{:04X}", self.value() as u16)
    }

    /// The inner value of the [`Id`].
    ///
    /// This is a high-entropy hash, or [`Self::NULL`].
    #[inline(always)]
    pub fn value(&self) -> u64 {
        self.0.get()
    }

    pub fn accesskit_id(&self) -> accesskit::NodeId {
        self.value().into()
    }

    /// Create a new [`Id`] from a high-entropy value. No hashing is done.
    ///
    /// This can be useful if you have an [`Id`] that was converted to some other type
    /// (e.g. accesskit::NodeId) and you want to convert it back to an [`Id`].
    ///
    /// # Safety
    /// You need to ensure that the value is high-entropy since it might be used in
    /// a [`IdSet`] or [`IdMap`], which rely on the assumption that [`Id`]s have good entropy.
    ///
    /// The method is not unsafe in terms of memory safety.
    ///
    /// # Panics
    /// If the value is zero, this will panic.
    #[doc(hidden)]
    #[expect(unsafe_code)]
    pub unsafe fn from_high_entropy_bits(value: u64) -> Self {
        Self(NonZeroU64::new(value).expect("Id must be non-zero."))
    }

    /// Paint a rectangle around the widget, if it can be found.
    pub fn try_highlight(self, ctx: &crate::Context) {
        let response = ctx.read_response(self);
        if let Some(response) = response {
            ctx.debug_painter().debug_rect(
                response.rect,
                Color32::GREEN,
                self.short_debug_format(),
            );
        }

        if let Some(area_rect) = ctx.memory(|mem| mem.area_rect(self)) {
            ctx.debug_painter()
                .debug_rect(area_rect, Color32::RED, self.short_debug_format());
        }
    }

    pub fn ui(self, ui: &mut crate::Ui) -> crate::Response {
        #[cfg(debug_assertions)]
        let debug_label = self
            .info()
            .map(|info| format!("{} ({})", self.short_debug_format(), info.source));
        #[cfg(not(debug_assertions))]
        let debug_label: Option<String> = None;
        let response = ui.code(debug_label.unwrap_or_else(|| self.short_debug_format()));

        #[cfg(debug_assertions)]
        let response = response.on_hover_ui(|ui| {
            let checkbox_id = Id::new("egui::id::show_as_code_checkbox");
            let mut show_as_code = ui
                .ctx()
                .data_mut(|d| *d.get_persisted_mut_or_default::<bool>(checkbox_id));
            ui.checkbox(&mut show_as_code, "Show as code");
            ui.ctx()
                .data_mut(|d| d.insert_temp(checkbox_id, show_as_code));

            if show_as_code {
                ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                ui.style_mut().interaction.selectable_labels = true;
                ui.code(self.to_code_string());
            } else {
                Self::tree_ui(ui, self, "", 0);
            }
        });

        if response.hovered() {
            self.try_highlight(ui.ctx());
        }

        response
    }
}

#[cfg(debug_assertions)]
mod id_source {
    use crate::{AsId, CollapsingHeader, Id, IdMap};
    use epaint::mutex::RwLock;
    use std::fmt::{Display, Formatter};
    use std::hash::Hasher;
    use std::sync::LazyLock;

    #[derive(Clone)]
    pub struct IdInfo {
        /// What was this Id generated from?
        pub source: IdSource,

        /// If the Id was crated via [`Id::with`], what was the parent Id?
        pub parent: Option<Id>,
    }

    #[derive(Clone)]
    pub enum IdSource {
        Id(Id),
        Other(String),
    }

    impl Display for IdSource {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Id(id) => {
                    write!(f, "{}", id.short_debug_format())
                }
                Self::Other(other) => {
                    write!(f, "{other}")
                }
            }
        }
    }

    static ID_MAP: LazyLock<RwLock<IdMap<IdInfo>>> = LazyLock::new(|| {
        let mut map = IdMap::default();
        map.insert(
            Id::NULL,
            IdInfo {
                source: IdSource::Other("Id::NULL".to_owned()),
                parent: None,
            },
        );
        RwLock::new(map)
    });

    /// Ugly hack to try to determine if T is an Id or not.
    #[derive(Default)]
    struct ExtractIdHasher {
        val: Option<u64>,
        not_id: bool,
    }

    impl ExtractIdHasher {
        fn id(&self) -> Option<Id> {
            self.val.map(Id::from_hash)
        }
    }

    impl Hasher for ExtractIdHasher {
        fn finish(&self) -> u64 {
            unreachable!()
        }

        fn write(&mut self, _bytes: &[u8]) {
            self.not_id = true;
            self.val = None;
        }

        fn write_u64(&mut self, i: u64) {
            if !self.not_id && self.val.is_none() {
                self.val = Some(i);
            } else {
                self.not_id = true;
                self.val = None;
            }
        }
    }

    /// Checks if [`T`] is a [`Id`].
    ///
    /// If it is, it returns `IdSource::Id`, otherwise it returns `IdSource::Other`.
    fn get_source<T: AsId>(t: T) -> IdSource {
        let mut hasher = ExtractIdHasher::default();

        t.hash(&mut hasher);

        let maybe_source_id = hasher.id();

        // Ideally we would just implement AsId for Id with specialization, but that's not
        // a thing yet :( So we check if the hash is already in the map, if so, the source must be
        // an Id.
        if let Some(maybe_source_id) = maybe_source_id {
            if ID_MAP.read().contains_key(&maybe_source_id) {
                IdSource::Id(maybe_source_id)
            } else {
                IdSource::Other(format!("{t:?}"))
            }
        } else {
            IdSource::Other(format!("{t:?}"))
        }
    }

    pub(super) fn maybe_insert(id: Id, source: impl AsId, parent: Option<Id>) {
        if !ID_MAP.read().contains_key(&id) {
            let source1 = get_source(source);
            ID_MAP.write().insert(
                id,
                IdInfo {
                    source: source1,
                    parent,
                },
            );
        }
    }

    /// Format a call like `Id::new(arg)` or `.with(arg)`.
    ///
    /// `outer_indent` is the indentation of the call itself (e.g. `"  "` if inside a chain).
    ///
    /// If the arg is single-line, keeps it inline: `Id::new("foo")`
    /// If the arg is multi-line, uses rustfmt style:
    /// ```text
    /// Id::new(
    ///     arg,
    /// )
    /// ```
    fn format_call(func: &str, arg: &str, outer_indent: &str, inner_indent: &str) -> String {
        if arg.contains('\n') {
            let indented_arg = arg
                .lines()
                .map(|l| format!("{outer_indent}{inner_indent}{l}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{outer_indent}{func}(\n{indented_arg}\n{outer_indent})")
        } else {
            format!("{outer_indent}{func}({arg})")
        }
    }

    /// Align all `// XXXX` comments in a string to the same column.
    fn align_comments(s: &str) -> String {
        let comment_marker = " // ";
        let max_code_len = s
            .lines()
            .filter_map(|line| line.find(comment_marker).map(|pos| pos))
            .max()
            .unwrap_or(0);

        s.lines()
            .map(|line| {
                if let Some(pos) = line.find(comment_marker) {
                    let code = &line[..pos];
                    let comment = &line[pos + 1..]; // include the space before //
                    format!("{code:<max_code_len$} {comment}")
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    impl Id {
        /// Get info about this id (what source was it generated from, what parent does it have)?
        ///
        /// Only available with `#[cfg(debug_assertions)]`.
        pub fn info(&self) -> Option<IdInfo> {
            ID_MAP.read().get(self).cloned()
        }

        /// Returns a Rust code representation of how this Id was constructed.
        ///
        /// Formats like rustfmt would:
        /// ```text
        /// Id::new("parent")
        ///   .with("child")
        /// ```
        pub fn to_code_string(&self) -> String {
            let calls = Self::collect_calls(*self);
            let result = Self::format_chain(&calls, true);
            align_comments(&result)
        }

        /// Code string without id comments, used for nested args.
        fn to_code_string_inner(&self) -> String {
            let calls = Self::collect_calls(*self);
            Self::format_chain(&calls, false)
        }

        fn collect_calls(id: Id) -> Vec<(&'static str, String, Id)> {
            let mut calls: Vec<(&str, String, Id)> = Vec::new();
            let mut current = id;

            loop {
                let Some(info) = current.info() else {
                    calls.push(("", format!("Id({})", current.short_debug_format()), current));
                    break;
                };

                let source_str = match &info.source {
                    IdSource::Id(id) => {
                        // Use commented version for multi-line args (each line
                        // except the first gets a comment), plain for single-line
                        // to avoid embedding comments inline.
                        let plain = id.to_code_string_inner();
                        if plain.contains('\n') {
                            id.to_code_string()
                        } else {
                            plain
                        }
                    }
                    IdSource::Other(s) => s.clone(),
                };

                match info.parent {
                    None => {
                        calls.push(("Id::new", source_str, current));
                        break;
                    }
                    Some(parent) => {
                        calls.push((".with", source_str, current));
                        current = parent;
                    }
                }
            }

            calls.reverse();
            calls
        }

        fn format_chain(calls: &[(&str, String, Id)], with_comments: bool) -> String {
            const INDENT: &str = "  ";

            let mut parts: Vec<String> = Vec::new();
            for (i, (func, arg, id)) in calls.iter().enumerate() {
                let base = if func.is_empty() {
                    arg.clone()
                } else {
                    let outer = if i == 0 { "" } else { INDENT };
                    format_call(func, arg, outer, INDENT)
                };

                if with_comments {
                    let comment = format!(" // {}", id.short_debug_format());
                    let mut lines: Vec<&str> = base.lines().collect();
                    let last = lines.len() - 1;
                    let last_with_comment = format!("{}{comment}", lines[last]);
                    lines[last] = &last_with_comment;
                    parts.push(lines.join("\n"));
                } else {
                    parts.push(base);
                }
            }

            if parts.len() <= 1 {
                parts.into_iter().next().unwrap_or_default()
            } else {
                let base = &parts[0];
                let withs: Vec<&str> = parts[1..].iter().map(|s| s.as_str()).collect();
                format!("{base}\n{}", withs.join("\n"))
            }
        }

        pub(super) fn tree_ui(ui: &mut crate::Ui, id: Self, prefix: &str, depth: usize) {
            let info = id.info();
            if let Some(info) = info {
                let response =
                    CollapsingHeader::new(format!("{}Id({})", prefix, id.short_debug_format()))
                        .default_open(depth < 4)
                        .show(ui, |ui| {
                            match info.source {
                                IdSource::Id(id_source) => {
                                    Self::tree_ui(ui, id_source, "Source: ", depth + 1);
                                }
                                IdSource::Other(other) => {
                                    ui.horizontal(|ui| {
                                        ui.add_space(ui.spacing().indent);
                                        ui.label("Source:");
                                        ui.code(other);
                                    });
                                }
                            }

                            if let Some(parent) = info.parent {
                                Self::tree_ui(ui, parent, "Parent: ", depth + 1);
                            }
                        });

                if response.header_response.hovered() {
                    id.try_highlight(ui.ctx());
                }
            }
        }
    }

    #[test]
    fn test_fake_hasher() {
        use std::hash::Hash as _;
        let mut hasher = ExtractIdHasher::default();

        let id = Id::new("test");
        id.hash(&mut hasher);

        assert_eq!(hasher.id(), Some(id));
    }

    #[test]
    fn test_to_code_string() {
        let parent = Id::new("parent");
        let child = parent.with("child");
        let grandchild = child.with("grandchild");
        let nested = Id::new(grandchild).with(grandchild);

        assert_eq!(
            parent.to_code_string(),
            r#"Id::new("parent") // 9DE0"#
        );
        assert_eq!(
            child.to_code_string(),
            r#"Id::new("parent") // 9DE0
  .with("child")  // F27D"#
        );
        assert_eq!(
            grandchild.to_code_string(),
            r#"Id::new("parent")     // 9DE0
  .with("child")      // F27D
  .with("grandchild") // 61DA"#
        );
        assert_eq!(
            nested.to_code_string(),
            r#"Id::new(
  Id::new("parent")       // 9DE0
    .with("child")        // F27D
    .with("grandchild")   // 61DA
)                         // 02A4
  .with(
    Id::new("parent")     // 9DE0
      .with("child")      // F27D
      .with("grandchild") // 61DA
  )                       // B2D6"#
        );
    }

    #[test]
    fn test_debug_format() {
        let parent = Id::new("parent");
        let child = parent.with("child");
        let nested = Id::new(parent).with(child);

        assert_eq!(format!("{parent:?}"), r#"9DE0 ("parent")"#);
        assert_eq!(format!("{child:?}"), r#"F27D ("child") <- 9DE0 ("parent")"#);
        assert_eq!(
            format!("{nested:?}"),
            r#"A8BE(F27D ("child") <- 9DE0 ("parent")) <- B20C(9DE0 ("parent"))"#
        );
    }
}

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04X}", self.value() as u16)?;

        #[cfg(debug_assertions)]
        if let Some(info) = self.info() {
            match info.source {
                id_source::IdSource::Id(source_id) => {
                    write!(f, "({source_id:?})")?;
                }
                id_source::IdSource::Other(label) => {
                    write!(f, " ({label})")?;
                }
            }
            if let Some(parent) = info.parent {
                // Let's hope there are no cycles!
                write!(f, " <- {parent:?}")?;
            }
        }

        Ok(())
    }
}

/// Convenience
impl From<&'static str> for Id {
    #[inline]
    fn from(string: &'static str) -> Self {
        Self::new(string)
    }
}

impl From<String> for Id {
    #[inline]
    fn from(string: String) -> Self {
        Self::new(string)
    }
}

#[test]
fn id_size() {
    assert_eq!(std::mem::size_of::<Id>(), 8);
    assert_eq!(std::mem::size_of::<Option<Id>>(), 8);
}

// ----------------------------------------------------------------------------

/// `IdSet` is a `HashSet<Id>` optimized by knowing that [`Id`] has good entropy, and doesn't need more hashing.
pub type IdSet = nohash_hasher::IntSet<Id>;

/// `IdMap<V>` is a `HashMap<Id, V>` optimized by knowing that [`Id`] has good entropy, and doesn't need more hashing.
pub type IdMap<V> = nohash_hasher::IntMap<Id, V>;
