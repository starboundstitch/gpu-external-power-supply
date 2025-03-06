
pub fn translate_point(point: (i32, i32)) -> Point {
    Point::new(27 + 6 * 9 * point.0, 16 + 16 * point.1)
}
