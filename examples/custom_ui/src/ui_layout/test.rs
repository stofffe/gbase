use core::f32;

use gbase::glam::vec4;

use crate::{
    ui_font::TextMeasurer,
    ui_layout::{
        builder::UIBuilder,
        layouter::UILayouter,
        LayoutDirection::{self, LeftToRight},
        Padding,
        Sizing::{self, Fit, Fixed, Grow, Percent},
        TextLayoutResult, TextSizeResult, UIElement,
    },
};

struct TestTextMeasurer {
    char_width: f32,
    char_height: f32,
}

impl TestTextMeasurer {
    pub fn new() -> Self {
        Self {
            char_width: 10.0,
            char_height: 10.0,
        }
    }
}

impl TextMeasurer for TestTextMeasurer {
    fn calculate_preferred_text_size(
        &mut self,
        text: &str,
        _font_size: u32,
        _wrap_on_newline: bool,
    ) -> TextSizeResult {
        let mut min_width = f32::MAX;
        let mut max_width = 0.0;

        let space_width = self.char_width;

        for word in text.split_whitespace() {
            let word_width = self.char_width * word.len() as f32;
            min_width = min_width.min(word_width);
            max_width += word_width;
            max_width += space_width;
        }

        max_width -= space_width;

        TextSizeResult {
            min_width,
            max_width,

            min_height: self.char_height,
            max_height: self.char_height,
        }
    }

    fn layout_text(&mut self, text: &str, _font_size: u32, max_width: f32) -> TextLayoutResult {
        let mut line_width = 0.0;
        let mut max_line_width = 0.0f32;
        let mut lines = 0;

        let space_width = self.char_width;

        for word in text.split_whitespace() {
            let word_width = self.char_width * word.len() as f32;

            if line_width > 0.0 {
                line_width += space_width;
            }

            // wrap
            if line_width + word_width > max_width {
                lines += 1;
                line_width -= space_width;
                max_line_width = max_line_width.max(line_width);
                line_width = 0.0;
            }

            line_width += word_width;
        }

        max_line_width = max_line_width.max(line_width);

        let height = (lines + 1) as f32 * self.char_height;
        let width = max_line_width;

        TextLayoutResult {
            width,
            height,
            glyphs: Vec::new(),
        }
    }
}

#[test]
fn multiple_top_level() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer::new();

    let elem1 = UIElement::new()
        .with_sizing_x(Fixed(300.0))
        .draw(&mut layouter);
    let elem2 = UIElement::new()
        .with_sizing_x(Fixed(200.0))
        .draw(&mut layouter);

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
    let mut measurer = TestTextMeasurer::new();

    let elem1 = UIElement::new()
        .with_sizing_x(Fixed(200.0))
        .with_sizing_y(Fixed(100.0))
        .draw(&mut layouter);

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 200.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 100.0);
}

#[test]
fn fixed_in_fixed() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer::new();

    let (elem1, elem2) = UIElement::new()
        .with_sizing_x(Fixed(500.0))
        .with_sizing_y(Fixed(600.0))
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Fixed(200.0))
                .with_sizing_y(Fixed(300.0))
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
    let mut measurer = TestTextMeasurer::new();

    let (elem1, elem2) = UIElement::new()
        .with_sizing_x(Fit)
        .with_sizing_y(Fit)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Fixed(200.0))
                .with_sizing_y(Fixed(300.0))
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
    let mut measurer = TestTextMeasurer::new();

    let (elem1, elem2) = UIElement::new()
        .with_sizing_x(Percent(0.5))
        .with_sizing_y(Percent(0.6))
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Fixed(200.0))
                .with_sizing_y(Fixed(300.0))
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
    let mut measurer = TestTextMeasurer::new();

    let (elem1, elem2) = UIElement::new()
        .with_sizing_x(Grow)
        .with_sizing_y(Grow)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Fixed(200.0))
                .with_sizing_y(Fixed(300.0))
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
    let mut measurer = TestTextMeasurer::new();

    let elem1 = UIElement::new()
        .with_sizing_x(Fixed(400.0))
        .with_sizing_y(Fixed(200.0))
        .draw(&mut layouter);

    layouter.layout_elements(&mut measurer, 300.0, 300.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 400.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 200.0);
}

