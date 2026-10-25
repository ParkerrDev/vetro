//! Rectangles in device pixels, with the semantics of the compositor's own
//! `CBox` where the plugin relied on them.

/// A rectangle: top left corner and size.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    /// A rectangle from its corner and size.
    #[must_use]
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    /// The part both rectangles cover. Rectangles that do not meet give a
    /// rectangle of no size, at the corner where they would have.
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Self {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.w).min(other.x + other.w);
        let bottom = (self.y + self.h).min(other.y + other.h);
        let (w, h) = (right - x, bottom - y);
        if w < 0.0 || h < 0.0 {
            Self::new(x, y, 0.0, 0.0)
        } else {
            Self::new(x, y, w, h)
        }
    }

    /// True when the rectangles share area; touching along an edge is not sharing.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }

    /// The smallest rectangle that holds both.
    #[must_use]
    pub fn unite(&self, other: &Self) -> Self {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = (self.x + self.w).max(other.x + other.w);
        let bottom = (self.y + self.h).max(other.y + other.h);
        Self::new(x, y, right - x, bottom - y)
    }

    /// True when the rectangle has area.
    #[must_use]
    pub fn has_area(&self) -> bool {
        self.w > 0.0 && self.h > 0.0
    }
}

/// The frame of a window: the band of glass that shows round a window that
/// is itself solid, as four strips that do not overlap.
///
/// `outer` is the pane, which reaches `band` beyond the window on every side.
/// A solid window hides the middle of its pane, so only the band is ever
/// seen, and at each corner the wedge that the window's own rounding leaves
/// open. Drawing these strips and nothing else costs the frame's area and not
/// the window's. The top and bottom strips run the whole width and are as
/// deep as `corner`, the pane's own corner radius, when that is more than
/// the band, so that the corners are whole; the left and right strips fill
/// the band between them.
///
/// In order: top, bottom, left, right. A strip has no area where the band is
/// none, or where the strips before it have already covered the pane.
#[must_use]
pub fn frame_strips(outer: &Rect, band: f64, corner: f64) -> [Rect; 4] {
    let none = Rect::new(outer.x, outer.y, 0.0, 0.0);
    if band.is_nan() || band <= 0.0 || !outer.has_area() {
        return [none; 4];
    }
    let depth = band.max(corner).min(outer.h / 2.0);
    let side = band.min(outer.w / 2.0);
    let between = (outer.h - 2.0 * depth).max(0.0);
    [
        Rect::new(outer.x, outer.y, outer.w, depth),
        Rect::new(outer.x, outer.y + outer.h - depth, outer.w, depth),
        Rect::new(outer.x, outer.y + depth, side, between),
        Rect::new(outer.x + outer.w - side, outer.y + depth, side, between),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangles_meet_as_the_compositors_boxes_do() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(
            !a.overlaps(&Rect::new(10.0, 0.0, 10.0, 10.0)),
            "touching is not overlapping"
        );
        assert!(a.overlaps(&Rect::new(5.0, 5.0, 10.0, 10.0)));
        assert!(
            a.overlaps(&Rect::new(3.0, 3.0, 0.0, 0.0)),
            "a point inside counts"
        );
        assert_eq!(
            a.intersection(&Rect::new(5.0, 5.0, 10.0, 10.0)),
            Rect::new(5.0, 5.0, 5.0, 5.0)
        );
        assert_eq!(
            a.intersection(&Rect::new(20.0, 20.0, 5.0, 5.0)),
            Rect::new(20.0, 20.0, 0.0, 0.0)
        );
        assert!(!a.intersection(&Rect::new(10.0, 0.0, 10.0, 10.0)).has_area());
    }

    #[test]
    fn a_frame_is_its_band_and_its_corners_and_never_the_middle() {
        let outer = Rect::new(100.0, 50.0, 400.0, 300.0);
        let strips = frame_strips(&outer, 4.0, 24.0);
        let [top, bottom, left, right] = strips;
        assert_eq!(top, Rect::new(100.0, 50.0, 400.0, 24.0));
        assert_eq!(bottom, Rect::new(100.0, 326.0, 400.0, 24.0));
        assert_eq!(left, Rect::new(100.0, 74.0, 4.0, 252.0));
        assert_eq!(right, Rect::new(496.0, 74.0, 4.0, 252.0));
        for (at, strip) in strips.iter().enumerate() {
            for other in strips.iter().skip(at + 1) {
                assert!(!strip.overlaps(other), "no pixel is drawn twice");
            }
        }
        // The window's own box, less the corners, is left alone.
        let middle = Rect::new(104.0, 74.0, 392.0, 252.0);
        assert!(strips.iter().all(|strip| !strip.overlaps(&middle)));
        let covered: f64 = strips.iter().map(|strip| strip.w * strip.h).sum();
        assert!(
            covered < 0.2 * outer.w * outer.h,
            "a frame is far less than a pane"
        );
    }

    #[test]
    fn a_frame_with_square_corners_is_the_band_alone() {
        let [top, bottom, left, right] = frame_strips(&Rect::new(0.0, 0.0, 100.0, 60.0), 3.0, 0.0);
        assert_eq!(top, Rect::new(0.0, 0.0, 100.0, 3.0));
        assert_eq!(bottom, Rect::new(0.0, 57.0, 100.0, 3.0));
        assert_eq!(left, Rect::new(0.0, 3.0, 3.0, 54.0));
        assert_eq!(right, Rect::new(97.0, 3.0, 3.0, 54.0));
    }

    #[test]
    fn no_band_and_no_pane_make_no_frame() {
        let outer = Rect::new(0.0, 0.0, 100.0, 60.0);
        assert!(
            frame_strips(&outer, 0.0, 20.0)
                .iter()
                .all(|strip| !strip.has_area())
        );
        assert!(
            frame_strips(&outer, -2.0, 20.0)
                .iter()
                .all(|strip| !strip.has_area())
        );
        assert!(
            frame_strips(&outer, f64::NAN, 20.0)
                .iter()
                .all(|strip| !strip.has_area())
        );
        let nothing = Rect::new(5.0, 5.0, 0.0, 0.0);
        assert!(
            frame_strips(&nothing, 3.0, 20.0)
                .iter()
                .all(|strip| !strip.has_area())
        );
    }

    #[test]
    fn a_pane_smaller_than_its_corners_is_covered_once() {
        // Corners deeper than half the pane: the top and bottom strips meet
        // in the middle and nothing is left for the sides.
        let outer = Rect::new(0.0, 0.0, 40.0, 30.0);
        let [top, bottom, left, right] = frame_strips(&outer, 4.0, 24.0);
        assert_eq!(top, Rect::new(0.0, 0.0, 40.0, 15.0));
        assert_eq!(bottom, Rect::new(0.0, 15.0, 40.0, 15.0));
        assert!(!left.has_area() && !right.has_area());
        assert!(!top.overlaps(&bottom));
    }

    #[test]
    fn the_union_holds_both() {
        let a = Rect::new(0.0, 0.0, 4.0, 4.0);
        let b = Rect::new(10.0, -2.0, 2.0, 3.0);
        assert_eq!(a.unite(&b), Rect::new(0.0, -2.0, 12.0, 6.0));
    }
}
