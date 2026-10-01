//! Immediate-mode UI: describe your UI every frame from an ordinary system, and
//! plume keeps the real widgets on screen in step with it.
//!
//! If you have used egui or Dear ImGui this will feel familiar. You don't spawn
//! widgets, hold on to them, or listen for their events. You call
//! `ui.slider(&mut volume, 0.0..=1.0)` every frame: the slider shows `volume`, and
//! when the user drags it, `volume` changes.
//!
//! # A first dialog
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_plume::prelude::*;
//!
//! #[derive(Resource, Clone, PartialEq)]
//! struct Audio {
//!     open: bool,
//!     volume: f32,
//!     muted: bool,
//! }
//!
//! fn audio_dialog(mut root: PlumeRoot, mut audio: ResMut<Audio>) {
//!     // Edit a copy, so bevy only sees the resource change when something did.
//!     let mut a = audio.clone();
//!     root.dialog("Audio", &mut a.open).show(|ui| {
//!         ui.horizontal(|ui| {
//!             ui.caption("Volume");
//!             ui.slider(&mut a.volume, 0.0..=1.0).enabled(!a.muted);
//!         });
//!         ui.checkbox(&mut a.muted, "Mute");
//!         if ui.button("Reset").clicked {
//!             a.volume = 0.5;
//!             a.muted = false;
//!         }
//!     });
//!     audio.set_if_neq(a);
//! }
//!
//! fn main() {
//!     App::new()
//!         .add_plugins((DefaultPlugins, PlumePlugins))
//!         .insert_resource(Audio { open: true, volume: 0.5, muted: false })
//!         .add_systems(Startup, |mut commands: Commands| {
//!             commands.spawn(Camera2d);
//!         })
//!         .add_systems(Update, audio_dialog)
//!         .run();
//! }
//! ```
//!
//! Three names cover most of it:
//!
//! - [`PlumeRoot`] is what your system asks for. It opens the things that sit
//!   straight on the screen: a [`screen`](PlumeRoot::screen), a
//!   [`dialog`](PlumeRoot::dialog), a [`modal`](PlumeRoot::modal) or a
//!   [`panel`](PlumeRoot::panel).
//! - [`Ui`] is what each of those hands to your closure. Every widget is a method
//!   on it, and containers such as `horizontal` or `section` pass a `Ui` on to a
//!   closure of their own. The methods live in the [`PlumeImm`] trait, which the
//!   prelude brings in.
//! - [`ImmResponse`] is what every widget call gives back. Ask it what happened
//!   (`.clicked`, `.changed`) or tack on extras (`.enabled(false)`,
//!   `.tooltip("…")`).
//!
//! # What happens behind the scenes
//!
//! Unlike egui, nothing is rebuilt from scratch each frame. The first time a
//! widget call runs, plume spawns a real `bevy_ui` widget for it. On later frames
//! the same call finds that same widget again and only updates what is different.
//! When a frame goes by without the call being made, the widget is removed.
//!
//! That bookkeeping is the work of [bevy_immediate], credited [below].
//!
//! Two things follow from that:
//!
//! - Widgets remember things for you. A dialog stays where the user dragged it, a
//!   scroll area keeps its place, a text box keeps its cursor. None of it needs
//!   storing on your side.
//! - Showing something only some of the time is just an `if`. Skip the call and
//!   the widget goes away; make it again and a fresh one appears. Dialogs lean on
//!   the same idea: `root.dialog(title, &mut open)` shows nothing while `open` is
//!   false, and the ✕ button sets it to false for you.
//!
//! # How plume tells widgets apart
//!
//! A widget is recognized by the line of code that made it and the container it
//! is in. That is why an `if` around one widget doesn't disturb the ones after
//! it. A call inside a loop makes one widget each time round, told apart by
//! order: first, second, third.
//!
//! Order is the wrong thing to go by when the list itself gets shuffled, or loses
//! entries from the middle - the third widget's memory would end up attached to a
//! different item. Wrap each entry in [`push_id`](PlumeImm::push_id) with
//! something that names the item (an id, a key) and its widgets follow the item
//! instead.
//!
//! # Who wins, you or the user?
//!
//! Widgets that edit a value work in both directions, so there has to be a rule
//! for when both sides want to change it:
//!
//! - Whatever the user does always reaches your value, and `.changed` is true on
//!   the frame it lands.
//! - When you change the value yourself, the widget updates to show it.
//! - The exception is while the user is in the middle of something - dragging a
//!   slider, typing in a text box. Your change is held back until they finish, so
//!   the widget never jumps out from under their hands.
//!
//! # When plume doesn't have the widget you need
//!
//! [`scene`](PlumeImm::scene) drops a hand-built `bsn!` scene into the middle of
//! an immediate-mode layout, and [`retained`](crate::retained) has the building
//! blocks to make one from - the same ones every widget here is made of.
//!
//! # Built on bevy_immediate
//!
//! The immediate-mode engine underneath this module is [bevy_immediate] by
//! Pēteris Pakalns, used under its MIT license. It does the part described in
//! "What happens behind the scenes": remembering which call made which entity,
//! spawning one the first time a call is seen, and removing it when the call
//! stops coming. Plume could not have been written this way without it.
//!
//! What plume adds on top is everything specific to its own widgets: the widget
//! methods themselves, the theming, the who-wins rules for values, and telling
//! repeated calls in a loop apart.
//!
//! Plume keeps bevy_immediate out of sight so that an app only ever imports
//! `bevy_plume`, which means you won't meet its types here. If what you want is
//! immediate mode over your own `bevy_ui` widgets rather than plume's, go straight
//! to bevy_immediate - it is built to be extended that way.
//!
//! [bevy_immediate]: https://github.com/PPakalns/bevy_immediate
//! [below]: #built-on-bevy_immediate
mod caps;
mod data;
mod response;
mod widgets;