#[test]
fn fixed_in_fit_in_fit() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer::new();

    let (elem1, (elem2, elem3)) = UIElement::new()
        .with_sizing_x(Fit)
        .with_sizing_y(Fit)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Fit)
                .draw_with_children(layouter, |layouter| {
                    UIElement::new()
                        .with_sizing_x(Fixed(100.0))
                        .with_sizing_y(Fixed(200.0))
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
    let mut measurer = TestTextMeasurer::new();

    let elem1 = UIElement::new()
        .with_sizing_x(Fit)
        .with_sizing_y(Fit)
        .draw(&mut layouter);

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 0.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 0.0);
}

#[test]
fn fit_in_fixed() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer::new();

    let (elem1, elem2) = UIElement::new()
        .with_layout_direction(LeftToRight)
        .with_sizing_x(Fixed(200.0))
        .with_sizing_y(Fixed(300.0))
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Fit)
                .with_sizing_y(Fit)
                .draw(layouter)
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
    let mut measurer = TestTextMeasurer::new();

    let (elem1, elem2) = UIElement::new()
        .with_layout_direction(LeftToRight)
        .with_sizing_x(Fit)
        .with_sizing_y(Fit)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Fit)
                .with_sizing_y(Fit)
                .draw(layouter)
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
    let mut measurer = TestTextMeasurer::new();

    let (elem1, elem2) = UIElement::new()
        .with_layout_direction(LeftToRight)
        .with_sizing_x(Percent(0.5))
        .with_sizing_y(Percent(0.6))
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Fit)
                .with_sizing_y(Fit)
                .draw(layouter)
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
    let mut measurer = TestTextMeasurer::new();

    let (elem1, elem2) = UIElement::new()
        .with_layout_direction(LeftToRight)
        .with_sizing_x(Grow)
        .with_sizing_y(Grow)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Fit)
                .with_sizing_y(Fit)
                .draw(layouter)
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
    let mut measurer = TestTextMeasurer::new();

    let elem1 = UIElement::new()
        .with_sizing_x(Percent(0.3))
        .with_sizing_y(Percent(0.7))
        .draw(&mut layouter);

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 300.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 700.0);
}

#[test]
fn percent_in_fixed() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer::new();

    let (elem1, elem2) = UIElement::new()
        .with_sizing_x(Fixed(400.0))
        .with_sizing_y(Fixed(600.0))
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Percent(0.5))
                .with_sizing_y(Percent(0.8))
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
    let mut measurer = TestTextMeasurer::new();

    let (elem1, elem2) = UIElement::new()
        .with_sizing_x(Fit)
        .with_sizing_y(Fit)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Percent(0.5))
                .with_sizing_y(Percent(0.9))
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
    let mut measurer = TestTextMeasurer::new();

    let (elem1, elem2) = UIElement::new()
        .with_sizing_x(Percent(0.5))
        .with_sizing_y(Percent(0.7))
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Percent(0.5))
                .with_sizing_y(Percent(1.0))
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
    let mut measurer = TestTextMeasurer::new();

    let (elem1, elem2) = UIElement::new()
        .with_sizing_x(Grow)
        .with_sizing_y(Grow)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Percent(0.5))
                .with_sizing_y(Percent(0.9))
                .draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 1000.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 1000.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 500.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 900.0);
}

#[test]
fn percent_in_grow_in_fixed() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer::new();

    let (elem1, (elem2, elem3)) = UIElement::new()
        .with_sizing_x(Fixed(1000.0))
        .with_sizing_y(Fixed(1000.0))
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Grow)
                .with_sizing_y(Grow)
                .draw_with_children(layouter, |layouter| {
                    UIElement::new()
                        .with_sizing_x(Percent(0.5))
                        .with_sizing_y(Percent(0.9))
                        .draw(layouter)
                })
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 1000.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 1000.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 1000.0);
    assert_eq!(layouter.get_element(&elem2).size[1], 1000.0);

    assert_eq!(layouter.get_element(&elem3).size[0], 500.0);
    assert_eq!(layouter.get_element(&elem3).size[1], 900.0);
}

//
// Grow
//

#[test]
fn grow_in_root() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer::new();

    let elem1 = UIElement::new()
        .with_sizing_x(Grow)
        .with_sizing_y(Grow)
        .draw(&mut layouter);

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 1000.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 1000.0);
}

