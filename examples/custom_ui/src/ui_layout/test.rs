use gbase::glam::vec4;

use crate::{
    ui_font::TextMeasurer,
    ui_layout::{
        layouter::UILayouter,
        LayoutDirection::{self, LeftToRight},
        Padding,
        Sizing::{self, Fit, Fixed, Grow, Percent},
        TextLayoutResult, TextSizeResult, UIElement,
    },
};

struct TestTextMeasurer {}
impl TextMeasurer for TestTextMeasurer {
    fn calculate_preferred_text_size(
        &mut self,
        _text: &str,
        _font_size: u32,
        _wrap_on_newline: bool,
    ) -> TextSizeResult {
        todo!()
    }

    fn layout_text(&mut self, _text: &str, _font_size: u32, _max_width: f32) -> TextLayoutResult {
        todo!()
    }
}

#[test]
fn multiple_top_level() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let elem1 = UIElement::new().sizing_x(Fixed(300.0)).draw(&mut layouter);
    let elem2 = UIElement::new().sizing_x(Fixed(200.0)).draw(&mut layouter);

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 300.0);
    assert_eq!(layouter.get_element(&elem2).size[0], 200.0);
}

//
// Fixed
//

#[test]
fn fixed_in_root() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let elem1 = UIElement::new()
        .sizing_x(Fixed(200.0))
        .sizing_y(Fixed(100.0))
        .draw(&mut layouter);

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 200.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 100.0);
}

#[test]
fn fixed_in_fixed() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (elem1, elem2) = UIElement::new()
        .sizing_x(Fixed(500.0))
        .sizing_y(Fixed(600.0))
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .sizing_x(Fixed(200.0))
                .sizing_y(Fixed(300.0))
                .draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 500.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 600.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 200.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 300.0);
}

#[test]
fn fixed_in_fit() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (elem1, elem2) = UIElement::new()
        .sizing_x(Fit)
        .sizing_y(Fit)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .sizing_x(Fixed(200.0))
                .sizing_y(Fixed(300.0))
                .draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 200.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 300.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 200.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 300.0);
}

#[test]
fn fixed_in_percent() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (elem1, elem2) = UIElement::new()
        .sizing_x(Percent(0.5))
        .sizing_y(Percent(0.6))
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .sizing_x(Fixed(200.0))
                .sizing_y(Fixed(300.0))
                .draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 500.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 600.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 200.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 300.0);
}

#[test]
fn fixed_in_grow() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (elem1, elem2) = UIElement::new()
        .sizing_x(Grow)
        .sizing_y(Grow)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .sizing_x(Fixed(200.0))
                .sizing_y(Fixed(300.0))
                .draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 1000.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 1000.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 200.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 300.0);
}

#[test]
fn fixed_overflow_stays_fixed() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let elem1 = UIElement::new()
        .sizing_x(Fixed(400.0))
        .sizing_y(Fixed(200.0))
        .draw(&mut layouter);

    layouter.layout_elements(&mut measurer, 300.0, 300.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 400.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 200.0);
}

#[test]
fn fixed_in_fit_in_fit() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (elem1, (elem2, elem3)) = UIElement::new()
        .sizing_x(Fit)
        .sizing_y(Fit)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .sizing_x(Fit)
                .draw_with_children(layouter, |layouter| {
                    UIElement::new()
                        .sizing_x(Fixed(100.0))
                        .sizing_y(Fixed(200.0))
                        .draw(layouter)
                })
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 100.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 200.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 100.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 200.0);

    assert_eq!(layouter.get_element(&elem3).size[0], 100.0);
    assert_eq!(layouter.get_element(&elem3).size[1], 200.0);
}

//
// Fit
//

#[test]
fn fit_in_root() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let elem1 = UIElement::new()
        .sizing_x(Fit)
        .sizing_y(Fit)
        .draw(&mut layouter);

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 0.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 0.0);
}

#[test]
fn fit_in_fixed() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (elem1, elem2) = UIElement::new()
        .layout_direction(LeftToRight)
        .sizing_x(Fixed(200.0))
        .sizing_y(Fixed(300.0))
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new().sizing_x(Fit).sizing_y(Fit).draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 200.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 300.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 0.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 0.0);
}

#[test]
fn fit_in_fit() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (elem1, elem2) = UIElement::new()
        .layout_direction(LeftToRight)
        .sizing_x(Fit)
        .sizing_y(Fit)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new().sizing_x(Fit).sizing_y(Fit).draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 0.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 0.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 0.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 0.0);
}

#[test]
fn fit_in_percent() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (elem1, elem2) = UIElement::new()
        .layout_direction(LeftToRight)
        .sizing_x(Percent(0.5))
        .sizing_y(Percent(0.6))
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new().sizing_x(Fit).sizing_y(Fit).draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 500.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 600.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 0.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 0.0);
}

