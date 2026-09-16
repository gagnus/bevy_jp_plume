//! Static, non-interactive widgets: elements that only display data, plus the
//! passive layout gaps (`space`, `flex_spacer`) and the hairline `separator`.

mod caption;
mod flex_spacer;
mod separator;
mod space;
mod tooltip;

pub use caption::{caption, icon};
pub use flex_spacer::flex_spacer;
pub(crate) use separator::SeparatorPlugin;
pub use separator::{Separator, SeparatorBleed, separator};
pub use space::space;
pub use tooltip::{Tooltip, TooltipContent, TooltipSettings, TooltipWhenClipped};
pub(crate) use tooltip::{TooltipPlugin, TooltipShowing, TooltipUi, tooltip_box, tooltip_chrome};
