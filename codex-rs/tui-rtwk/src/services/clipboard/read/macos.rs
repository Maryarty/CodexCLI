//! Read native image representations in macOS pasteboard item order (oracle
//! `clipboard_paste/macos.rs`).

use std::io::Cursor;

use image::DynamicImage;
use image::ImageFormat;
use image::ImageReader;
use image::Limits;
use objc2_app_kit::NSPasteboard;
use objc2_app_kit::NSPasteboardItem;
use objc2_foundation::NSArray;
use objc2_foundation::NSString;
use objc2_foundation::ns_string;

use super::decode::decode_oriented;

/// Decode an offered PNG or JPEG before falling back to [arboard's macOS image reader].
/// That reader requests TIFF even when the pasteboard already holds the original PNG or
/// JPEG. Clipboard owners such as Vicinae then generate a large TIFF just for this paste.
///
/// [arboard's macOS image reader]: https://github.com/1Password/arboard/blob/a3750c79a5d63f3987317e03c412b7d9dffdc2af/src/platform/osx.rs#L217-L231
pub(super) fn clipboard_image() -> Option<DynamicImage> {
    objc2::rc::autoreleasepool(|_| {
        let items = NSPasteboard::generalPasteboard().pasteboardItems()?;
        first_image(&items)
    })
}

fn first_image(items: &NSArray<NSPasteboardItem>) -> Option<DynamicImage> {
    let png = ns_string!("public.png");
    for item in items {
        let types = item.types();
        // Prefer lossless PNG, even when it isn't the first representation advertised.
        if types.containsObject(png)
            && let Some(image) = decode_representation(&item, png, ImageFormat::Png)
        {
            return Some(image);
        }

        // For JPEG and TIFF, respect the owner's order. arboard reads the first TIFF.
        for ty in types {
            if *ty == *ns_string!("public.tiff") {
                return None;
            }
            if *ty == *ns_string!("public.jpeg")
                && let Some(image) = decode_representation(&item, &ty, ImageFormat::Jpeg)
            {
                return Some(image);
            }
        }
    }
    None
}

fn decode_representation(
    item: &NSPasteboardItem,
    ty: &NSString,
    format: ImageFormat,
) -> Option<DynamicImage> {
    let bytes = item.dataForType(ty)?.to_vec();
    let reader = ImageReader::with_format(Cursor::new(bytes), format);
    decode_oriented(reader, Limits::default()).ok()
}

#[cfg(test)]
mod tests {
    use objc2::rc::Retained;
    use objc2_foundation::NSData;
    use pretty_assertions::assert_eq;

    use super::*;

    fn item(ty: &NSString, bytes: &[u8]) -> Retained<NSPasteboardItem> {
        let item = NSPasteboardItem::new();
        assert!(item.setData_forType(&NSData::with_bytes(bytes), ty));
        item
    }

    fn encoded(format: ImageFormat, width: u32) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::new_rgb8(width, /*height*/ 1)
            .write_to(&mut bytes, format)
            .unwrap();
        bytes.into_inner()
    }

    #[test]
    fn tiff_item_precedes_later_png() {
        let tiff = item(ns_string!("public.tiff"), b"owned by arboard");
        let png = item(
            ns_string!("public.png"),
            &encoded(ImageFormat::Png, /*width*/ 2),
        );
        let items = NSArray::from_retained_slice(&[tiff, png]);
        assert_eq!(first_image(&items), None);
    }

    #[test]
    fn jpeg_item_precedes_later_png() {
        let text = item(ns_string!("public.utf8-plain-text"), b"text");
        let jpeg = item(
            ns_string!("public.jpeg"),
            &encoded(ImageFormat::Jpeg, /*width*/ 3),
        );
        let png = item(
            ns_string!("public.png"),
            &encoded(ImageFormat::Png, /*width*/ 2),
        );
        let items = NSArray::from_retained_slice(&[text, jpeg, png]);
        assert_eq!(
            first_image(&items),
            Some(DynamicImage::new_rgb8(/*width*/ 3, /*height*/ 1))
        );
    }
}
