//! This example shows how you can generate pdfs with layers using optional content groups.
//! Not all pdf viewers support these, or have support for the full specification.
//! Some care was taken to ensure the pdfs output worked with the following viewers
//!
//! 1. The Poppler based pdf viewer evince.
//! 2. The pdf.js based pdf viewer in firefox.
//!
//! However many pdf viewers like the chrome built-in viewer do not appear to support optional
//! content groups at all.
//!
//! If your pdf viewer supports these, you should see a layer "Parent Layer",
//! with 2 sublayers "Grid", and "Hexagon". Toggling the parent layer off hides both it's children.
//! While each sublayer can be toggled on and off independently.
use pdf_writer::{
    writers::Catalog, writers::Page, Content, Finish, Name, Pdf, Rect, Ref, Str,
};
use std::f32::consts::PI;
use std::sync::atomic::{AtomicI32, Ordering};

pub fn gen_ref() -> Ref {
    static COUNTER: AtomicI32 = AtomicI32::new(1);
    Ref::new(COUNTER.fetch_add(1, Ordering::Relaxed))
}

struct Layers {
    parent: Ref,
    subgroup: Ref,
    grid: Ref,
    hexagon: Ref,
}

fn main() -> std::io::Result<()> {
    let mut pdf = Pdf::new();
    let catalog_id = gen_ref();
    let page_tree_id = gen_ref();
    let page_id = gen_ref();
    let content_stream_id = gen_ref();
    let layers = Layers {
        parent: gen_ref(),
        subgroup: gen_ref(),
        grid: gen_ref(),
        hexagon: gen_ref(),
    };
    layers.setup_subgroup(&mut pdf);
    let mut catalog = pdf.catalog(catalog_id);
    catalog.pages(page_tree_id);
    // Set the optional content group panel to visible on file open.
    catalog.pair(Name(b"PageMode"), Name(b"UseOC"));
    layers.make_oc_properties(&mut catalog);
    catalog.finish();

    pdf.pages(page_tree_id).kids([page_id]).count(1).finish();

    layers.make_ocg(&mut pdf);

    let mut page = pdf.page(page_id);
    page.parent(page_tree_id);
    page.media_box(Rect::new(0.0, 0.0, 400.0, 400.0));
    page.contents(content_stream_id);
    layers.make_page_resources(&mut page);
    page.finish();

    let mut content = Content::new();

    // Parent layer
    content
        .begin_marked_content_with_properties(Name(b"OC"))
        .operand(Name(b"ParentLayer"))
        .finish();
    {
        // Grid layer
        content
            .begin_marked_content_with_properties(Name(b"OC"))
            .operand(Name(b"GridLayer"))
            .finish();
        draw_grid(&mut content);
        content.end_marked_content();

        // Hexagon layer
        content
            .begin_marked_content_with_properties(Name(b"OC"))
            .operand(Name(b"HexagonLayer"))
            .finish();
        draw_hexagon(&mut content);
        content.end_marked_content();
    }
    content.end_marked_content();
    pdf.stream(content_stream_id, &content.finish());
    std::fs::write("target/layers.pdf", pdf.finish())?;
    Ok(())
}

fn draw_grid(content: &mut Content) {
    content.set_stroke_rgb(0.6, 0.6, 0.6);
    content.set_line_width(1.0);
    for x in (50..=350).step_by(25) {
        content.move_to(x as f32, 50.0);
        content.line_to(x as f32, 350.0);
    }
    for y in (50..=350).step_by(25) {
        content.move_to(50.0, y as f32);
        content.line_to(350.0, y as f32);
    }
    content.stroke();
}

fn draw_hexagon(content: &mut Content) {
    let center_x = 200.0;
    let center_y = 200.0;
    let radius = 60.0;

    content.set_line_width(6.0);
    content.set_stroke_rgb(0.5, 0.2, 0.8);

    let start_x = center_x + radius;
    let start_y = center_y;
    content.move_to(start_x, start_y);

    for i in 1..6 {
        let angle = 2.0 * PI * (i as f32) / 6.0;
        let x = center_x + radius * angle.cos();
        let y = center_y + radius * angle.sin();
        content.line_to(x, y);
    }

    content.close_path();
    content.stroke();
}

impl Layers {
    fn setup_subgroup(&self, pdf: &mut Pdf) {
        pdf.indirect(self.subgroup).array().items([self.grid, self.hexagon]);
    }
    fn make_oc_properties(&self, catalog: &mut Catalog) {
        let mut oc_props = catalog.insert(Name(b"OCProperties")).dict();
        oc_props.insert(Name(b"OCGs")).array().items([
            self.parent,
            self.grid,
            self.hexagon,
        ]);

        let mut d_dict = oc_props.insert(Name(b"D")).dict();
        d_dict
            .insert(Name(b"ON"))
            .array()
            .items([self.parent, self.grid, self.hexagon]);

        let mut order = d_dict.insert(Name(b"Order")).array();
        order.item(self.parent);
        order.item(self.subgroup);
        order.finish();

        d_dict
            .insert(Name(b"Locked"))
            .array()
            .items([self.grid, self.hexagon]);
        d_dict.finish();
        oc_props.finish();
    }

    fn make_ocg(&self, pdf: &mut Pdf) {
        pdf.indirect(self.parent)
            .dict()
            .pair(Name(b"Type"), Name(b"OCG"))
            .pair(Name(b"Name"), Str(b"Parent Layer"))
            .insert(Name(b"Usage"))
            .dict()
            .insert(Name(b"View"))
            .dict()
            .pair(Name(b"ViewState"), Name(b"ON"))
            .finish()
            .finish()
            .finish();

        pdf.indirect(self.grid)
            .dict()
            .pair(Name(b"Type"), Name(b"OCG"))
            .pair(Name(b"Name"), Str(b"Grid"))
            .pair(Name(b"Parent"), self.parent)
            .finish();

        pdf.indirect(self.hexagon)
            .dict()
            .pair(Name(b"Type"), Name(b"OCG"))
            .pair(Name(b"Name"), Str(b"Hexagon"))
            .pair(Name(b"Parent"), self.parent)
            .finish();
    }

    fn make_page_resources(&self, page: &mut Page) {
        // Map layer names to their refs.
        let mut res = page.resources();
        let mut ocg_dict = res.insert(Name(b"Properties")).dict();
        ocg_dict.pair(Name(b"ParentLayer"), self.parent);
        ocg_dict.pair(Name(b"GridLayer"), self.grid);
        ocg_dict.pair(Name(b"HexagonLayer"), self.hexagon);
        ocg_dict.finish();
        res.finish();
    }
}
