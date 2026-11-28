use super::*;

fn push(queue: &mut PriorityQueue, priority: f64) -> EntryId {
    let mut work = usize::MAX;
    queue.push(0, false, priority, &mut work).unwrap()
}

fn order(queue: &mut PriorityQueue) -> Vec<EntryId> {
    let mut work = usize::MAX;
    let mut result = Vec::new();
    while !queue.is_empty() {
        result.push(queue.pop(&mut work).unwrap().id);
    }
    result
}

#[test]
fn priority_queue() {
    let mut queue = PriorityQueue::default();
    let large = push(&mut queue, 3.0);
    let medium = push(&mut queue, 2.0);
    let small = push(&mut queue, 1.0);
    assert_eq!(order(&mut queue), [small, medium, large]);
    assert!(queue.is_empty());
}

#[test]
fn priority_queue_decrease() {
    let mut queue = PriorityQueue::default();
    let mut work = usize::MAX;
    let expected = push(&mut queue, 3.0);
    push(&mut queue, 2.0);
    queue.decrease(expected, 1.0, &mut work).unwrap();
    assert_eq!(queue.pop(&mut work).unwrap().id, expected);
    assert_eq!(
        queue.decrease(expected, 0.0, &mut work),
        Err(Error::InvalidGraph)
    );
}

#[test]
fn priority_queue_decrease_preserves_order() {
    let mut queue = PriorityQueue::default();
    let first = push(&mut queue, 1.0);
    let second = push(&mut queue, 2.0);
    let mut work = usize::MAX;
    queue.decrease(second, 1.0, &mut work).unwrap();
    assert_eq!(order(&mut queue), [first, second]);
}

#[test]
fn priority_queue_equal_priority_order() {
    let mut queue = PriorityQueue::default();
    let mut work = usize::MAX;
    let expected = [true, false, true, false]
        .map(|horizontal| queue.push(0, horizontal, 1.0, &mut work).unwrap());
    for (id, horizontal) in expected.into_iter().zip([true, false, true, false]) {
        let actual = queue.pop(&mut work).unwrap();
        assert_eq!(actual.id, id);
        assert_eq!(actual.horizontal, horizontal);
    }
    assert!(queue.is_empty());
}

#[test]
fn priority_queue_guard_failure_does_not_mutate() {
    let mut queue = PriorityQueue::default();
    let mut exhausted = 0;
    assert_eq!(
        queue.push(0, false, 1.0, &mut exhausted),
        Err(Error::WorkLimit)
    );
    assert!(queue.items.is_empty());
    assert!(queue.entries.is_empty());
    assert_eq!(queue.entries.capacity(), 0);

    let first = push(&mut queue, 1.0);
    let second = push(&mut queue, 2.0);
    assert!(matches!(queue.pop(&mut exhausted), Err(Error::WorkLimit)));
    assert_eq!(queue.items, [first, second]);
    assert_eq!(queue.entries[first.0].index, Some(0));
    assert_eq!(queue.entries[second.0].index, Some(1));

    let mut queue = PriorityQueue::default();
    let only = push(&mut queue, 2.0);
    assert_eq!(
        queue.decrease(only, 1.0, &mut exhausted),
        Err(Error::WorkLimit)
    );
    assert_eq!(queue.items, [only]);
    assert_eq!(queue.entries[only.0].index, Some(0));
    assert_eq!(queue.entries[only.0].priority, 2.0);
}

fn swap_heap_path(heap: &mut [usize], mut index: usize, priorities: &[f64], up: bool) {
    let less = |left: usize, right: usize| {
        if priorities[left] == priorities[right] {
            left < right
        } else {
            priorities[left] < priorities[right]
        }
    };
    loop {
        let next = if up {
            if index == 0 {
                return;
            }
            (index - 1) / 2
        } else {
            let left = index * 2 + 1;
            let Some(&first) = heap.get(left) else {
                return;
            };
            let right = left + 1;
            if heap.get(right).is_some_and(|&second| less(second, first)) {
                right
            } else {
                left
            }
        };
        let swap = if up {
            less(heap[index], heap[next])
        } else {
            less(heap[next], heap[index])
        };
        if !swap {
            return;
        }
        heap.swap(index, next);
        index = next;
    }
}

#[test]
fn priority_queue_matches_swap_heap() {
    let mut rng = crate::go_rand::GoRand::new(29);
    let mut random = |range| rng.int31n(range);
    let mut queue = PriorityQueue::default();
    for _ in 0..300 {
        queue.reset();
        let mut work = usize::MAX;
        let mut expected_heap = Vec::new();
        let mut priorities = Vec::new();
        for _ in 0..400 {
            let action = random(10);
            let mut expected_charge = 0;
            let before = work;
            if action < 5 || expected_heap.is_empty() {
                let priority = match random(20) {
                    0 => f64::NAN,
                    1 => f64::INFINITY,
                    2 => f64::NEG_INFINITY,
                    _ => random(30) as f64,
                };
                let id = priorities.len();
                assert_eq!(
                    queue.push(id, false, priority, &mut work).unwrap(),
                    EntryId(id)
                );
                priorities.push(priority);
                expected_heap.push(id);
                expected_charge = expected_heap.len().ilog2() as usize + 1;
                let index = expected_heap.len() - 1;
                swap_heap_path(&mut expected_heap, index, &priorities, true);
            } else if action < 7 {
                let id = random(priorities.len());
                if let Some(index) = expected_heap.iter().position(|&next| next == id) {
                    let priority = priorities[id] - (1 + random(5)) as f64;
                    if priority < priorities[id] {
                        queue.decrease(EntryId(id), priority, &mut work).unwrap();
                        priorities[id] = priority;
                        expected_charge = (index + 1).ilog2() as usize + 1;
                        swap_heap_path(&mut expected_heap, index, &priorities, true);
                    }
                }
            } else {
                let expected = expected_heap[0];
                let state = queue.pop(&mut work).unwrap();
                assert_eq!(state.id, EntryId(expected));
                assert_eq!(state.node, expected);
                assert_eq!(state.priority.to_bits(), priorities[expected].to_bits());
                assert_eq!(queue.entries[expected].index, None);
                expected_charge = 2 * (expected_heap.len().ilog2() as usize + 1);
                expected_heap.swap_remove(0);
                if !expected_heap.is_empty() {
                    swap_heap_path(&mut expected_heap, 0, &priorities, false);
                }
            }
            assert_eq!(before - work, expected_charge);
            assert_eq!(queue.items.len(), expected_heap.len());
            for (position, (&actual, &expected)) in
                queue.items.iter().zip(&expected_heap).enumerate()
            {
                assert_eq!(actual, EntryId(expected));
                assert_eq!(queue.entries[expected].index, Some(position));
                assert_eq!(
                    queue.entries[expected].priority.to_bits(),
                    priorities[expected].to_bits()
                );
            }
        }
    }
}
