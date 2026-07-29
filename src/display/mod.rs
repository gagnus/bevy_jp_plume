//! Static widgets that only display data and are not interactive.

mod caption;
mod tooltip;

pub use caption::{caption, caption_color, caption_small_caps, fa_icon};
pub use tooltip::{Tooltip, TooltipContent, TooltipSettings};
pub(crate) use tooltip::{TooltipPlugin, TooltipShowing, TooltipUi, tooltip_box, tooltip_chrome};
