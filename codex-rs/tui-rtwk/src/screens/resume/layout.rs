//! The picker's pure geometry and ages: the chrome's rows and relative ages.
//! Every function is a port of the oracle's, named after it; none touches a
//! widget or a string table (the view passes the texts in).
//! Oracle: `resume_picker.rs` and `resume_picker/layout.rs`.

/// The rows of the picker's chrome (oracle: `layout::areas`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Areas {
    pub toolbar: u16,
    pub search: u16,
    pub list: u16,
    pub list_height: u16,
    pub footer: u16,
}

const FIXED_CHROME_ROWS: u16 = 7;
const MIN_LIST_ROWS: u16 = 6;
/// The list sits two cells in (oracle: `PICKER_LIST_HORIZONTAL_INSET`).
pub const LIST_INSET: u16 = 2;

/// Decorative gaps collapse before the list loses room for two comfortable
/// rows.
pub fn areas(height: u16) -> Areas {
    let gaps = height
        .saturating_sub(FIXED_CHROME_ROWS + MIN_LIST_ROWS)
        .min(3);
    let toolbar = 1 + u16::from(gaps >= 3);
    let search = toolbar + 1 + u16::from(gaps >= 2);
    let list = search + 1 + u16::from(gaps >= 1);
    let list_height = height.saturating_sub(FIXED_CHROME_ROWS + gaps);
    Areas {
        toolbar,
        search,
        list,
        list_height,
        footer: list + list_height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chrome_rows_collapse_with_the_height() {
        let tall = areas(22);
        assert_eq!((tall.toolbar, tall.search, tall.list, tall.list_height), (2, 4, 6, 12));
        assert_eq!(tall.footer, 18);
        let short = areas(13);
        assert_eq!((short.toolbar, short.search, short.list, short.list_height), (1, 2, 3, 6));
        assert_eq!(short.footer, 9);
    }
}