use core::sync::atomic::{AtomicU64, Ordering};

use bevy::app::Plugin;
use bevy::ecs::change_detection::Tick;
use bevy::ecs::system::{SystemAccess, SystemMeta, SystemParam, SystemParamValidationError};
use bevy::ecs::world::World;
use bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy::picking::hover::Hovered;
use bevy_immediate::{
    BevyImmediatePlugin, Imm, ImmCtx, ImmEntity, ImmId, ImmIdBuilder, ImmScopeGuard,
};
use caps::PlumeOccurrences;
pub use data::ImmDataAppExt;
pub use response::{ImmResponse, kind};
pub use widgets::{
    Corner, Floating, ImmDialog, ImmMenu, ImmMenuBar, ImmPopup, ImmSelect, ImmTab, ImmTabs, Modal,
    Panel, Placed, PlumeImm, Titled, tab_header,
};

use crate::containers::SplitSize;
pub use crate::utils::numeric::Numeric;

/// Capability set powering plume's immediate-mode layer.
#[doc(hidden)]
pub struct PlumeCaps;

// `impl_capability_set!` emits a bare `pub trait`, which no attribute can reach; a
// private module keeps it unnameable. Trait impls aren't module-scoped, so
// `PlumeCaps`' capabilities still apply crate-wide.
mod cap_set {
    use bevy_immediate::ui::activated::CapabilityUiActivated;
    use bevy_immediate::ui::base::CapabilityUiBase;
    use bevy_immediate::ui::disabled::CapabilityUiDisabled;
    use bevy_immediate::ui::interaction::CapabilityUiInteraction;
    use bevy_immediate::ui::layout_order::CapabilityUiLayoutOrder;
    use bevy_immediate::{ImplCapsEmpty, impl_capability_set};

    use super::PlumeCaps;
    use super::caps::{
        CapabilityPlumeChecked, CapabilityPlumeColor, CapabilityPlumeDialog, CapabilityPlumeIds,
        CapabilityPlumeMenu, CapabilityPlumeReorder, CapabilityPlumeSelect, CapabilityPlumeText,
        CapabilityPlumeTooltip, CapabilityPlumeValue,
    };

    impl_capability_set!(
        PlumeCaps,
        ImplPlumeCaps > ImplCapsEmpty,
        (
            CapabilityUiBase,
            CapabilityUiLayoutOrder,
            CapabilityUiInteraction,
            CapabilityUiActivated,
            CapabilityUiDisabled,
            CapabilityPlumeValue,
            CapabilityPlumeChecked,
            CapabilityPlumeSelect,
            CapabilityPlumeText,
            CapabilityPlumeColor,
            CapabilityPlumeDialog,
            CapabilityPlumeIds,
            CapabilityPlumeTooltip,
            CapabilityPlumeMenu,
            CapabilityPlumeReorder,
        )
    );
}

/// The immediate-mode context handed to container closures; all widget calls live
/// in [`PlumeImm`].
#[repr(transparent)]
pub struct Ui<'w, 's>(pub(crate) Imm<'w, 's, PlumeCaps>);

impl<'w, 's> Ui<'w, 's> {
    /// Is the pointer over the container this scope is filling, or anything inside it
    /// - so chrome the container reveals doesn't vanish as the pointer reaches it.
    pub fn hovered(&mut self) -> bool {
        let Some(parent) = self.0.current_entity() else {
            return false;
        };
        if let Ok(entity) = self.0.ctx().cap_entities.get(parent) {
            if let Some(hovered) = entity.get::<Hovered>() {
                return hovered.get();
            }
        } else {
            return false;
        }
        // Containers that never ask about hover don't carry `Hovered`; seed it so
        // picking starts tracking, answered from the next frame on.
        self.0
            .ctx_mut()
            .commands
            .entity(parent)
            .insert(Hovered::default());
        false
    }

