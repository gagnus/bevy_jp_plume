//! Meta-module containing all containers: passive widgets that hold other widgets.
mod column;
mod dialog;
mod modal;
mod popup;
mod reorderable;
mod row;
mod screen;
mod scroll_area;
mod section;
mod splitter;
mod tabs;

pub use column::column;
pub use dialog::{CloseRequested, PlumeDialog, PlumeDialogProps};
pub(crate) use dialog::{DialogChrome, DialogHeader, DialogPlugin, dialog_body, dialog_frame};
pub(crate) use modal::{ModalPlugin, modal_barrier};
pub use modal::{PlumeModal, PlumeModalProps, modal_title};
pub(crate) use popup::*;
pub use popup::{
    PlumePopup, PlumePopupProps, PopupDismiss, PopupPlacement, PopupSocket, close_popup,
    popup_socket,
};
pub use reorderable::{
    PlumeReorderable, PlumeReorderableItem, PlumeReorderableItemProps, PlumeReorderableProps,
    ReorderMove,
};
pub(crate) use reorderable::{
    ReorderMailbox, ReorderablePlugin, reorderable_frame, reorderable_grip, reorderable_item,
};
pub use row::row;
pub use screen::{Screen, screen};
pub(crate) use scroll_area::*;
pub use scroll_area::{PlumeScrollArea, PlumeScrollAreaProps, ScrollAxis};
pub use section::{PlumeSection, PlumeSectionProps, SectionCollapsed};
pub(crate) use section::{SectionCollapsible, SectionPlugin, section_body, section_frame};
pub use splitter::{
    PlumeSplitter, PlumeSplitterProps, SplitAxis, SplitCollapsible, SplitDividerAutoHide, SplitMin,
    SplitPane, SplitSize, SplitSized,
};
pub(crate) use splitter::{SplitterPlugin, splitter_divider, splitter_frame, splitter_pane};
pub use tabs::{
    PlumeTab, PlumeTabProps, PlumeTabs, PlumeTabsProps, TabTarget, tab_body, tab_label,
};
pub(crate) use tabs::{TabsPlugin, tab_button, tab_chrome, tab_strip, tab_strip_frame, tabs_frame};
