//! Immediate-mode API: write plain systems taking [`PlumeUi`] and call widgets on
//! it (`if ui.slider(&mut v, 0.0..=1.0).changed { … }`); a reconciler maps the
//! calls onto retained plume scenes.
mod caps;
mod response;
mod widgets;

pub use response::ImmResponse;
pub use widgets::PlumeImm;

use core::{
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU64, Ordering},
};

use bevy_app::Plugin;
use bevy_ecs::{
    change_detection::Tick,
    query::FilteredAccessSet,
    system::{SystemMeta, SystemParam, SystemParamValidationError},
    world::{World, unsafe_world_cell::UnsafeWorldCell},
};
use bevy_immediate::{
    BevyImmediatePlugin, Imm, ImmCtx, ImplCapsEmpty, impl_capability_set,
    ui::{
        activated::CapabilityUiActivated, base::CapabilityUiBase, disabled::CapabilityUiDisabled,
        interaction::CapabilityUiInteraction, layout_order::CapabilityUiLayoutOrder,
    },
};

use caps::{
    CapabilityPlumeChecked, CapabilityPlumeDialog, CapabilityPlumeSelect, CapabilityPlumeValue,
};

/// Capability set powering plume's immediate-mode layer.
pub struct PlumeCaps;

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
        CapabilityPlumeDialog,
    )
);

/// The immediate-mode context handed to container closures; all widget calls
/// live in [`PlumeImm`].
pub type Ui<'w, 's> = Imm<'w, 's, PlumeCaps>;

/// System param for immediate-mode UI systems: add a system taking `mut ui: PlumeUi`
/// to `Update` and call [`PlumeImm`] widgets on it directly.
pub struct PlumeUi<'w, 's> {
    imm: Ui<'w, 's>,
}

impl<'w, 's> Deref for PlumeUi<'w, 's> {
    type Target = Ui<'w, 's>;

    fn deref(&self) -> &Self::Target {
        &self.imm
    }
}

impl DerefMut for PlumeUi<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.imm
    }
}

type CtxStatic = ImmCtx<'static, 'static, PlumeCaps>;

/// State for [`PlumeUi`]: the inner context state plus a unique per-system root id.
pub struct PlumeUiState {
    ctx: <CtxStatic as SystemParam>::State,
    root_id: u64,
}

// SAFETY: delegates all access registration and fetching to `ImmCtx`'s impl;
// only wraps the fetched ctx into a rooted `Imm`.
unsafe impl SystemParam for PlumeUi<'_, '_> {
    type State = PlumeUiState;
    type Item<'w, 's> = PlumeUi<'w, 's>;

    fn init_state(world: &mut World) -> Self::State {
        static NEXT_ROOT_ID: AtomicU64 = AtomicU64::new(0);
        PlumeUiState {
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
        world: bevy_ecs::world::DeferredWorld,
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
        Ok(PlumeUi {
            imm: ctx.build_immediate_root(("plume_ui_root", state.root_id)),
        })
    }
}

/// Registers the immediate-mode reconciler.
pub(crate) struct ImmPlugin;

impl Plugin for ImmPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins(BevyImmediatePlugin::<PlumeCaps>::new());
    }
}
