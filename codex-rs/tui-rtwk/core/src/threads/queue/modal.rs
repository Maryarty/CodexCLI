//! Native dialog results for queue holds. The app issues a token whose type fixes the
//! result a particular dialog can return.

use std::marker::PhantomData;

use super::QueueOperation;
use crate::app::effects::CopyFormat;
use crate::ids::ThreadId;
use crate::screens::chat::surface_items::STATUS_LINE;
use crate::screens::select::{apps, permissions, quick_dials};

/// A picker can choose a clipboard value. The thread and destination belong to its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CopyChoice {
    pub(crate) text: String,
    pub(crate) label: String,
    pub(crate) format: CopyFormat,
}

impl CopyChoice {
    pub fn plain_text(text: String, label: String) -> Self {
        Self {
            text,
            label,
            format: CopyFormat::PlainText,
        }
    }

    pub fn markdown(text: String, label: String) -> Self {
        Self {
            text,
            label,
            format: CopyFormat::Markdown,
        }
    }
}

/// The selected fields in display order and the status line's own colour preference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusLineChoice {
    pub(crate) items: Vec<String>,
    pub(crate) use_theme_colors: bool,
}

impl StatusLineChoice {
    pub fn new(items: Vec<String>, use_theme_colors: bool) -> Option<Self> {
        items
            .iter()
            .all(|item| STATUS_LINE.contains(&item.as_str()))
            .then_some(Self {
                items,
                use_theme_colors,
            })
    }
}

/// Which effect the core owner authorized when it registered the hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ModalKind {
    Copy,
    StatusLine,
    Permissions,
    Apps,
    Preset,
}

/// An operation issued by the queue owner. `C` fixes the result type of the native dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModalToken<C> {
    operation: QueueOperation,
    choice: PhantomData<fn(C)>,
}

impl<C> ModalToken<C> {
    pub fn thread(&self) -> &ThreadId {
        self.operation.thread()
    }

    fn new(operation: QueueOperation) -> Self {
        Self {
            operation,
            choice: PhantomData,
        }
    }
}

/// The registered operation handed to a presenter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModalOperation {
    Copy(ModalToken<CopyChoice>),
    StatusLine(ModalToken<StatusLineChoice>),
    Permissions(ModalToken<permissions::Selection>),
    Apps(ModalToken<apps::ModalChoice>),
    Preset(ModalToken<quick_dials::Write>),
}

impl ModalOperation {
    pub(crate) fn new(kind: ModalKind, operation: QueueOperation) -> Self {
        match kind {
            ModalKind::Copy => Self::Copy(ModalToken::new(operation)),
            ModalKind::StatusLine => Self::StatusLine(ModalToken::new(operation)),
            ModalKind::Permissions => Self::Permissions(ModalToken::new(operation)),
            ModalKind::Apps => Self::Apps(ModalToken::new(operation)),
            ModalKind::Preset => Self::Preset(ModalToken::new(operation)),
        }
    }

    pub fn thread(&self) -> &ThreadId {
        self.operation().thread()
    }

    pub fn cancelled(self) -> ModalReceipt {
        match self {
            Self::Copy(token) => ModalReceipt::Copy(token, None),
            Self::StatusLine(token) => ModalReceipt::StatusLine(token, None),
            Self::Permissions(token) => ModalReceipt::Permissions(token, None),
            Self::Apps(token) => ModalReceipt::Apps(token, None),
            Self::Preset(token) => ModalReceipt::Preset(token, None),
        }
    }

    pub(super) fn operation(&self) -> &QueueOperation {
        match self {
            Self::Copy(token) => &token.operation,
            Self::StatusLine(token) => &token.operation,
            Self::Permissions(token) => &token.operation,
            Self::Apps(token) => &token.operation,
            Self::Preset(token) => &token.operation,
        }
    }

    pub(super) fn kind(&self) -> ModalKind {
        match self {
            Self::Copy(_) => ModalKind::Copy,
            Self::StatusLine(_) => ModalKind::StatusLine,
            Self::Permissions(_) => ModalKind::Permissions,
            Self::Apps(_) => ModalKind::Apps,
            Self::Preset(_) => ModalKind::Preset,
        }
    }
}

/// A terminal result pairs the issued token with its dialog's selected value or cancellation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModalReceipt {
    Copy(ModalToken<CopyChoice>, Option<CopyChoice>),
    StatusLine(ModalToken<StatusLineChoice>, Option<StatusLineChoice>),
    Permissions(
        ModalToken<permissions::Selection>,
        Option<permissions::Selection>,
    ),
    Apps(ModalToken<apps::ModalChoice>, Option<apps::ModalChoice>),
    Preset(ModalToken<quick_dials::Write>, Option<quick_dials::Write>),
}

impl ModalReceipt {
    pub(crate) fn modal(&self) -> ModalOperation {
        match self {
            Self::Copy(token, _) => ModalOperation::Copy(token.clone()),
            Self::StatusLine(token, _) => ModalOperation::StatusLine(token.clone()),
            Self::Permissions(token, _) => ModalOperation::Permissions(token.clone()),
            Self::Apps(token, _) => ModalOperation::Apps(token.clone()),
            Self::Preset(token, _) => ModalOperation::Preset(token.clone()),
        }
    }

    pub fn thread(&self) -> &ThreadId {
        self.operation().thread()
    }

    pub(super) fn operation(&self) -> &QueueOperation {
        match self {
            Self::Copy(token, _) => &token.operation,
            Self::StatusLine(token, _) => &token.operation,
            Self::Permissions(token, _) => &token.operation,
            Self::Apps(token, _) => &token.operation,
            Self::Preset(token, _) => &token.operation,
        }
    }

    pub(super) fn kind(&self) -> ModalKind {
        match self {
            Self::Copy(..) => ModalKind::Copy,
            Self::StatusLine(..) => ModalKind::StatusLine,
            Self::Permissions(..) => ModalKind::Permissions,
            Self::Apps(..) => ModalKind::Apps,
            Self::Preset(..) => ModalKind::Preset,
        }
    }
}
