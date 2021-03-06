// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    self,
    ElementType,
};

use dom;

use short::{
    AId,
    EId,
};

use traits::{
    GetValue,
    GetViewBox,
};

use math::{
    Rect,
    Size,
};

use {
    ErrorKind,
    Options,
    Result,
};

mod fill;
mod stroke;
mod gradient;
mod image;
mod path;
mod shapes;
mod text;


pub fn convert_doc(svg_doc: &svgdom::Document, opt: &Options) -> Result<dom::Document> {
    let svg = if let Some(svg) = svg_doc.svg_element() {
        svg
    } else {
        // Can be reached if 'preproc' module has a bug,
        // otherwise document will always have an svg node.
        //
        // Or if someone passed an invalid document directly though API.
        return Err(ErrorKind::MissingSvgNode.into());
    };

    let defs = convert_ref_nodes(&svg);

    Ok(dom::Document {
        size: get_img_size(&svg)?,
        view_box: get_view_box(&svg)?,
        dpi: opt.dpi,
        elements: convert_nodes(&svg, &defs, opt),
        defs: defs,
    })
}

pub fn convert_ref_nodes(parent: &svgdom::Node) -> Vec<dom::RefElement> {
    let mut defs: Vec<dom::RefElement> = Vec::new();

    for (id, node) in parent.descendants().svg() {
        if !node.is_referenced() {
            continue;
        }

        if !node.is_used() {
            continue;
        }

        match id {
            EId::LinearGradient => {
                if let Some(elem) = gradient::convert_linear(&node) {
                    defs.push(elem);
                }
            }
            EId::RadialGradient => {
                if let Some(elem) = gradient::convert_radial(&node) {
                    defs.push(elem);
                }
            }
            _ => {
                warn!("Unsupported element '{}'.", id);
            }
        }
    }

    defs
}

pub fn convert_nodes(
    parent: &svgdom::Node,
    defs: &[dom::RefElement],
    opt: &Options,
) -> Vec<dom::Element>
{
    let mut elements: Vec<dom::Element> = Vec::new();

    for (id, node) in parent.children().svg() {
        if node.is_referenced() {
            continue;
        }

        match id {
              EId::Title
            | EId::Desc
            | EId::Metadata
            | EId::Defs => {
                // skip, because pointless
            }
            EId::G => {
                debug_assert!(node.has_children(), "the 'g' element must contain nodes");

                let attrs = node.attributes();

                let ts = attrs.get_transform(AId::Transform).unwrap_or_default();
                let opacity = attrs.get_number(AId::Opacity);
                let children = convert_nodes(&node, defs, opt);

                // TODO: check that opacity != 1.0

                let elem = dom::Element {
                    id: node.id().clone(),
                    data: dom::Type::Group(dom::Group {
                        opacity,
                        children,
                    }),
                    transform: ts,
                };

                elements.push(elem);
            }
              EId::Line
            | EId::Rect
            | EId::Polyline
            | EId::Polygon
            | EId::Circle
            | EId::Ellipse => {
                if let Some(d) = shapes::convert(&node) {
                    if let Ok(elem) = path::convert(defs, &node, d) {
                        elements.push(elem);
                    }
                }
            }
              EId::Use
            | EId::Switch => {
                warn!("'{}' must be resolved.", id);
            }
            EId::Svg => {
                warn!("Nested 'svg' unsupported.");
            }
            EId::Path => {
                let attrs = node.attributes();
                if let Some(d) = attrs.get_path(AId::D) {
                    if let Ok(elem) = path::convert(defs, &node, d.clone()) {
                        elements.push(elem);
                    }
                }
            }
            EId::Text => {
                if let Some(elem) = text::convert(defs, &node) {
                    elements.push(elem);
                }
            }
            EId::Image => {
                if let Some(elem) = image::convert(&node, opt) {
                    elements.push(elem);
                }
            }
            _ => {
                warn!("Unsupported element '{}'.", id);
            }
        }
    }

    elements
}

fn get_img_size(svg: &svgdom::Node) -> Result<Size> {
    let attrs = svg.attributes();

    let w = attrs.get_number(AId::Width);
    let h = attrs.get_number(AId::Height);

    let (w, h) = if let (Some(w), Some(h)) = (w, h) {
        (w, h)
    } else {
        // Can be reached if 'preproc' module has a bug,
        // otherwise document will always have a valid size.
        //
        // Or if someone passed an invalid document directly though API.
        return Err(ErrorKind::InvalidSize.into());
    };

    let size = Size::new(w.round(), h.round());
    Ok(size)
}

fn get_view_box(svg: &svgdom::Node) -> Result<Rect> {
    let vbox = svg.get_viewbox()?;

    let vbox = Rect::new(
        vbox.x.round(), vbox.y.round(),
        vbox.w.round(), vbox.h.round()
    );

    Ok(vbox)
}
