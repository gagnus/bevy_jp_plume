//! Meta-module containing all containers: passive widgets that hold other widgets.
mod body;
mod column;
mod dialog;
mod flex_spacer;
mod popup;
mod row;
mod screen;
mod scroll_area;
mod section;
mod separator;
mod space;
mod splitter;
mod tabs;

pub(crate) use body::apply_body_style;
pub use body::{BodyGap, BodyPadding};
pub use column::column;
pub use dialog::{CloseRequested, PlumeDialog, PlumeDialogProps};
pub(crate) use dialog::{DialogChrome, DialogHeader, DialogPlugin, dialog_body, dialog_frame};
pub use flex_spacer::flex_spacer;
pub(crate) use popup::*;
pub use popup::{
    PlumePopup, PlumePopupProps, PopupDismiss, PopupPlacement, PopupSocket, close_popup,
    popup_socket,
};
pub use row::row;
pub use screen::{Screen, screen};
pub(crate) use scroll_area::*;
pub use scroll_area::{PlumeScrollArea, PlumeScrollAreaProps, ScrollAxis};
pub use section::{PlumeSection, PlumeSectionProps, SectionCollapsed};
pub(crate) use section::{SectionCollapsible, SectionPlugin, section_body, section_frame};
pub(crate) use separator::SeparatorPlugin;
pub use separator::{Separator, SeparatorBleed, separator};
pub use space::space;
pub use splitter::{
    PlumeSplitter, PlumeSplitterProps, SplitAxis, SplitCollapsible, SplitDividerAutoHide, SplitMin,
    SplitPane, SplitSize, SplitSized,
};
pub(crate) use splitter::{SplitterPlugin, splitter_divider, splitter_frame, splitter_pane};
pub use tabs::{
    PlumeTab, PlumeTabProps, PlumeTabs, PlumeTabsProps, TabTarget, tab_body, tab_label,
};
pub(crate) use tabs::{TabsPlugin, tab_button, tab_chrome, tab_strip, tab_strip_frame, tabs_frame};
