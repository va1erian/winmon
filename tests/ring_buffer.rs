use winmon::model::RingBuffer;

#[test]
fn empty() {
    let r = RingBuffer::<f32>::new(4);
    assert!(r.is_empty());
    assert_eq!(r.latest(), None);
    assert_eq!(r.iter_oldest_first().count(), 0);
}

#[test]
fn fills_in_order() {
    let mut r = RingBuffer::new(4);
    r.push(1);
    r.push(2);
    assert_eq!(r.len(), 2);
    assert_eq!(r.latest(), Some(2));
    assert_eq!(r.iter_oldest_first().collect::<Vec<_>>(), [1, 2]);
}

#[test]
fn wraps_around() {
    let mut r = RingBuffer::new(3);
    for i in 1..=7 {
        r.push(i);
    }
    assert_eq!(r.len(), 3);
    assert_eq!(r.capacity(), 3);
    assert_eq!(r.latest(), Some(7));
    assert_eq!(r.iter_oldest_first().collect::<Vec<_>>(), [5, 6, 7]);
}

#[test]
fn capacity_one() {
    let mut r = RingBuffer::new(1);
    r.push(1);
    r.push(2);
    assert_eq!(r.len(), 1);
    assert_eq!(r.iter_oldest_first().collect::<Vec<_>>(), [2]);
    // Zero is bumped to one rather than panicking.
    let mut z = RingBuffer::new(0);
    z.push(9);
    assert_eq!(z.latest(), Some(9));
}

#[test]
fn clear() {
    let mut r = RingBuffer::new(2);
    r.push(1);
    r.clear();
    assert!(r.is_empty());
    r.push(3);
    assert_eq!(r.iter_oldest_first().collect::<Vec<_>>(), [3]);
}
