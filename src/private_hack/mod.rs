#![allow(warnings)]

pub use egui::{
    emath, epaint, Id, InputState, LayerId, ViewportBuilder, ViewportClass, ViewportId,
    ViewportIdMap,
};

pub use crate::private_hack::animation_manager::*;
pub use crate::private_hack::hit_test::*;
pub use crate::private_hack::interaction::*;
pub use crate::private_hack::layers::*;
pub use crate::private_hack::widget_rect::*;
use egui::emath::*;
use egui::epaint::*;
use egui::load::*;
use egui::os::*;
use egui::style::*;
use egui::*;
use serde::*;
use std::mem::*;
use std::sync::*;
use std::time::*;

mod animation_manager;
mod hit_test;
mod interaction;
mod layers;
mod widget_rect;

pub struct Context(pub Arc<egui::mutex::RwLock<ContextImpl>>);

impl Context {
    pub fn from_context(context: &egui::Context) -> &Self {
        unsafe {
            const _ASSERT_SIZES_EQ: () = {
                if size_of::<Context>() != size_of::<egui::Context>() {
                    panic!("Context size not equal.");
                }
                if size_of::<Memory>() != size_of::<egui::Memory>() {
                    panic!("Memory size not equal.");
                }
                if size_of::<Style>() != size_of::<egui::Style>() {
                    panic!("Style size not equal.");
                }
            };
            transmute(context)
        }
    }
}

