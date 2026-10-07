use win32ui::d2d::RectF;
use winmon::ui::layout::{columns, sections};

fn overlaps(a: RectF, b: RectF) -> bool {
    a.left < b.right - 0.01
        && b.left < a.right - 0.01
        && a.top < b.bottom - 0.01
        && b.top < a.bottom - 0.01
}

fn inside(inner: RectF, outer: RectF) -> bool {
    inner.left >= outer.left - 0.01
        && inner.top >= outer.top - 0.01
        && inner.right <= outer.right + 0.01
        && inner.bottom <= outer.bottom + 0.01
}

#[test]
fn no_overlap_and_within_bounds() {
    for (w, h) in [
        (480.0, 800.0),
        (800.0, 480.0),
        (320.0, 480.0),
        (1024.0, 600.0),
        (600.0, 1024.0),
        (1920.0, 480.0),
    ] {
        let bounds = RectF::new(0.0, 0.0, w, h);
        let s = sections(bounds);
        let all = s.all();
        for (i, a) in all.iter().enumerate() {
            assert!(inside(*a, bounds), "{w}x{h}: section {i} {a:?} outside");
            assert!(
                a.width() > 0.0 && a.height() > 0.0,
                "{w}x{h}: section {i} empty"
            );
            for (j, b) in all.iter().enumerate().skip(i + 1) {
                assert!(!overlaps(*a, *b), "{w}x{h}: sections {i} and {j} overlap");
            }
        }
    }
}

#[test]
fn portrait_is_stacked() {
    let s = sections(RectF::new(0.0, 0.0, 480.0, 800.0));
    assert!(s.clock.bottom <= s.cpu.top);
    assert!(s.cpu.bottom <= s.mem.top);
    assert!(s.mem.bottom <= s.temps.top);
    assert!(s.temps.bottom <= s.weather.top);
    assert!((s.cpu.width() - s.weather.width()).abs() < 0.01);
}

#[test]
fn landscape_has_two_columns() {
    let s = sections(RectF::new(0.0, 0.0, 800.0, 480.0));
    assert!(s.clock.right <= s.cpu.left);
    assert!(s.weather.right <= s.mem.left);
}

#[test]
fn columns_split_evenly() {
    let c = columns(RectF::new(0.0, 0.0, 100.0, 10.0), 4, 4.0);
    assert_eq!(c.len(), 4);
    assert!((c[0].width() - 22.0).abs() < 0.01);
    assert!((c[3].right - 100.0).abs() < 0.01);
    assert!(columns(RectF::new(0.0, 0.0, 100.0, 10.0), 0, 4.0).is_empty());
}
