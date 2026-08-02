//! Immediate-mode API: write plain systems taking [`Ui`] and call widgets on
//! it (`if ui.slider(&mut v, 0.0..=1.0).changed { … }`); a reconciler maps the
//! calls onto retained plume scenes.
mod caps;
mod response;
mod widgets;

use core::sync::atomic::{AtomicU64, Ordering};

use bevy::app::Plugin;
use bevy::ecs::change_detection::Tick;
use bevy::ecs::query::FilteredAccessSet;
use bevy::ecs::system::{SystemMeta, SystemParam, SystemParamValidationError};
use bevy::ecs::world::World;
use bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy_immediate::{
    BevyImmediatePlugin, Imm, ImmCtx, ImmEntity, ImmId, ImmIdBuilder, ImmScopeGuard,
};
use caps::PlumeOccurrences;
pub use response::{ImmResponse, kind};
pub use widgets::{Corner, ImmDialog, ImmPanel, ImmPopup, ImmSelect, ImmTab, ImmTabs, PlumeImm};

pub use crate::utils::numeric::Numeric;

/// Capability set powering plume's immediate-mode layer.
#[doc(hidden)]
pub struct PlumeCaps;

// `impl_capability_set!` emits a bare `pub trait`, which no attribute can reach;
// a private module keeps it unnameable. Trait impls are not module-scoped, so
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
        CapabilityPlumeSelect, CapabilityPlumeText, CapabilityPlumeTooltip, CapabilityPlumeValue,
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
        )
    );
}

/// The immediate-mode context handed to container closures; all widget calls
/// live in [`PlumeImm`].
///
/// Wraps the reconciler rather than aliasing it, so the vendored `bevy_immediate`
/// stays a private dependency and its API is not part of plume's.
#[repr(transparent)]
pub struct Ui<'w, 's>(pub(crate) Imm<'w, 's, PlumeCaps>);

impl<'w, 's> Ui<'w, 's> {
    // The reconciler hands closures a `&mut Imm`; `repr(transparent)` makes the
    // reference cast to the wrapper layout-identical, which is the only reason
    // container bodies can take a `&mut Ui`.
    pub(crate) fn wrap_mut<'a>(imm: &'a mut Imm<'w, 's, PlumeCaps>) -> &'a mut Self {
        // SAFETY: `Ui` is `repr(transparent)` over exactly this type.
        unsafe { &mut *(imm as *mut Imm<'w, 's, PlumeCaps> as *mut Self) }
    }

    // The slice of the reconciler plume's own widgets build on. Forwarded rather
    // than exposed, so none of it reaches an app.
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

/// System param for immediate-mode UI: open a top-level surface —
/// [`screen`](Self::screen) or [`dialog`](Self::dialog) — to get the [`Ui`] that
/// [`PlumeImm`] widgets are called on.
///
/// Widgets are deliberately unreachable at root scope: the root is virtual, so a
/// widget there would anchor at the viewport origin with no font ancestor, both
/// silently. A `Ui` is only handed out inside a surface that fixes them.
pub struct PlumeRoot<'w, 's> {
    imm: Ui<'w, 's>,
}

impl<'w, 's> PlumeRoot<'w, 's> {
    /// Full-screen surface for a system's top-level content: a transparent,
    /// padded column filling the viewport that establishes the standard font and
    /// text color. The usual choice for screen-filling UI.
    #[track_caller]
    pub fn screen(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Screen> {
        self.imm.screen(f)
    }

    /// Movable floating dialog — the other top-level surface. Configure via the
    /// returned [`ImmDialog`] and build the body with [`ImmDialog::show`]; the
    /// dialog is absolutely positioned, so it stands alone at root scope.
    #[track_caller]
    pub fn dialog<'a>(&'a mut self, title: &str, open: &'a mut bool) -> ImmDialog<'a, 'w, 's> {
        self.imm.dialog(title, open)
    }

    /// Headerless floating surface — a [`Self::dialog`] with no title bar. Configure
    /// via the returned [`ImmPanel`] and build the body with [`ImmPanel::show`].
    #[track_caller]
    pub fn panel(&mut self) -> ImmPanel<'_, 'w, 's> {
        self.imm.panel()
    }

    /// Scope surface ids by `id`, for keying several screens/dialogs built in a
    /// loop by data rather than call order. See [`PlumeImm::push_id`].
    pub fn push_id<R>(
        &mut self,
        id: impl core::hash::Hash,
        f: impl FnOnce(&mut Ui<'w, 's>) -> R,
    ) -> R {
        self.imm.push_id(id, f)
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
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        <CtxStatic as SystemParam>::init_access(
            &state.ctx,
            system_meta,
            component_access_set,
            world,
        );
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
        if let Ok(mut occurrences) = imm
            .ctx_mut()
            .cap_resources
            .resources
            .get_mut::<PlumeOccurrences>()
        {
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