#[test]
fn fit_in_grow() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (elem1, elem2) = UIElement::new()
        .layout_direction(LeftToRight)
        .sizing_x(Grow)
        .sizing_y(Grow)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new().sizing_x(Fit).sizing_y(Fit).draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 1000.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 1000.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 0.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 0.0);
}

//
// Percent
//

#[test]
fn percent_in_root() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let elem1 = UIElement::new()
        .sizing_x(Percent(0.3))
        .sizing_y(Percent(0.7))
        .draw(&mut layouter);

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 300.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 700.0);
}

#[test]
fn percent_in_fixed() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (elem1, elem2) = UIElement::new()
        .sizing_x(Fixed(400.0))
        .sizing_y(Fixed(600.0))
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .sizing_x(Percent(0.5))
                .sizing_y(Percent(0.8))
                .draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 400.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 600.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 200.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 480.0);
}

#[test]
fn percent_in_fit() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (elem1, elem2) = UIElement::new()
        .sizing_x(Fit)
        .sizing_y(Fit)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .sizing_x(Percent(0.5))
                .sizing_y(Percent(0.9))
                .draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 0.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 0.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 0.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 0.0);
}

#[test]
fn percent_in_percent() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (elem1, elem2) = UIElement::new()
        .sizing_x(Percent(0.5))
        .sizing_y(Percent(0.7))
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .sizing_x(Percent(0.5))
                .sizing_y(Percent(1.0))
                .draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 500.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 700.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 250.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 700.0);
}

#[test]
fn percent_in_grow() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (elem1, elem2) = UIElement::new()
        .sizing_x(Grow)
        .sizing_y(Grow)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .sizing_x(Percent(0.5))
                .sizing_y(Percent(0.9))
                .draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 1000.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 1000.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 500.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 900.0);
}

//
// Grow
//

#[test]
fn grow_in_root() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let elem1 = UIElement::new()
        .sizing_x(Grow)
        .sizing_y(Grow)
        .draw(&mut layouter);

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 1000.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 1000.0);
}

#[test]
fn grow_in_fixed() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (elem1, elem2) = UIElement::new()
        .sizing_x(Fixed(200.0))
        .sizing_y(Fixed(100.0))
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .sizing_x(Grow)
                .sizing_y(Grow)
                .draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 200.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 100.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 200.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 100.0);
}

#[test]
fn grow_in_fit() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (elem1, elem2) = UIElement::new()
        .sizing_x(Fit)
        .sizing_y(Fit)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .sizing_x(Grow)
                .sizing_y(Grow)
                .draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 0.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 0.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 0.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 0.0);
}

#[test]
fn grow_in_percent() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (elem1, elem2) = UIElement::new()
        .sizing_x(Percent(0.4))
        .sizing_y(Percent(0.2))
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .sizing_x(Grow)
                .sizing_y(Grow)
                .draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 400.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 200.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 400.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 200.0);
}

#[test]
fn grow_in_grow() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (elem1, elem2) = UIElement::new()
        .sizing_x(Grow)
        .sizing_y(Grow)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .sizing_x(Grow)
                .sizing_y(Grow)
                .draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 1000.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 1000.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 1000.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 1000.0);
}

#[test]
fn grow_distribution_main_axis() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (_, (elem2, elem3)) = UIElement::new()
        .layout_direction(LeftToRight)
        .sizing_x(Fixed(400.0))
        .draw_with_children(&mut layouter, |layouter| {
            let elem2 = UIElement::new().sizing_x(Grow).draw(layouter);
            let elem3 = UIElement::new().sizing_x(Grow).draw(layouter);
            (elem2, elem3)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 200.0);
    assert_eq!(layouter.get_element(&elem3).size[0], 200.0);
}

#[test]
fn grow_distribution_cross_axis() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (_, (elem2, elem3)) = UIElement::new()
        .layout_direction(LeftToRight)
        .sizing_y(Fixed(300.0))
        .draw_with_children(&mut layouter, |layouter| {
            let elem2 = UIElement::new().sizing_y(Grow).draw(layouter);
            let elem3 = UIElement::new().sizing_y(Grow).draw(layouter);
            (elem2, elem3)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem2).size[1], 300.0);
    assert_eq!(layouter.get_element(&elem3).size[1], 300.0);
}

