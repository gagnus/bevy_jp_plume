//! Meta-module containing all containers: passive widgets that hold other widgets.
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

pub use column::column;
pub(crate) use dialog::{CloseRequested, DialogChrome, DialogHeader, dialog_frame};
pub use dialog::{
    PlumeDialog, PlumeDialogBody, PlumeDialogBodyProps, PlumeDialogClose, PlumeDialogProps,
};
pub use flex_spacer::flex_spacer;
pub(crate) use popup::*;
pub use popup::{
    PlumePopup, PlumePopupProps, PopupDismiss, PopupPlacement, PopupSocket, close_popup,
    popup_socket,
};
pub use row::row;
pub use screen::screen;
pub(crate) use scroll_area::*;
pub use scroll_area::{PlumeScrollArea, PlumeScrollAreaProps};
pub use section::{PlumeSection, PlumeSectionProps, SectionCollapsed};
pub(crate) use section::{SectionCollapsible, SectionPlugin, section_body, section_frame};
pub use separator::separator;
pub use space::space;
pub use splitter::{PlumeSplitter, PlumeSplitterProps, SplitAxis, SplitFraction, SplitMin};
pub(crate) use splitter::{
    SplitPane, SplitterPlugin, splitter_divider, splitter_frame, splitter_pane,
};
pub use tabs::{PlumeTab, PlumeTabProps, PlumeTabs, PlumeTabsProps, TabTarget, tab_body};
pub(crate) use tabs::{TabsPlugin, tab_button, tab_strip, tabs_frame};
