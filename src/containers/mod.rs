//! Meta-module containing all containers: passive widgets that hold other widgets.
mod column;
mod dialog;
mod flex_spacer;
mod group;
mod row;
mod screen;
mod scroll_area;
mod section;
mod separator;
mod space;

pub use column::*;
pub use dialog::*;
pub use flex_spacer::*;
pub use group::*;
pub use row::*;
pub use screen::*;
pub(crate) use scroll_area::*;
pub use section::*;
pub use separator::*;
pub use space::*;