pub struct ContextImpl {
    pub fonts: std::collections::BTreeMap<OrderedFloat<f32>, Fonts>,
    pub font_definitions: FontDefinitions,
    pub memory: Memory,
    pub animation_manager: AnimationManager,
    pub plugins: Plugins,
    pub tex_manager: WrappedTextureManager,
    pub new_zoom_factor: Option<f32>,
    pub os: OperatingSystem,
    pub viewport_stack: Vec<ViewportIdPair>,
    pub last_viewport: ViewportId,
    pub paint_stats: PaintStats,
    pub request_repaint_callback: Option<Box<dyn Fn(RequestRepaintInfo) + Send + Sync>>,
    pub viewport_parents: ViewportIdMap<ViewportId>,
    pub viewports: ViewportIdMap<ViewportState>,
    pub embed_viewports: bool,
    #[cfg(feature = "accesskit")]
    pub is_accesskit_enabled: bool,
    pub loaders: Arc<Loaders>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Memory {
    pub options: Options,
    #[serde(skip)]
    pub data: egui::util::IdTypeMap,
    #[serde(skip)]
    pub caches: crate::util::cache::CacheStorage,
    #[serde(skip)]
    pub new_font_definitions: Option<epaint::text::FontDefinitions>,
    #[serde(skip)]
    pub add_fonts: Vec<FontInsert>,
    #[serde(skip)]
    pub viewport_id: ViewportId,
    #[serde(skip)]
    pub everything_is_visible: bool,
    pub to_global: ahash::HashMap<LayerId, TSTransform>,
    pub areas: ViewportIdMap<Areas>,
    #[serde(skip)]
    pub interactions: ViewportIdMap<InteractionState>,
    #[serde(skip)]
    pub focus: ViewportIdMap<Focus>,
    #[serde(skip)]
    pub popups: ViewportIdMap<OpenPopup>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct OpenPopup {
    pub id: Id,
    pub pos: Option<Pos2>,
    pub open_this_frame: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontInsert {
    pub name: String,
    pub data: FontData,
    pub families: Vec<InsertFontFamily>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertFontFamily {
    pub family: FontFamily,
    pub priority: FontPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FontPriority {
    Highest,
    Lowest,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct WidgetTextCursor {
    pub widget_id: Id,
    pub ccursor: egui::text::CCursor,
    pub pos: Pos2,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CurrentSelection {
    pub layer_id: LayerId,
    pub primary: WidgetTextCursor,
    pub secondary: WidgetTextCursor,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LabelSelectionState {
    pub selection: Option<CurrentSelection>,
    pub selection_bbox_last_frame: Rect,
    pub selection_bbox_this_frame: Rect,
    pub any_hovered: bool,
    pub is_dragging: bool,
    pub has_reached_primary: bool,
    pub has_reached_secondary: bool,
    pub text_to_copy: String,
    pub last_copied_galley_rect: Option<Rect>,
    pub painted_selections: Vec<(ShapeIdx, Vec<RowVertexIndices>)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RowVertexIndices {
    pub row: usize,
    pub vertex_indices: [u32; 6],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShapeIdx(pub usize);

impl LabelSelectionState {
    /// Converts the `egui` object reference to a reference of this type.
    pub fn from_label_selection_state(value: &egui::text_selection::LabelSelectionState) -> &Self {
        unsafe { transmute(value) }
    }
}

impl From<LabelSelectionState> for egui::text_selection::LabelSelectionState {
    fn from(value: LabelSelectionState) -> Self {
        unsafe { transmute(value) }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Options {
    #[serde(skip)]
    pub dark_style: std::sync::Arc<Style>,
    #[serde(skip)]
    pub light_style: std::sync::Arc<Style>,
    pub theme_preference: ThemePreference,
    pub fallback_theme: Theme,
    #[serde(skip)]
    pub system_theme: Option<Theme>,
    pub zoom_factor: f32,
    pub zoom_with_keyboard: bool,
    pub tessellation_options: epaint::TessellationOptions,
    pub repaint_on_widget_change: bool,
    pub max_passes: std::num::NonZeroUsize,
    pub screen_reader: bool,
    pub preload_font_glyphs: bool,
    pub warn_on_id_clash: bool,
    pub input_options: InputOptions,
    pub reduce_texture_memory: bool,
}

impl Options {
    pub fn style_mut(&mut self) -> &mut std::sync::Arc<Style> {
        match self.theme() {
            Theme::Dark => &mut self.dark_style,
            Theme::Light => &mut self.light_style,
        }
    }

    pub fn theme(&self) -> Theme {
        match self.theme_preference {
            ThemePreference::Dark => Theme::Dark,
            ThemePreference::Light => Theme::Light,
            ThemePreference::System => self.system_theme.unwrap_or(self.fallback_theme),
        }
    }

    pub fn style(&self) -> &std::sync::Arc<Style> {
        match self.theme() {
            Theme::Dark => &self.dark_style,
            Theme::Light => &self.light_style,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub enum Theme {
    Dark,
    Light,
}

#[derive(Clone)]
pub struct NumberFormatter(
    pub Arc<dyn 'static + Sync + Send + Fn(f64, crate::RangeInclusive<usize>) -> String>,
);

impl NumberFormatter {
    /// The first argument is the number to be formatted.
    /// The second argument is the range of the number of decimals to show.
    ///
    /// See [`Self::format`] for the meaning of the `decimals` argument.
    #[inline]
    pub fn new(
        formatter: impl 'static + Sync + Send + Fn(f64, crate::RangeInclusive<usize>) -> String,
    ) -> Self {
        Self(Arc::new(formatter))
    }

    #[inline]
    pub fn format(&self, value: f64, decimals: crate::RangeInclusive<usize>) -> String {
        (self.0)(value, decimals)
    }
}

impl Default for Style {
    fn default() -> Self {
        #[allow(deprecated)]
        Self {
            override_font_id: None,
            override_text_style: None,
            override_text_valign: None,
            text_styles: default_text_styles(),
            drag_value_text_style: TextStyle::Button,
            number_formatter: NumberFormatter(Arc::new(emath::format_with_decimals_in_range)),
            wrap: None,
            wrap_mode: None,
            spacing: Spacing::default(),
            interaction: Interaction::default(),
            visuals: Visuals::default(),
            animation_time: 1.0 / 12.0,
            #[cfg(debug_assertions)]
            debug: Default::default(),
            explanation_tooltips: false,
            url_in_tooltip: false,
            always_scroll_the_only_direction: false,
            scroll_animation: ScrollAnimation::default(),
            compact_menu_style: false,
            
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub enum ThemePreference {
    Dark,
    Light,
    System,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct InputOptions {
    pub line_scroll_speed: f32,
    pub scroll_zoom_speed: f32,
    pub max_click_dist: f32,
    pub max_click_duration: f64,
    pub max_double_click_delay: f64,
    pub zoom_modifier: Modifiers,
    pub horizontal_scroll_modifier: Modifiers,
    pub vertical_scroll_modifier: Modifiers,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Style {
    pub override_text_style: Option<TextStyle>,
    pub override_font_id: Option<FontId>,
    pub override_text_valign: Option<Align>,
    pub text_styles: std::collections::BTreeMap<TextStyle, FontId>,
    pub drag_value_text_style: TextStyle,
    #[serde(default = "default_number_formatter", skip)]
    pub number_formatter: NumberFormatter,
    pub wrap: Option<bool>,
    pub wrap_mode: Option<crate::TextWrapMode>,
    pub spacing: Spacing,
    pub interaction: Interaction,
    pub visuals: Visuals,
    pub animation_time: f32,
    #[cfg(debug_assertions)]
    #[serde(skip)]
    pub debug: DebugOptions,
    pub explanation_tooltips: bool,
    pub url_in_tooltip: bool,
    pub always_scroll_the_only_direction: bool,
    pub scroll_animation: ScrollAnimation,
    pub compact_menu_style: bool,
}

fn default_number_formatter() -> NumberFormatter {
    NumberFormatter::new(emath::format_with_decimals_in_range)
}

type OrderMap = ahash::HashMap<LayerId, usize>;

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Areas {
    pub areas: IdMap<egui::AreaState>,
    pub visible_areas_last_frame: ahash::HashSet<LayerId>,
    pub visible_areas_current_frame: ahash::HashSet<LayerId>,
    pub order: Vec<LayerId>,
    pub order_map: OrderMap,
    pub wants_to_be_on_top: ahash::HashSet<LayerId>,
    pub sublayers: ahash::HashMap<LayerId, ahash::HashSet<LayerId>>,
}

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct InteractionState {
    pub potential_click_id: Option<Id>,
    pub potential_drag_id: Option<Id>,
}

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Focus {
    focused_widget: Option<FocusWidget>,
    id_previous_frame: Option<Id>,
    id_next_frame: Option<Id>,
    #[cfg(feature = "accesskit")]
    id_requested_by_accesskit: Option<accesskit::NodeId>,
    give_to_next: bool,
    last_interested: Option<Id>,
    focus_direction: FocusDirection,
    top_modal_layer: Option<LayerId>,
    top_modal_layer_current_frame: Option<LayerId>,
    focus_widgets_cache: IdMap<Rect>,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
struct FocusWidget {
    pub id: Id,
    pub filter: EventFilter,
}

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct EventFilter {
    pub tab: bool,
    pub horizontal_arrows: bool,
    pub vertical_arrows: bool,
    pub escape: bool,
}

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
enum FocusDirection {
    Up,
    Right,
    Down,
    Left,
    Previous,
    Next,
    #[default]
    None,
}

#[derive(Clone)]
pub struct NamedContextCallback {
    pub debug_name: &'static str,
    pub callback: ContextCallback,
}

pub struct Plugins {
    pub on_begin_pass: Vec<NamedContextCallback>,
    pub on_end_pass: Vec<NamedContextCallback>,
}

pub struct WrappedTextureManager(Arc<RwLock<epaint::TextureManager>>);

pub type ContextCallback = Arc<dyn Fn(&Context) + Send + Sync>;

pub struct ViewportState {
    pub class: ViewportClass,
    pub builder: ViewportBuilder,
    pub viewport_ui_cb: Option<Arc<DeferredViewportUiCallback>>,
    pub input: InputState,
    pub this_pass: PassState,
    pub prev_pass: PassState,
    pub used: bool,
    pub repaint: ViewportRepaintInfo,
    pub hits: WidgetHits,
    pub interact_widgets: InteractionSnapshot,
    pub graphics: GraphicLayers,
    pub output: PlatformOutput,
    pub commands: Vec<ViewportCommand>,
    pub num_multipass_in_row: usize,
}

pub struct ViewportRepaintInfo {
    pub cumulative_frame_nr: u64,
    pub cumulative_pass_nr: u64,
    pub repaint_delay: Duration,
    pub outstanding: u8,
    pub causes: Vec<RepaintCause>,
    pub prev_causes: Vec<RepaintCause>,
    pub prev_pass_paint_delay: Duration,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct DebugRect {
    pub rect: Rect,
    pub callstack: String,
    pub is_clicking: bool,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct PassState {
    pub used_ids: IdMap<Rect>,
    pub widgets: crate::private_hack::widget_rect::WidgetRects,
    pub layers: ahash::HashMap<LayerId, PerLayerState>,
    pub tooltips: TooltipPassState,
    pub available_rect: Rect,
    pub unused_rect: Rect,
    pub used_by_panels: Rect,
    pub scroll_target: [Option<ScrollTarget>; 2],
    pub scroll_delta: (Vec2, style::ScrollAnimation),
    #[cfg(feature = "accesskit")]
    #[serde(skip)]
    pub accesskit_state: Option<AccessKitPassState>,
    pub highlight_next_pass: IdSet,
    #[cfg(debug_assertions)]
    #[serde(skip)]
    pub debug_rect: Option<DebugRect>,
}

impl Default for PassState {
    fn default() -> Self {
        Self {
            used_ids: Default::default(),
            widgets: Default::default(),
            layers: Default::default(),
            tooltips: Default::default(),
            available_rect: Rect::NAN,
            unused_rect: Rect::NAN,
            used_by_panels: Rect::NAN,
            scroll_target: [None, None],
            scroll_delta: (Vec2::default(), style::ScrollAnimation::none()),
            #[cfg(feature = "accesskit")]
            accesskit_state: None,
            highlight_next_pass: Default::default(),

            #[cfg(debug_assertions)]
            debug_rect: None,
        }
    }
}

#[derive(Default)]
pub struct IdHasher(u64);

impl std::hash::Hasher for IdHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_u8(&mut self, _n: u8) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_u16(&mut self, _n: u16) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_u32(&mut self, _n: u32) {
        unreachable!("Invalid use of IdHasher");
    }

    #[inline(always)]
    fn write_u64(&mut self, n: u64) {
        self.0 = n;
    }

    fn write_usize(&mut self, _n: usize) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_i8(&mut self, _n: i8) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_i16(&mut self, _n: i16) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_i32(&mut self, _n: i32) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_i64(&mut self, _n: i64) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_isize(&mut self, _n: isize) {
        unreachable!("Invalid use of IdHasher");
    }

    #[inline(always)]
    fn finish(&self) -> u64 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct BuildIdHasher {}

impl std::hash::BuildHasher for BuildIdHasher {
    type Hasher = IdHasher;

    #[inline(always)]
    fn build_hasher(&self) -> IdHasher {
        IdHasher::default()
    }
}

pub type IdSet = std::collections::HashSet<Id, BuildIdHasher>;

pub type IdMap<V> = std::collections::HashMap<Id, V, BuildIdHasher>;

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct PerLayerState {
    pub open_popups: IdSet,
    pub widget_with_tooltip: Option<Id>,
}

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Sense(u8);


#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct TooltipPassState {
    pub widget_tooltips: IdMap<PerWidgetTooltipState>,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct PerWidgetTooltipState {
    pub bounding_rect: Rect,
    pub tooltip_count: usize,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ScrollTarget {
    pub range: Rangef,
    pub align: Option<Align>,
    pub animation: style::ScrollAnimation,
}
