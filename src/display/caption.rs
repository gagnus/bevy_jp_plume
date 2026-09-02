//! BSN scene function for displaying a plain text string in the correct font.
use bevy::scene::{Scene, bsn};
use bevy::text::FontSourceTemplate;
use bevy::ui::widget::Text;

use crate::constants::Icon;
use crate::font_styles::InheritableFont;

/// A caption within, say, a button using inherited color.
pub fn caption(text: impl Into<String>) -> impl Scene {
    bsn! {
        Text(text)
    }
}

/// An icon glyph, drawn in the embedded icon font; size inherits, so icons
/// track the surrounding text.
pub fn icon(icon: Icon) -> impl Scene {
    let glyph = icon.glyph();
    let font_path = icon.font_path();
    bsn! {
        Text(glyph)
        InheritableFont {
            font: FontSourceTemplate::Handle(font_path),
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::MinimalPlugins;
    use bevy::app::App;
    use bevy::asset::{AssetApp, AssetPlugin};
    use bevy::scene::WorldSceneExt;

    use super::*;
    use crate::font_styles::{PlumeFontSize, small_caps};

    // Both modifiers patch one `InheritableFont`, so bsn has to merge the two
    // patches rather than let the second replace the first.
    #[test]
    fn caption_modifiers_compose() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::scene::ScenePlugin,
        ));
        app.init_asset::<bevy::text::Font>();
        let world = app.world_mut();
        let entity = world
            .spawn_scene(bsn! {
                @caption("Hello")
                @small_caps()
                InheritableFont { font_size: PlumeFontSize::Em(1.25) }
            })
            .expect("scene spawns")
            .id();
        let font = world
            .get::<InheritableFont>(entity)
            .expect("InheritableFont present");
        assert_eq!(
            font.font_size,
            Some(PlumeFontSize::Em(1.25)),
            "size survived"
        );
        assert!(font.font_features.is_some(), "features survived");
    }
}
