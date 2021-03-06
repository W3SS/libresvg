// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use cairo;

use pango::{
    self,
    LayoutExt,
    ContextExt,
};

use pangocairo::functions as pc;

use dom;

use math::{
    Rect,
    Point,
};

use super::{
    fill,
    stroke,
};


const PANGO_SCALE_64: f64 = pango::SCALE as f64;


pub fn draw(
    doc: &dom::Document,
    elem: &dom::Text,
    cr: &cairo::Context,
) {
    if elem.children.is_empty() {
        return;
    }

    for chunk in &elem.children {
        let mut chunk_width = 0.0;

        for tspan in &chunk.children {
            let pango_context = pc::create_context(cr).unwrap();
            pc::context_set_resolution(&pango_context, doc.dpi);

            let font = init_font(&tspan.font, doc.dpi);

            let layout = pango::Layout::new(&pango_context);
            layout.set_font_description(Some(&font));
            layout.set_text(&tspan.text);

            let layout_width = layout.get_size().0 as f64 / PANGO_SCALE_64;

            chunk_width += layout_width as f64;
        }

        let mut pos = Point::new(chunk.x, chunk.y);

        pos.x = process_text_anchor(pos.x, chunk.anchor, chunk_width);

        for tspan in &chunk.children {
            pos.x += draw_tspan(doc, tspan, pos, cr);
        }
    }
}

fn draw_tspan(
    doc: &dom::Document,
    tspan: &dom::TSpan,
    start_pos: Point,
    cr: &cairo::Context,
) -> f64
{
    let pango_context = pc::create_context(cr).unwrap();

    pc::update_context(cr, &pango_context);
    pc::context_set_resolution(&pango_context, doc.dpi);

    let font = init_font(&tspan.font, doc.dpi);

    let layout = pango::Layout::new(&pango_context);
    layout.set_font_description(Some(&font));
    layout.set_text(&tspan.text);

    let font_metrics = pango_context.get_metrics(Some(&font), None).unwrap();

    let mut pos = start_pos;

    let mut layout_iter = layout.get_iter().unwrap();
    let baseline_offset = (layout_iter.get_baseline() / pango::SCALE) as f64;
    pos.y -= baseline_offset;

    // Contains only characters path bounding box,
    // so spaces around text are ignored.
    let bbox = calc_layout_bbox(&layout, pos.x, pos.y);

    // Contains layout width including leading and trailing spaces.
    let layout_width = layout.get_size().0 as f64 / PANGO_SCALE_64;

    let mut line_rect = Rect {
        x: pos.x,
        y: 0.0,
        w: layout_width,
        h: font_metrics.get_underline_thickness() as f64 / PANGO_SCALE_64,
    };

    // Draw underline.
    //
    // Should be drawn before/under text.
    if let Some(ref style) = tspan.decoration.underline {
        line_rect.y = pos.y + baseline_offset
                      - font_metrics.get_underline_position() as f64 / PANGO_SCALE_64;
        draw_line(doc, &style.fill, &style.stroke, line_rect, cr);
    }

    // Draw overline.
    //
    // Should be drawn before/under text.
    if let Some(ref style) = tspan.decoration.overline {
        line_rect.y = pos.y + font_metrics.get_underline_thickness() as f64 / PANGO_SCALE_64;
        draw_line(doc, &style.fill, &style.stroke, line_rect, cr);
    }

    // Draw text.
    cr.move_to(pos.x, pos.y);

    fill::apply(doc, &tspan.fill, cr, &bbox);
    pc::update_layout(cr, &layout);
    pc::show_layout(cr, &layout);

    stroke::apply(doc, &tspan.stroke, cr, &bbox);
    pc::layout_path(cr, &layout);
    cr.stroke();

    cr.move_to(-pos.x, -pos.y);

    // Draw line-through.
    //
    // Should be drawn after/over text.
    if let Some(ref style) = tspan.decoration.line_through {
        line_rect.y = pos.y + baseline_offset
                      - font_metrics.get_strikethrough_position() as f64 / PANGO_SCALE_64;
        line_rect.h = font_metrics.get_strikethrough_thickness() as f64 / PANGO_SCALE_64;
        draw_line(doc, &style.fill, &style.stroke, line_rect, cr);
    }

    layout_width
}