    // The reconciler hands closures a `&mut Imm`; `repr(transparent)` makes the cast
    // to the wrapper layout-identical, which is why container bodies take `&mut Ui`.
    pub(crate) fn wrap_mut<'a>(imm: &'a mut Imm<'w, 's, PlumeCaps>) -> &'a mut Self {
        // SAFETY: `Ui` is `repr(transparent)` over exactly this type.
        unsafe { &mut *(imm as *mut Imm<'w, 's, PlumeCaps> as *mut Self) }
    }

    // The slice of the reconciler plume's own widgets build on.
    pub(crate) fn ch_id<T: core::hash::Hash>(&mut self, id: T) -> ImmEntity<'_, 'w, 's, PlumeCaps> {
        self.0.ch_id(id)
    }

    pub(crate) fn ch_with_manual_id(
        &mut self,
        id: ImmIdBuilder,
    ) -> ImmEntity<'_, 'w, 's, PlumeCaps> {
        self.0.ch_with_manual_id(id)
    }

    pub(crate) fn current_imm_id(&self) -> ImmId {
        self.0.current_imm_id()
    }

    pub(crate) fn ctx_mut(&mut self) -> &mut ImmCtx<'w, 's, PlumeCaps> {
        self.0.ctx_mut()
    }

    pub(crate) fn with_add_id_pref(
        &mut self,
        id: impl core::hash::Hash,
    ) -> ImmScopeGuard<'_, 'w, 's, PlumeCaps> {
        self.0.with_add_id_pref(id)
    }
}

// Container bodies take a `&mut Ui`; the reconciler's own `add` hands out a
// `&mut Imm`.
pub(crate) trait ImmEntityExt<'w, 's> {
    fn add_ui(self, f: impl FnOnce(&mut Ui<'w, 's>)) -> Self;
    fn unrooted_ui<T: core::hash::Hash>(self, id: T, f: impl FnOnce(&mut Ui<'w, 's>)) -> Self;
}

impl<'r, 'w, 's> ImmEntityExt<'w, 's> for ImmEntity<'r, 'w, 's, PlumeCaps> {
    fn add_ui(self, f: impl FnOnce(&mut Ui<'w, 's>)) -> Self {
        self.add(|imm| f(Ui::wrap_mut(imm)))
    }

    fn unrooted_ui<T: core::hash::Hash>(self, id: T, f: impl FnOnce(&mut Ui<'w, 's>)) -> Self {
        self.unrooted(id, |imm| f(Ui::wrap_mut(imm)))
    }
}

/// System param for immediate-mode UI: open a top-level surface -
/// [`screen`](Self::screen) or [`dialog`](Self::dialog) - to get the [`Ui`] that
/// [`PlumeImm`] widgets are called on.
///
/// Widgets are unreachable at root scope: the root is virtual, so one there would
/// anchor at the viewport origin with no font ancestor, both silently.
#[repr(transparent)]
pub struct PlumeRoot<'w, 's> {
    pub(crate) imm: Ui<'w, 's>,
}

impl<'w, 's> PlumeRoot<'w, 's> {
    // A scoped root over a borrowed `Ui`, the way `Ui::wrap_mut` sits over an
    // `Imm`: `repr(transparent)` makes the cast layout-identical.
    pub(crate) fn wrap_mut<'a>(ui: &'a mut Ui<'w, 's>) -> &'a mut Self {
        // SAFETY: `PlumeRoot` is `repr(transparent)` over exactly this type.
        unsafe { &mut *(ui as *mut Ui<'w, 's> as *mut Self) }
    }

    /// Full-screen surface for a system's top-level content: a transparent, padded
    /// column filling the viewport, establishing the standard font and text color.
    #[track_caller]
    pub fn screen(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Screen> {
        self.imm.screen(f)
    }

    /// Movable floating dialog - the other top-level surface. Configure via the
    /// returned [`ImmDialog`] and build the body with [`ImmDialog::show`].
    #[track_caller]
    pub fn dialog<'a>(
        &'a mut self,
        title: &str,
        open: &'a mut bool,
    ) -> ImmDialog<'a, 'w, 's, Floating> {
        self.imm.dialog(title, open)
    }

    /// Modal dialog - centered over a barrier that blocks the app behind it until
    /// it is answered. The same [`ImmDialog`] builder in its [`Modal`] mode, minus
    /// the placement and drag a centered surface has no use for.
    #[track_caller]
    pub fn modal<'a>(
        &'a mut self,
        title: &str,
        open: &'a mut bool,
    ) -> ImmDialog<'a, 'w, 's, Modal> {
        self.imm.modal(title, open)
    }

    /// Headerless floating surface - a [`Self::dialog`] with no title bar, ✕ or
    /// drag. Top-level like the other two: it floats over the app, pinned to the
    /// viewport or, via [`ImmDialog::at_corner_of`], to another entity's rect.
    /// The same [`ImmDialog`] builder in its [`Panel`] mode.
    #[track_caller]
    pub fn panel(&mut self) -> ImmDialog<'_, 'w, 's, Panel> {
        self.imm.panel()
    }

    /// Scope surface ids by `id`, for keying several screens/dialogs built in a
    /// loop by data rather than call order - the closure gets a root scoped to
    /// `id`, so it can build surfaces and nothing else. The `Ui` counterpart is
    /// [`PlumeImm::push_id`].
    pub fn push_id<R>(
        &mut self,
        id: impl core::hash::Hash,
        f: impl FnOnce(&mut PlumeRoot<'w, 's>) -> R,
    ) -> R {
        let mut scope = self.imm.with_add_id_pref(id);
        f(PlumeRoot::wrap_mut(Ui::wrap_mut(&mut scope)))
    }
}

