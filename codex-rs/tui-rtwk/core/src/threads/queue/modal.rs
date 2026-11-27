//! Native dialog results for queue holds. The app issues a token whose type fixes the
//! result a particular dialog can return.

use std::marker::PhantomData;

use super::QueueOperation;
use crate::app::effects::CopyFormat;
use crate::ids::ThreadId;
use crate::screens::chat::surface_items::STATUS_LINE;

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
}

/// An operation issued by the queue owner. `C` fixes the result type of the native dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModalToken<C> {
    operation: QueueOperation,
    choice: PhantomData<fn(C)>,
}

impl<C> ModalToken<C> {
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
}

impl ModalOperation {
    pub(crate) fn new(kind: ModalKind, operation: QueueOperation) -> Self {
        match kind {
            ModalKind::Copy => Self::Copy(ModalToken::new(operation)),
            ModalKind::StatusLine => Self::StatusLine(ModalToken::new(operation)),
        }
    }

    pub fn thread(&self) -> &ThreadId {
        self.operation().thread()
    }

    pub fn cancelled(self) -> ModalReceipt {
        match self {
            Self::Copy(token) => ModalReceipt::Copy(token, None),
            Self::StatusLine(token) => ModalReceipt::StatusLine(token, None),
        }
    }

    pub(super) fn operation(&self) -> &QueueOperation {
        match self {
            Self::Copy(token) => &token.operation,
            Self::StatusLine(token) => &token.operation,
        }
    }

    pub(super) fn kind(&self) -> ModalKind {
        match self {
            Self::Copy(_) => ModalKind::Copy,
            Self::StatusLine(_) => ModalKind::StatusLine,
        }
    }
}

/// A terminal result pairs the issued token with its dialog's selected value or cancellation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModalReceipt {
    Copy(ModalToken<CopyChoice>, Option<CopyChoice>),
    StatusLine(ModalToken<StatusLineChoice>, Option<StatusLineChoice>),
}

impl ModalReceipt {
    pub fn thread(&self) -> &ThreadId {
        self.operation().thread()
    }

    pub(super) fn operation(&self) -> &QueueOperation {
        match self {
            Self::Copy(token, _) => &token.operation,
            Self::StatusLine(token, _) => &token.operation,
        }
    }

    pub(super) fn kind(&self) -> ModalKind {
        match self {
            Self::Copy(..) => ModalKind::Copy,
            Self::StatusLine(..) => ModalKind::StatusLine,
        }
    }
}
