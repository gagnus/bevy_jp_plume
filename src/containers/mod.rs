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
mod tabs;

pub use column::column;
pub use dialog::{
    PlumeDialog, PlumeDialogBody, PlumeDialogBodyProps, PlumeDialogClose, PlumeDialogProps,
};
pub use flex_spacer::flex_spacer;
pub use row::row;
pub use screen::screen;
pub use section::{PlumeSection, PlumeSectionProps, SectionCollapsed};
pub use separator::separator;
pub use space::space;
pub use tabs::{PlumeTab, PlumeTabProps, PlumeTabs, PlumeTabsProps, TabTarget, tab_body};

pub(crate) use dialog::{CloseRequested, DialogChrome, DialogHeader, dialog_frame};
pub(crate) use popup::*;
pub(crate) use scroll_area::*;
pub(crate) use section::{SectionCollapsible, SectionPlugin, section_body, section_frame};
pub(crate) use tabs::{TabsPlugin, tab_button, tab_strip, tabs_frame};