/// The [`Ui`] a splitter hands each of its panes: a `Ui` in every respect, plus the
/// divider's settled [`split`](Self::split), which the app's own binding cannot
/// supply while it is on loan to the [`split_horizontal`](PlumeImm::split_horizontal) call.
pub struct PaneUi<'a, 'w, 's> {
    pub(crate) ui: &'a mut Ui<'w, 's>,
    pub(crate) split: SplitSize,
}

impl PaneUi<'_, '_, '_> {
    /// The splitter's state as the divider has settled it: the sized pane's
    /// length, and which pane a drag has [closed](ImmResponse::collapsible), if any.
    pub fn split(&self) -> SplitSize {
        self.split
    }
}

impl<'w, 's> core::ops::Deref for PaneUi<'_, 'w, 's> {
    type Target = Ui<'w, 's>;

    fn deref(&self) -> &Self::Target {
        self.ui
    }
}

impl core::ops::DerefMut for PaneUi<'_, '_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ui
    }
}

type CtxStatic = ImmCtx<'static, 'static, PlumeCaps>;

/// State for [`PlumeRoot`]: the inner context state plus a unique per-system root id.
#[doc(hidden)]
pub struct PlumeRootState {
    ctx: <CtxStatic as SystemParam>::State,
    root_id: u64,
}

// SAFETY: delegates all access registration and fetching to `ImmCtx`'s impl;
// only wraps the fetched ctx into a rooted `Imm`.
unsafe impl SystemParam for PlumeRoot<'_, '_> {
    type State = PlumeRootState;
    type Item<'w, 's> = PlumeRoot<'w, 's>;

    fn init_state(world: &mut World) -> Self::State {
        static NEXT_ROOT_ID: AtomicU64 = AtomicU64::new(0);
        PlumeRootState {
            ctx: <CtxStatic as SystemParam>::init_state(world),
            root_id: NEXT_ROOT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        system_access: &mut SystemAccess,
        world: &mut World,
    ) {
        <CtxStatic as SystemParam>::init_access(&state.ctx, system_meta, system_access, world);
    }

    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        <CtxStatic as SystemParam>::apply(&mut state.ctx, system_meta, world);
    }

    fn queue(
        state: &mut Self::State,
        system_meta: &SystemMeta,
        world: bevy::ecs::world::DeferredWorld,
    ) {
        <CtxStatic as SystemParam>::queue(&mut state.ctx, system_meta, world);
    }

    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Result<Self::Item<'w, 's>, SystemParamValidationError> {
        // SAFETY: forwarded verbatim; access was registered by `init_access` above.
        let ctx = unsafe {
            <CtxStatic as SystemParam>::get_param(&mut state.ctx, system_meta, world, change_tick)
        }?;
        let mut imm = ctx.build_immediate_root(("plume_ui_root", state.root_id));
        // Occurrence counts are per pass: this system run starts fresh, so a
        // widget built at the same call site as last frame keeps its id.
        if let Ok(mut occurrences) = imm.ctx_mut().cap_resources.get_mut::<PlumeOccurrences>() {
            occurrences.0.clear();
        }
        Ok(PlumeRoot { imm: Ui(imm) })
    }
}

// Registers the immediate-mode reconciler.
pub(crate) struct ImmPlugin;

impl Plugin for ImmPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins(BevyImmediatePlugin::<PlumeCaps>::new());
    }
}
