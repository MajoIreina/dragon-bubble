//! One monochrome vector family, drawn in a centered 24 × 24 logical box.
use bevy_egui::egui::{self, Color32, Painter, Pos2, Rect, Shape, Stroke};

pub(super) fn paint(painter: &Painter, rect: Rect, index: usize, color: Color32) {
    let point =
        |x: f32, y: f32| rect.min + egui::vec2(x / 24. * rect.width(), y / 24. * rect.height());
    let polygon = |coords: &[(f32, f32)]| {
        // All supplied polygons are convex; overlapping pieces form the silhouettes.
        painter.add(Shape::convex_polygon(
            coords.iter().map(|&(x, y)| point(x, y)).collect(),
            color,
            Stroke::NONE,
        ));
    };
    let line = |a: (f32, f32), b: (f32, f32), width: f32| {
        painter.line_segment(
            [point(a.0, a.1), point(b.0, b.1)],
            Stroke::new(width * rect.width() / 24., color),
        );
        for p in [a, b] {
            painter.circle_filled(point(p.0, p.1), width * rect.width() / 48., color);
        }
    };
    match index {
        0 => {
            // Rounded dragon body, two horns, four feet and a pointed tail.
            painter.add(Shape::ellipse_filled(
                point(13., 13.),
                egui::vec2(rect.width() * 0.38, rect.height() * 0.29),
                color,
            ));
            polygon(&[(7., 9.), (8., 2.), (12., 7.)]);
            polygon(&[(15., 7.), (20., 2.), (20., 11.)]);
            polygon(&[(7., 14.), (1., 12.), (3., 19.), (9., 19.)]);
            for x in [6., 10., 16., 20.] {
                line((x, 17.), (x, 21.), 2.4);
            }
        }
        1 => {
            painter.circle_filled(point(15., 4.), rect.width() * 0.105, color);
            line((12., 9.), (15., 12.), 3.4);
            line((12., 9.), (9., 13.), 3.4);
            line((10., 11.), (7., 10.), 2.2);
            line((7., 10.), (4., 13.), 2.2);
            line((14., 11.), (18., 12.), 2.2);
            line((18., 12.), (21., 9.), 2.2);
            line((10., 14.), (13., 17.), 2.8);
            line((13., 17.), (11., 21.), 2.8);
            line((10., 14.), (7., 18.), 2.8);
            line((7., 18.), (3., 18.), 2.8);
        }
        2 => {
            painter.rect_filled(
                Rect::from_min_max(point(2., 3.), point(22., 17.)),
                2.,
                color,
            );
            polygon(&[(10., 16.), (14., 16.), (14., 20.), (10., 20.)]);
            line((7., 21.), (17., 21.), 2.);
        }
        3 => polygon(&[(3., 2.), (21., 12.), (3., 22.)]),
        4 => {
            // Two interlocking gears. Annular teeth leave genuine transparent centers.
            gear(painter, point(8., 8.), rect.width() * 0.265, color, 0.);
            gear(
                painter,
                point(16., 16.),
                rect.width() * 0.265,
                color,
                std::f32::consts::PI / 8.,
            );
        }
        6 => {
            for radius in [0.38, 0.23, 0.07] {
                painter.circle_stroke(
                    point(12., 12.),
                    rect.width() * radius,
                    Stroke::new(2_f32, color),
                );
            }
        }
        _ => {
            for x in [4., 12., 20.] {
                painter.circle_filled(point(x, 12.), rect.width() / 12., color);
            }
        }
    }
}

fn gear(painter: &Painter, center: Pos2, radius: f32, color: Color32, phase: f32) {
    let segments = 64;
    let polar =
        |angle: f32, distance: f32| center + egui::vec2(angle.cos(), angle.sin()) * distance;
    for i in 0..segments {
        let angle = |n: usize| n as f32 / segments as f32 * std::f32::consts::TAU + phase;
        let outer = |n: usize| if n % 8 < 4 { radius } else { radius * 0.78 };
        painter.add(Shape::convex_polygon(
            vec![
                polar(angle(i), radius * 0.32),
                polar(angle(i), outer(i)),
                polar(angle(i + 1), outer(i)),
                polar(angle(i + 1), radius * 0.32),
            ],
            color,
            Stroke::NONE,
        ));
    }
}
