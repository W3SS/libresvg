// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path;

use base64;
use svgdom;
use dom;

use short::{
    AId,
};

use traits::{
    GetValue,
};

use math::{
    Rect,
};

use {
    Options,
};


pub fn convert(node: &svgdom::Node, opt: &Options) -> Option<dom::Element> {
    let attrs = node.attributes();

    let ts = attrs.get_transform(AId::Transform).unwrap_or_default();

    let x = attrs.get_number(AId::X).unwrap_or(0.0);
    let y = attrs.get_number(AId::Y).unwrap_or(0.0);

    macro_rules! get_attr {
        ($aid:expr) => (
            if let Some(v) = attrs.get_type($aid) {
                v
            } else {
                warn!("The 'image' element lacks '{}' attribute. Skipped.", $aid);
                return None;
            }
        )
    }

    let w: f64 = *get_attr!(AId::Width);
    let h: f64 = *get_attr!(AId::Height);

    let href: &String = get_attr!(AId::XlinkHref);

    if let Some(data) = get_href_data(href, opt.path.as_ref()) {
        let elem = dom::Element {
            id: node.id().clone(),
            data: dom::Type::Image(dom::Image {
                rect: Rect::new(x, y, w, h),
                data: data,
            }),
            transform: ts,
        };

        Some(elem)
    } else {
        None
    }
}

fn get_href_data(href: &str, path: Option<&path::PathBuf>) -> Option<dom::ImageData> {
    if href.starts_with("data:image") {
        if let Some(idx) = href.find(',') {
            let kind = if href[..idx].contains("image/jpg") {
                dom::ImageDataKind::JPEG
            } else if href[..idx].contains("image/png") {
                dom::ImageDataKind::PNG
            } else {
                return None;
            };

            let base_data = &href[(idx + 1)..];

            let conf = base64::Config::new(
                base64::CharacterSet::Standard,
                true,
                true,
                base64::LineWrap::NoWrap,
            );

            if let Ok(data) = base64::decode_config(base_data, conf) {
                return Some(dom::ImageData::Raw(data.to_owned(), kind));
            }
        }

        warn!("Invalid xlink:href content.");
    } else {
        let path = match path {
            Some(path) => path.parent().unwrap().join(href),
            None => path::PathBuf::from(href),
        };

        if path.exists() {
            return Some(dom::ImageData::Path(path.to_owned()));
        }

        warn!("Linked file does not exist: '{}'.", href);
    }

    None
}