#[test]
fn grow_in_fixed() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer::new();

    let (elem1, elem2) = UIElement::new()
        .with_sizing_x(Fixed(200.0))
        .with_sizing_y(Fixed(100.0))
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Grow)
                .with_sizing_y(Grow)
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
    let mut measurer = TestTextMeasurer::new();

    let (elem1, elem2) = UIElement::new()
        .with_sizing_x(Fit)
        .with_sizing_y(Fit)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Grow)
                .with_sizing_y(Grow)
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
    let mut measurer = TestTextMeasurer::new();

    let (elem1, elem2) = UIElement::new()
        .with_sizing_x(Percent(0.4))
        .with_sizing_y(Percent(0.2))
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Grow)
                .with_sizing_y(Grow)
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
    let mut measurer = TestTextMeasurer::new();

    let (elem1, elem2) = UIElement::new()
        .with_sizing_x(Grow)
        .with_sizing_y(Grow)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Grow)
                .with_sizing_y(Grow)
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
    let mut measurer = TestTextMeasurer::new();

    let (_, (elem2, elem3)) = UIElement::new()
        .with_layout_direction(LeftToRight)
        .with_sizing_x(Fixed(400.0))
        .draw_with_children(&mut layouter, |layouter| {
            let elem2 = UIElement::new().with_sizing_x(Grow).draw(layouter);
            let elem3 = UIElement::new().with_sizing_x(Grow).draw(layouter);
            (elem2, elem3)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem2).size[0], 200.0);
    assert_eq!(layouter.get_element(&elem3).size[0], 200.0);
}

#[test]
fn grow_distribution_cross_axis() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer::new();

    let (_, (elem2, elem3)) = UIElement::new()
        .with_layout_direction(LeftToRight)
        .with_sizing_y(Fixed(300.0))
        .draw_with_children(&mut layouter, |layouter| {
            let elem2 = UIElement::new().with_sizing_y(Grow).draw(layouter);
            let elem3 = UIElement::new().with_sizing_y(Grow).draw(layouter);
            (elem2, elem3)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem2).size[1], 300.0);
    assert_eq!(layouter.get_element(&elem3).size[1], 300.0);
}