#[test]
fn navbar_sidebar_statusbar() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer {};

    let (
        elem1,
        (
            elem1_1,
            elem1_2,
            elem1_3,
            elem1_1_1,
            elem1_1_2,
            elem1_1_3,
            elem1_1_4,
            elem1_2_1,
            elem1_2_2,
            elem1_3_1,
        ),
    ) = UIElement::new()
        .sizing_x(Sizing::Grow)
        .sizing_y(Sizing::Grow)
        .layout_direction(LayoutDirection::TopToBottom)
        .draw_with_children(&mut layouter, |layouter| {
            let (elem1_1, (elem1_1_1, elem1_1_2, elem1_1_3, elem1_1_4)) = UIElement::new()
                .sizing_x(Sizing::Grow)
                .sizing_y(Sizing::Fit)
                .background_color(vec4(0.2, 0.2, 0.2, 1.0))
                .draw_with_children(layouter, |layouter| {
                    let elem1_1_1 = UIElement::new()
                        .sizing_x(Sizing::Fixed(100.0))
                        .sizing_y(Sizing::Fixed(100.0))
                        .background_color(vec4(1.0, 1.0, 1.0, 1.0))
                        .draw(layouter);
                    let elem1_1_2 = UIElement::new().sizing_x(Sizing::Grow).draw(layouter);
                    let elem1_1_3 = UIElement::new()
                        .sizing_x(Sizing::Fixed(100.0))
                        .sizing_y(Sizing::Fixed(100.0))
                        .background_color(vec4(1.0, 1.0, 1.0, 1.0))
                        .draw(layouter);
                    let elem1_1_4 = UIElement::new()
                        .sizing_x(Sizing::Fixed(100.0))
                        .sizing_y(Sizing::Fixed(100.0))
                        .background_color(vec4(1.0, 1.0, 1.0, 1.0))
                        .draw(layouter);
                    (elem1_1_1, elem1_1_2, elem1_1_3, elem1_1_4)
                });

            let (elem1_2, (elem1_2_1, elem1_2_2)) = UIElement::new()
                .sizing_x(Sizing::Grow) // TODO: this adds somehow
                .sizing_y(Sizing::Grow)
                .background_color(vec4(0.1, 0.1, 0.1, 1.0))
                .draw_with_children(layouter, |layouter| {
                    let elem1_2_1 = UIElement::new()
                        .sizing_x(Sizing::Percent(0.2))
                        .sizing_y(Sizing::Grow)
                        .background_color(vec4(0.6, 0.6, 0.6, 1.0))
                        .draw(layouter);
                    let elem1_2_2 = UIElement::new()
                        .sizing_x(Sizing::Grow)
                        .sizing_y(Sizing::Grow)
                        .draw(layouter);
                    (elem1_2_1, elem1_2_2)
                });

            let (elem1_3, elem1_3_1) = UIElement::new()
                .sizing_x(Sizing::Grow)
                .sizing_y(Sizing::Fit)
                .background_color(vec4(0.2, 0.2, 0.2, 1.0))
                .draw_with_children(layouter, |layouter| {
                    let elem1_3_1 = UIElement::new()
                        .sizing_x(Sizing::Fixed(100.0))
                        .sizing_y(Sizing::Fixed(100.0))
                        .background_color(vec4(0.2, 0.2, 0.2, 1.0))
                        .draw(layouter);
                    //
                    elem1_3_1
                });

            (
                elem1_1, elem1_2, elem1_3, elem1_1_1, elem1_1_2, elem1_1_3, elem1_1_4, elem1_2_1,
                elem1_2_2, elem1_3_1,
            )
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 1000.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 1000.0);

    assert_eq!(layouter.get_element(&elem1_1).size[0], 1000.0);
    assert_eq!(layouter.get_element(&elem1_1).size[1], 100.0);

    assert_eq!(layouter.get_element(&elem1_2).size[0], 1000.0);
    assert_eq!(layouter.get_element(&elem1_2).size[1], 800.0);

    assert_eq!(layouter.get_element(&elem1_3).size[0], 1000.0);
    assert_eq!(layouter.get_element(&elem1_3).size[1], 100.0);

    assert_eq!(layouter.get_element(&elem1_1_1).size[0], 100.0);
    assert_eq!(layouter.get_element(&elem1_1_1).size[1], 100.0);

    assert_eq!(layouter.get_element(&elem1_1_2).size[0], 700.0);
    assert_eq!(layouter.get_element(&elem1_1_2).size[1], 0.0);

    assert_eq!(layouter.get_element(&elem1_1_3).size[0], 100.0);
    assert_eq!(layouter.get_element(&elem1_1_3).size[1], 100.0);

    assert_eq!(layouter.get_element(&elem1_1_4).size[0], 100.0);
    assert_eq!(layouter.get_element(&elem1_1_4).size[1], 100.0);

    assert_eq!(layouter.get_element(&elem1_2_1).size[0], 200.0);
    assert_eq!(layouter.get_element(&elem1_2_1).size[1], 800.0);

    assert_eq!(layouter.get_element(&elem1_2_2).size[0], 800.0);
    assert_eq!(layouter.get_element(&elem1_2_2).size[1], 800.0);

    assert_eq!(layouter.get_element(&elem1_3_1).size[0], 100.0);
    assert_eq!(layouter.get_element(&elem1_3_1).size[1], 100.0);
}
