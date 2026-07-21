//! Shared easing for the crate's UI micro-interactions: an [`AnimState`] glides a
//! `pos` toward a target each frame and applies it to the entity's transform.
use bevy_app::{Plugin, Update};
use bevy_camera::visibility::Visibility;
use bevy_ecs::{
    component::Component,
    reflect::ReflectComponent,
    system::{Query, Res},
};
use bevy_math::{Rot2, Vec2};
use bevy_reflect::Reflect;
use bevy_time::Time;
use bevy_ui::{UiTransform, px};

/// Exponential-approach rate for the crate's UI micro-transitions; higher settles
/// faster.
pub(crate) const UI_ANIM_RATE: f32 = 44.0;

/// Snap distance: exponential approach only nears the target asymptotically, so
/// within this the value jumps onto it and idle systems stop writing.
const SNAP_EPS: f32 = 0.001;

/// Move `current` toward `target` by one framerate-independent exponential step,
/// snapping on within [`SNAP_EPS`] so tweens terminate.
pub(crate) fn approach(current: f32, target: f32, rate: f32, dt: f32) -> f32 {
    let next = current + (target - current) * (1.0 - (-rate * dt).exp());
    if (next - target).abs() < SNAP_EPS {
        target
    } else {
        next
    }
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

/// The [`UiTransform`] channel an [`AnimState`] drives, with its `pos = 0` and
/// `pos = 1` endpoints.
#[derive(Clone, Copy, Reflect)]
pub(crate) enum AnimOutput {
    /// Uniform scale from `.0` to `.1`.
    Scale(f32, f32),
    /// X-translation in px from `.0` to `.1`.
    TranslateX(f32, f32),
    /// Rotation in radians from `.0` to `.1`.
    Rotation(f32, f32),
}

impl AnimOutput {
    fn apply(self, pos: f32, transform: &mut UiTransform) {
        match self {
            AnimOutput::Scale(from, to) => transform.scale = Vec2::splat(lerp(from, to, pos)),
            AnimOutput::TranslateX(from, to) => transform.translation.x = px(lerp(from, to, pos)),
            AnimOutput::Rotation(from, to) => {
                transform.rotation = Rot2::radians(lerp(from, to, pos))
            }
        }
    }
}

/// A UI micro-animation on an entity's [`UiTransform`]: `pos` eases toward
/// `target` (both `0..=1`) each frame, driving `output`. The owning control sets
/// `target`; [`advance_ui_anims`] owns `pos` and the transform write.
#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub(crate) struct AnimState {
    pos: f32,
    /// The end `pos` is easing toward, set by the owning control's style pass.
    pub(crate) target: f32,
    output: AnimOutput,
    hide_at_zero: bool,
}

impl AnimState {
    /// Ease uniform [`UiTransform`] scale between `from` (at rest) and `to`.
    pub(crate) fn scale(from: f32, to: f32) -> Self {
        Self::new(AnimOutput::Scale(from, to))
    }

    /// Ease [`UiTransform`] x-translation in px between `from` (at rest) and `to`.
    pub(crate) fn translate_x(from: f32, to: f32) -> Self {
        Self::new(AnimOutput::TranslateX(from, to))
    }

    /// Ease [`UiTransform`] rotation in radians between `from` (at rest) and `to`.
    pub(crate) fn rotation(from: f32, to: f32) -> Self {
        Self::new(AnimOutput::Rotation(from, to))
    }

    /// Also hide the entity ([`Visibility::Hidden`]) while `pos` is 0, so a
    /// collapsed scale reads as absent rather than a zero-size node.
    pub(crate) fn hide_at_zero(mut self) -> Self {
        self.hide_at_zero = true;
        self
    }

    fn new(output: AnimOutput) -> Self {
        Self {
            pos: 0.0,
            target: 0.0,
            output,
            hide_at_zero: false,
        }
    }
}

/// Ease every [`AnimState`] toward its target and apply it to the entity's
/// [`UiTransform`]. Post-layout, so it never triggers a relayout.
fn advance_ui_anims(
    time: Res<Time>,
    mut q_anims: Query<(&mut AnimState, &mut UiTransform, Option<&mut Visibility>)>,
) {
    let dt = time.delta_secs();
    for (mut anim, mut transform, visibility) in q_anims.iter_mut() {
        if anim.pos == anim.target {
            continue;
        }
        anim.pos = approach(anim.pos, anim.target, UI_ANIM_RATE, dt);
        anim.output.apply(anim.pos, &mut transform);
        if anim.hide_at_zero
            && let Some(mut visibility) = visibility
        {
            let wanted = if anim.pos > 0.0 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if *visibility != wanted {
                *visibility = wanted;
            }
        }
    }
}

/// Registers [`advance_ui_anims`], driving every [`AnimState`] in the app.
pub(crate) struct UiAnimPlugin;

impl Plugin for UiAnimPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(Update, advance_ui_anims);
    }
}