#[test]
fn navbar_sidebar_statusbar() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer::new();

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
        .with_sizing_x(Sizing::Grow)
        .with_sizing_y(Sizing::Grow)
        .with_layout_direction(LayoutDirection::TopToBottom)
        .draw_with_children(&mut layouter, |layouter| {
            let (elem1_1, (elem1_1_1, elem1_1_2, elem1_1_3, elem1_1_4)) = UIElement::new()
                .with_sizing_x(Sizing::Grow)
                .with_sizing_y(Sizing::Fit)
                .with_background_color(vec4(0.2, 0.2, 0.2, 1.0))
                .draw_with_children(layouter, |layouter| {
                    let elem1_1_1 = UIElement::new()
                        .with_sizing_x(Sizing::Fixed(100.0))
                        .with_sizing_y(Sizing::Fixed(100.0))
                        .with_background_color(vec4(1.0, 1.0, 1.0, 1.0))
                        .draw(layouter);
                    let elem1_1_2 = UIElement::new().with_sizing_x(Sizing::Grow).draw(layouter);
                    let elem1_1_3 = UIElement::new()
                        .with_sizing_x(Sizing::Fixed(100.0))
                        .with_sizing_y(Sizing::Fixed(100.0))
                        .with_background_color(vec4(1.0, 1.0, 1.0, 1.0))
                        .draw(layouter);
                    let elem1_1_4 = UIElement::new()
                        .with_sizing_x(Sizing::Fixed(100.0))
                        .with_sizing_y(Sizing::Fixed(100.0))
                        .with_background_color(vec4(1.0, 1.0, 1.0, 1.0))
                        .draw(layouter);
                    (elem1_1_1, elem1_1_2, elem1_1_3, elem1_1_4)
                });

            let (elem1_2, (elem1_2_1, elem1_2_2)) = UIElement::new()
                .with_sizing_x(Sizing::Grow) // TODO: this adds somehow
                .with_sizing_y(Sizing::Grow)
                .with_background_color(vec4(0.1, 0.1, 0.1, 1.0))
                .draw_with_children(layouter, |layouter| {
                    let elem1_2_1 = UIElement::new()
                        .with_sizing_x(Sizing::Percent(0.2))
                        .with_sizing_y(Sizing::Grow)
                        .with_background_color(vec4(0.6, 0.6, 0.6, 1.0))
                        .draw(layouter);
                    let elem1_2_2 = UIElement::new()
                        .with_sizing_x(Sizing::Grow)
                        .with_sizing_y(Sizing::Grow)
                        .draw(layouter);
                    (elem1_2_1, elem1_2_2)
                });

            let (elem1_3, elem1_3_1) = UIElement::new()
                .with_sizing_x(Sizing::Grow)
                .with_sizing_y(Sizing::Fit)
                .with_background_color(vec4(0.2, 0.2, 0.2, 1.0))
                .draw_with_children(layouter, |layouter| {
                    let elem1_3_1 = UIElement::new()
                        .with_sizing_x(Sizing::Fixed(100.0))
                        .with_sizing_y(Sizing::Fixed(100.0))
                        .with_background_color(vec4(0.2, 0.2, 0.2, 1.0))
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

//
// Text
//

#[test]
fn text_with_fit() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer::new();

    let text = "hello duuude";

    let elem1 = UIBuilder::new()
        .sizing_x(Fit)
        .sizing_y(Fit)
        .text(text)
        .draw(&mut layouter);
    // let elem1 = UIElement::new()
    //     .layout_direction(LeftToRight)
    //     .sizing_x(Fit)
    //     .sizing_y(Fit)
    //     .text(text)
    //     .draw(&mut layouter);

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(
        layouter.get_element(&elem1).size[0],
        measurer.char_width * text.len() as f32
    );
    assert_eq!(layouter.get_element(&elem1).size[1], measurer.char_height);
}

#[test]
fn text_with_fixed_width() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer::new();

    // will wrap every 5 chars
    // hello
    // bro
    let text = "hello bro";

    let elem1 = UIBuilder::new()
        .sizing_x(Fixed(50.0))
        .sizing_y(Fit)
        .text(text)
        .element()
        .draw(&mut layouter);

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 50.0);
    assert_eq!(
        layouter.get_element(&elem1).size[1],
        measurer.char_height * 2.0
    );
}

#[test]
fn text_distribution() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer::new();

    // will wrap every 5 chars
    // hello
    // bro i
    // saw
    // a rad
    // electric
    // bike
    let text1 = "hello bro i saw a rad electric bike";

    // will wrap every 5 chars
    // hello
    // bro
    let text2 = "hello bro";

    let (elem1, (elem2, elem3)) = UIBuilder::new()
        .sizing_x(Fixed(100.0))
        .sizing_y(Fit)
        .container()
        .layout_direction(LeftToRight)
        .draw_with_children(&mut layouter, |layouter| {
            let elem2 = UIBuilder::new()
                .sizing_x(Fit)
                .sizing_y(Fit)
                .text(text1)
                .element()
                .draw(layouter);
            let elem3 = UIBuilder::new()
                .sizing_x(Fit)
                .sizing_y(Fit)
                .text(text2)
                .element()
                .draw(layouter);
            (elem2, elem3)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    let wrapped_text1_height = measurer.char_height * 6.0;
    let wrapped_text2_height = measurer.char_height * 2.0;

    assert_eq!(layouter.get_element(&elem1).size[0], 100.0);
    assert_eq!(layouter.get_element(&elem1).size[1], wrapped_text1_height);

    assert_eq!(layouter.get_element(&elem2).size[0], 50.0);
    assert_eq!(layouter.get_element(&elem2).size[1], wrapped_text1_height);

    assert_eq!(layouter.get_element(&elem3).size[0], 50.0);
    assert_eq!(layouter.get_element(&elem3).size[1], wrapped_text2_height);
}

#[test]
fn text_in_fixed_width() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer::new();

    // will wrap every 5 chars
    // hello
    // bro i
    // saw
    // a rad
    // electric
    // bike
    let text = "hello bro i saw a rad electric bike";

    let (elem1, elem2) = UIElement::new()
        .with_sizing_x(Fixed(50.0))
        .with_sizing_y(Fit)
        .with_layout_direction(LeftToRight)
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Fit)
                .with_sizing_y(Fit)
                .with_text(text)
                .draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    let wrapped_text_height = measurer.char_height * 6.0;

    assert_eq!(layouter.get_element(&elem1).size[0], 50.0);
    assert_eq!(layouter.get_element(&elem1).size[1], wrapped_text_height);

    assert_eq!(layouter.get_element(&elem2).size[0], 50.0);
    assert_eq!(layouter.get_element(&elem2).size[1], wrapped_text_height);
}