fn init_font(dom_font: &dom::Font, dpi: f64) -> pango::FontDescription {
    let mut font = pango::FontDescription::new();

    font.set_family(&dom_font.family);

    let font_style = match dom_font.style {
        dom::FontStyle::Normal => pango::Style::Normal,
        dom::FontStyle::Italic => pango::Style::Italic,
        dom::FontStyle::Oblique => pango::Style::Oblique,
    };
    font.set_style(font_style);

    let font_variant = match dom_font.variant {
        dom::FontVariant::Normal => pango::Variant::Normal,
        dom::FontVariant::SmallCaps => pango::Variant::SmallCaps,
    };
    font.set_variant(font_variant);

    let font_weight = match dom_font.weight {
        dom::FontWeight::W100       => pango::Weight::Thin,
        dom::FontWeight::W200       => pango::Weight::Ultralight,
        dom::FontWeight::W300       => pango::Weight::Light,
        dom::FontWeight::W400       => pango::Weight::Normal,
        dom::FontWeight::W500       => pango::Weight::Medium,
        dom::FontWeight::W600       => pango::Weight::Semibold,
        dom::FontWeight::W700       => pango::Weight::Bold,
        dom::FontWeight::W800       => pango::Weight::Ultrabold,
        dom::FontWeight::W900       => pango::Weight::Heavy,
        dom::FontWeight::Normal     => pango::Weight::Normal,
        dom::FontWeight::Bold       => pango::Weight::Bold,
        dom::FontWeight::Bolder     => pango::Weight::Ultrabold,
        dom::FontWeight::Lighter    => pango::Weight::Light,
    };
    font.set_weight(font_weight);

    let font_stretch = match dom_font.stretch {
        dom::FontStretch::Normal         => pango::Stretch::Normal,
        dom::FontStretch::Narrower |
        dom::FontStretch::Condensed      => pango::Stretch::Condensed,
        dom::FontStretch::UltraCondensed => pango::Stretch::UltraCondensed,
        dom::FontStretch::ExtraCondensed => pango::Stretch::ExtraCondensed,
        dom::FontStretch::SemiCondensed  => pango::Stretch::SemiCondensed,
        dom::FontStretch::SemiExpanded   => pango::Stretch::SemiExpanded,
        dom::FontStretch::Wider |
        dom::FontStretch::Expanded       => pango::Stretch::Expanded,
        dom::FontStretch::ExtraExpanded  => pango::Stretch::ExtraExpanded,
        dom::FontStretch::UltraExpanded  => pango::Stretch::UltraExpanded,
    };
    font.set_stretch(font_stretch);


    let font_size = dom_font.size * PANGO_SCALE_64 / dpi * 72.0;
    font.set_size(font_size as i32);

    font
}

fn calc_layout_bbox(layout: &pango::Layout, x: f64, y: f64) -> Rect {
    let (ink_rect, _) = layout.get_extents();

    Rect {
        x: x + ink_rect.x  as f64 / PANGO_SCALE_64,
        y: y + ink_rect.y  as f64 / PANGO_SCALE_64,
        w: ink_rect.width  as f64 / PANGO_SCALE_64,
        h: ink_rect.height as f64 / PANGO_SCALE_64,
    }
}

fn draw_line(
    doc: &dom::Document,
    fill: &Option<dom::Fill>,
    stroke: &Option<dom::Stroke>,
    line_bbox: Rect,
    cr: &cairo::Context,
) {
    cr.new_sub_path();
    cr.move_to(line_bbox.x, line_bbox.y);
    cr.rel_line_to(line_bbox.w, 0.0);
    cr.rel_line_to(0.0, line_bbox.h);
    cr.rel_line_to(-line_bbox.w, 0.0);
    cr.close_path();

    fill::apply(doc, fill, cr, &line_bbox);
    cr.fill_preserve();
    stroke::apply(doc, stroke, cr, &line_bbox);
    cr.stroke();
}

fn process_text_anchor(x: f64, a: dom::TextAnchor, text_width: f64) -> f64 {
    match a {
        dom::TextAnchor::Start =>  x, // Nothing.
        dom::TextAnchor::Middle => x - text_width / 2.0,
        dom::TextAnchor::End =>    x - text_width,
    }
}