#[test]
fn text_in_fixed_with_many_children() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer::new();

    // will wrap every 5 chars
    // hello
    // bro i
    // saw
    // a rad
    // electric
    // bike
    let text = "hello bro i saw a rad electric bike";
    let wrapped_text_height = measurer.char_height * 6.0;

    let (elem1, (elem2, elem3, elem4)) = UIBuilder::new()
        .sizing_x(Fixed(300.0))
        .sizing_y(Fit)
        .container()
        .layout_direction(LeftToRight)
        .draw_with_children(&mut layouter, |layouter| {
            let elem2 = UIBuilder::new()
                .sizing_x(Fixed(100.0))
                .sizing_y(Grow)
                .draw(layouter);
            let elem3 = UIBuilder::new().text(text).element().draw(layouter);
            let elem4 = UIBuilder::new()
                .sizing_x(Fixed(150.0))
                .sizing_y(Fit)
                .draw(layouter);
            (elem2, elem3, elem4)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 300.0);
    assert_eq!(layouter.get_element(&elem1).size[1], wrapped_text_height);

    assert_eq!(layouter.get_element(&elem2).size[0], 100.0);
    assert_eq!(layouter.get_element(&elem2).size[1], wrapped_text_height);

    assert_eq!(layouter.get_element(&elem3).size[0], 50.0);
    assert_eq!(layouter.get_element(&elem3).size[1], wrapped_text_height);

    assert_eq!(layouter.get_element(&elem4).size[0], 150.0);
    assert_eq!(layouter.get_element(&elem4).size[1], 0.0);
}

//
// Padding
//

#[test]
fn grow_in_fixed_with_padding() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer::new();

    let padding = Padding::all(10.0);

    let (elem1, elem2) = UIElement::new()
        .with_sizing_x(Fixed(200.0))
        .with_sizing_y(Fixed(100.0))
        .with_padding(padding.clone())
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Grow)
                .with_sizing_y(Grow)
                .draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 200.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 100.0);

    assert_eq!(
        layouter.get_element(&elem2).size[0],
        200.0 - padding.horizontal_sum()
    );
    assert_eq!(
        layouter.get_element(&elem2).size[1],
        100.0 - padding.vertical_sum()
    );
}

#[test]
fn percent_in_fixed_with_padding() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer::new();

    let padding = Padding::all(10.0);

    let (elem1, elem2) = UIElement::new()
        .with_sizing_x(Fixed(200.0))
        .with_sizing_y(Fixed(100.0))
        .with_padding(padding.clone())
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new()
                .with_sizing_x(Percent(0.5))
                .with_sizing_y(Percent(0.9))
                .draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 200.0);
    assert_eq!(layouter.get_element(&elem1).size[1], 100.0);

    assert_eq!(
        layouter.get_element(&elem2).size[0],
        (200.0 - padding.horizontal_sum()) * 0.5
    );
    assert_eq!(
        layouter.get_element(&elem2).size[1],
        (100.0 - padding.vertical_sum()) * 0.9
    );
}

#[test]
fn text_in_fixed_with_padding() {
    let mut layouter = UILayouter::new();
    let mut measurer = TestTextMeasurer::new();

    // should wrap after 8 chars
    // a b c d
    // e f g h
    // i
    let text = "a b c d e f g h i";
    let wrapped_text_height = measurer.char_height * 3.0;
    let padding = Padding::all(10.0);

    let (elem1, elem2) = UIElement::new()
        .with_sizing_x(Fixed(100.0))
        .with_sizing_y(Fit)
        .with_padding(padding.clone())
        .draw_with_children(&mut layouter, |layouter| {
            UIElement::new().with_text(text).draw(layouter)
        });

    layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

    assert_eq!(layouter.get_element(&elem1).size[0], 100.0);
    assert_eq!(
        layouter.get_element(&elem1).size[1],
        wrapped_text_height + padding.vertical_sum()
    );

    assert_eq!(
        layouter.get_element(&elem2).size[0],
        100.0 - padding.horizontal_sum()
    );
    assert_eq!(layouter.get_element(&elem2).size[1], wrapped_text_height);
}
