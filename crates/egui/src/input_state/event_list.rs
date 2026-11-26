//! The event list for one pass, with acceptance attached to each event.

use crate::Event;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::ops::{Bound, Index, IndexMut, RangeBounds};
use core::slice::SliceIndex;
use core::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

/// Globally unique identifier for one event slot inside [`Events`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EventId(u64);

fn allocate_event_ids(count: usize) -> u64 {
    if count == 0 {
        return 0;
    }

    #[cfg(target_has_atomic = "64")]
    {
        use core::sync::atomic::AtomicU64;

        static NEXT_EVENT_ID: AtomicU64 = AtomicU64::new(1);
        NEXT_EVENT_ID.fetch_add(count as u64, AtomicOrdering::Relaxed)
    }

    #[cfg(not(target_has_atomic = "64"))]
    {
        use core::sync::atomic::AtomicUsize;

        static NEXT_EVENT_ID: AtomicUsize = AtomicUsize::new(1);
        NEXT_EVENT_ID.fetch_add(count, AtomicOrdering::Relaxed) as u64
    }
}

fn push_fresh_meta(meta: &mut Vec<EventMeta>, count: usize) {
    if count == 0 {
        return;
    }

    let base = allocate_event_ids(count);
    meta.reserve(count);
    for offset in 0..count {
        meta.push(EventMeta {
            id: EventId(base.wrapping_add(offset as u64)),
            accepted: AtomicBool::new(false),
        });
    }
}

/// Events for this pass, in the order they arrived.
///
/// This is what [`crate::InputState::events`] stores. It dereferences to
/// `[Event]` for reading and mirrors common [`Vec`] operations (`push`, `pop`,
/// `insert`, `remove`, `retain`, `clear`, `drain`, `splice`, `extend`,
/// `append`, `From<Vec<Event>>`, `FromIterator<Event>`, `IntoIterator`).
/// Each mutating method moves or drops the acceptance mark with the event it
/// belongs to, so removing, retaining, inserting, reordering, swapping,
/// sorting, or truncating cannot make a different event look accepted.
///
/// Two events that compare equal are still two events. Replacing a value, or
/// taking a mutable reference to it, drops the old event and stores a new one
/// that has not been accepted. Each event receives a globally unique internal
/// id, so replacing the whole `Events` collection cannot let stale ids from
/// earlier in the pass match unrelated events.
///
/// Acceptance is not serialized. A pass runs on one thread. The flags are
/// atomic because [`crate::InputState`] sits behind a [`epaint::mutex::RwLock`]
/// and accepting an event only needs a shared reference.
#[derive(Default)]
pub struct Events {
    events: Vec<Event>,
    meta: Vec<EventMeta>,
}

struct EventMeta {
    id: EventId,
    accepted: AtomicBool,
}

impl EventMeta {
    fn fresh() -> Self {
        Self {
            id: EventId(allocate_event_ids(1)),
            accepted: AtomicBool::new(false),
        }
    }
}

impl Clone for EventMeta {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            accepted: AtomicBool::new(self.accepted.load(AtomicOrdering::Relaxed)),
        }
    }
}

impl core::fmt::Debug for Events {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.events, f)
    }
}

impl Clone for Events {
    fn clone(&self) -> Self {
        Self {
            events: self.events.clone(),
            meta: self.meta.clone(),
        }
    }
}

impl PartialEq for Events {
    fn eq(&self, other: &Self) -> bool {
        self.events == other.events
    }
}

impl PartialEq<Vec<Event>> for Events {
    fn eq(&self, other: &Vec<Event>) -> bool {
        &self.events == other
    }
}

impl PartialEq<Events> for Vec<Event> {
    fn eq(&self, other: &Events) -> bool {
        self == &other.events
    }
}

impl PartialEq<[Event]> for Events {
    fn eq(&self, other: &[Event]) -> bool {
        self.events.as_slice() == other
    }
}

impl PartialEq<&[Event]> for Events {
    fn eq(&self, other: &&[Event]) -> bool {
        self.events.as_slice() == *other
    }
}

impl PartialEq<Events> for [Event] {
    fn eq(&self, other: &Events) -> bool {
        self == other.events.as_slice()
    }
}

impl PartialEq<Events> for &[Event] {
    fn eq(&self, other: &Events) -> bool {
        *self == other.events.as_slice()
    }
}

impl From<Vec<Event>> for Events {
    fn from(events: Vec<Event>) -> Self {
        let mut meta = Vec::new();
        push_fresh_meta(&mut meta, events.len());
        Self { events, meta }
    }
}

impl From<Events> for Vec<Event> {
    fn from(events: Events) -> Self {
        events.into_vec()
    }
}

impl FromIterator<Event> for Events {
    fn from_iter<I: IntoIterator<Item = Event>>(iter: I) -> Self {
        let events: Vec<Event> = iter.into_iter().collect();
        Self::from(events)
    }
}

impl IntoIterator for Events {
    type Item = Event;
    type IntoIter = alloc::vec::IntoIter<Event>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.into_iter()
    }
}

impl<'a> IntoIterator for &'a Events {
    type Item = &'a Event;
    type IntoIter = core::slice::Iter<'a, Event>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.iter()
    }
}

impl<'a> IntoIterator for &'a mut Events {
    type Item = &'a mut Event;
    type IntoIter = core::slice::IterMut<'a, Event>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl Extend<Event> for Events {
    fn extend<I: IntoIterator<Item = Event>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        let (lower, _) = iter.size_hint();
        self.reserve(lower);
        for event in iter {
            self.push(event);
        }
    }
}

impl core::ops::Deref for Events {
    type Target = [Event];

    fn deref(&self) -> &Self::Target {
        &self.events
    }
}

impl AsRef<[Event]> for Events {
    fn as_ref(&self) -> &[Event] {
        &self.events
    }
}

impl Borrow<[Event]> for Events {
    fn borrow(&self) -> &[Event] {
        &self.events
    }
}

impl<I: SliceIndex<[Event]>> Index<I> for Events {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.events[index]
    }
}

impl IndexMut<usize> for Events {
    fn index_mut(&mut self, index: usize) -> &mut Event {
        // A mutable reference can change the value. The value that was accepted
        // is gone, and whatever is written here is a different event.
        self.meta[index] = EventMeta::fresh();
        &mut self.events[index]
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Events {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.events.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Events {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Vec::<Event>::deserialize(deserializer)?.into())
    }
}

impl Events {
    /// Construct a new, empty event list without allocating.
    #[inline]
    pub const fn new() -> Self {
        Self {
            events: Vec::new(),
            meta: Vec::new(),
        }
    }

    /// Construct an empty event list with space for at least `capacity` events.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Vec::with_capacity(capacity),
            meta: Vec::with_capacity(capacity),
        }
    }

    /// Number of events the list can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.events.capacity().min(self.meta.capacity())
    }

    /// Reserve capacity for at least `additional` more events.
    pub fn reserve(&mut self, additional: usize) {
        self.events.reserve(additional);
        self.meta.reserve(additional);
    }

    /// View the stored events as a slice.
    #[inline]
    pub fn as_slice(&self) -> &[Event] {
        &self.events
    }

    /// Unwrap the underlying [`Vec<Event>`], discarding acceptance metadata.
    #[inline]
    pub fn into_vec(self) -> Vec<Event> {
        self.events
    }

    /// Append one new event. It has not been accepted.
    pub fn push(&mut self, event: Event) {
        self.meta.push(EventMeta::fresh());
        self.events.push(event);
    }

    /// Remove the last event, if there is one.
    pub fn pop(&mut self) -> Option<Event> {
        self.meta.pop();
        self.events.pop()
    }

    /// Insert a new event at `index`. It has not been accepted.
    pub fn insert(&mut self, index: usize, event: Event) {
        self.meta.insert(index, EventMeta::fresh());
        self.events.insert(index, event);
    }

    /// Remove the event at `index` and shift later events down.
    ///
    /// The acceptance mark of every event that stays is unchanged.
    pub fn remove(&mut self, index: usize) -> Event {
        self.meta.remove(index);
        self.events.remove(index)
    }

    /// Remove the event at `index` and move the last event into its place.
    pub fn swap_remove(&mut self, index: usize) -> Event {
        self.meta.swap_remove(index);
        self.events.swap_remove(index)
    }

    /// Replace the event at `index` with a new one.
    ///
    /// The old event, and any acceptance of it, is gone. The value now stored
    /// has not been accepted. Returns the old event.
    pub fn replace(&mut self, index: usize, event: Event) -> Option<Event> {
        if index >= self.events.len() {
            return None;
        }
        self.meta[index] = EventMeta::fresh();
        Some(core::mem::replace(&mut self.events[index], event))
    }

    /// Drop every event.
    ///
    /// Capacity is kept. New events get fresh globally unique ids, so a stale
    /// id cannot accept a later event.
    pub fn clear(&mut self) {
        self.events.clear();
        self.meta.clear();
    }

    /// Keep only the first `len` events.
    pub fn truncate(&mut self, len: usize) {
        self.events.truncate(len);
        self.meta.truncate(len);
    }

    /// Keep the events for which `keep` returns true, in order.
    pub fn retain(&mut self, mut keep: impl FnMut(&Event) -> bool) {
        let mut write = 0;
        for read in 0..self.events.len() {
            if keep(&self.events[read]) {
                if write != read {
                    self.events.swap(write, read);
                    self.meta.swap(write, read);
                }
                write += 1;
            }
        }
        self.truncate(write);
    }

    /// Exchange two events, acceptance included.
    pub fn swap(&mut self, a: usize, b: usize) {
        self.events.swap(a, b);
        self.meta.swap(a, b);
    }

    /// Reverse the order. Acceptance stays on the same events.
    pub fn reverse(&mut self) {
        self.events.reverse();
        self.meta.reverse();
    }

    /// Rotate the list left by `mid` places. Acceptance stays on the same events.
    pub fn rotate_left(&mut self, mid: usize) {
        self.events.rotate_left(mid);
        self.meta.rotate_left(mid);
    }

    /// Rotate the list right by `k` places. Acceptance stays on the same events.
    pub fn rotate_right(&mut self, k: usize) {
        self.events.rotate_right(k);
        self.meta.rotate_right(k);
    }

    /// Sort by the event value. Acceptance stays on the same events.
    pub fn sort_by<F>(&mut self, mut compare: F)
    where
        F: FnMut(&Event, &Event) -> Ordering,
    {
        let events = &self.events;
        let mut order: Vec<usize> = (0..events.len()).collect();
        order.sort_by(|&left, &right| compare(&events[left], &events[right]));
        self.apply_order(&order);
    }

    /// Sort by a key taken from the event. Acceptance stays on the same events.
    pub fn sort_by_key<K, F>(&mut self, mut key: F)
    where
        F: FnMut(&Event) -> K,
        K: Ord,
    {
        let events = &self.events;
        let mut order: Vec<usize> = (0..events.len()).collect();
        order.sort_by_key(|&index| key(&events[index]));
        self.apply_order(&order);
    }

    /// Unstable sort by a comparison. Acceptance stays on the same events.
    pub fn sort_unstable_by<F>(&mut self, mut compare: F)
    where
        F: FnMut(&Event, &Event) -> Ordering,
    {
        let events = &self.events;
        let mut order: Vec<usize> = (0..events.len()).collect();
        order.sort_unstable_by(|&left, &right| compare(&events[left], &events[right]));
        self.apply_order(&order);
    }

    /// Unstable sort by a key. Acceptance stays on the same events.
    pub fn sort_unstable_by_key<K, F>(&mut self, mut key: F)
    where
        F: FnMut(&Event) -> K,
        K: Ord,
    {
        let events = &self.events;
        let mut order: Vec<usize> = (0..events.len()).collect();
        order.sort_unstable_by_key(|&index| key(&events[index]));
        self.apply_order(&order);
    }

    /// Drop consecutive duplicates, keeping the first event and its acceptance.
    pub fn dedup(&mut self) {
        self.dedup_by(|later, earlier| later == earlier);
    }

    /// Drop consecutive events for which `same` returns true, keeping the earlier one.
    ///
    /// `same` is called as `same(later, earlier)`. A `true` result drops `later`.
    /// The predicate cannot modify an event. A modified value would be a different
    /// event, and this method keeps the earlier event's acceptance.
    pub fn dedup_by<F>(&mut self, mut same: F)
    where
        F: FnMut(&Event, &Event) -> bool,
    {
        if self.events.len() < 2 {
            return;
        }
        let mut write = 1;
        for read in 1..self.events.len() {
            let duplicate = same(&self.events[read], &self.events[write - 1]);
            if !duplicate {
                if write != read {
                    self.events.swap(write, read);
                    self.meta.swap(write, read);
                }
                write += 1;
            }
        }
        self.truncate(write);
    }

    /// [`Self::dedup_by`] using a key derived from each event.
    pub fn dedup_by_key<K, F>(&mut self, mut key: F)
    where
        F: FnMut(&Event) -> K,
        K: PartialEq,
    {
        self.dedup_by(|later, earlier| key(later) == key(earlier));
    }

    /// Remove a range and return those events.
    ///
    /// Their acceptance marks are dropped with them.
    pub fn drain<R>(&mut self, range: R) -> alloc::vec::Drain<'_, Event>
    where
        R: RangeBounds<usize>,
    {
        let (start, end) = self.resolve_range(range);
        let _ = self.meta.drain(start..end);
        self.events.drain(start..end)
    }

    /// Split off every event from `at` to the end.
    ///
    /// Each half keeps the acceptance of the events it still holds.
    pub fn split_off(&mut self, at: usize) -> Self {
        Self {
            events: self.events.split_off(at),
            meta: self.meta.split_off(at),
        }
    }

    /// Move every event from `other` onto the end of this list.
    ///
    /// An event keeps its acceptance and its id when that id is not already
    /// present in this list. When the same id appears twice (for instance after
    /// cloning a list and appending the clone), the appended copy gets a fresh
    /// globally unique id and is not accepted.
    pub fn append(&mut self, other: &mut Self) {
        for index in 0..other.meta.len() {
            let id = other.meta[index].id;
            let collides = self.meta.iter().any(|existing| existing.id == id)
                || other.meta[..index].iter().any(|existing| existing.id == id);
            if collides {
                other.meta[index] = EventMeta::fresh();
            }
        }
        self.events.append(&mut other.events);
        self.meta.append(&mut other.meta);
    }

    /// Replace a range with `replacement`.
    ///
    /// Removed events are returned. Inserted events have not been accepted.
    /// Events outside the range keep their acceptance.
    pub fn splice<R, I>(&mut self, range: R, replacement: I) -> Vec<Event>
    where
        R: RangeBounds<usize>,
        I: IntoIterator<Item = Event>,
    {
        let (start, end) = self.resolve_range(range);
        let removed: Vec<Event> = self.events.drain(start..end).collect();
        let _ = self.meta.drain(start..end);
        for (inserted, event) in replacement.into_iter().enumerate() {
            self.events.insert(start + inserted, event);
            self.meta.insert(start + inserted, EventMeta::fresh());
        }
        removed
    }

    /// Mutable access to every event.
    ///
    /// Taking this iterator replaces every event: acceptance is cleared and
    /// each slot gets a new id, including slots the iterator never yields.
    /// Use [`Self::replace`] to change one event.
    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, Event> {
        let len = self.meta.len();
        self.meta.clear();
        push_fresh_meta(&mut self.meta, len);
        self.events.iter_mut()
    }

    pub(crate) fn accept_index(&self, index: usize) {
        if let Some(meta) = self.meta.get(index) {
            meta.accepted.store(true, AtomicOrdering::Relaxed);
        }
    }

    pub(crate) fn accept_id(&self, id: EventId) -> bool {
        for meta in &self.meta {
            if meta.id == id {
                meta.accepted.store(true, AtomicOrdering::Relaxed);
                return true;
            }
        }
        false
    }

    pub(crate) fn accept_first(&self, mut predicate: impl FnMut(&Event) -> bool) -> bool {
        for (event, meta) in core::iter::zip(&self.events, &self.meta) {
            if !meta.accepted.load(AtomicOrdering::Relaxed) && predicate(event) {
                meta.accepted.store(true, AtomicOrdering::Relaxed);
                return true;
            }
        }
        false
    }

    pub(crate) fn accept_where(&self, mut predicate: impl FnMut(&Event) -> bool) {
        for (event, meta) in core::iter::zip(&self.events, &self.meta) {
            if predicate(event) {
                meta.accepted.store(true, AtomicOrdering::Relaxed);
            }
        }
    }

    pub(crate) fn id_at(&self, index: usize) -> Option<EventId> {
        self.meta.get(index).map(|meta| meta.id)
    }

    /// Replace the contents with clones of `source`.
    ///
    /// The buffers are reused. Every stored event is new, so none of them is accepted.
    pub(crate) fn clone_from_events(&mut self, source: &[Event]) {
        self.events.clear();
        self.events.extend_from_slice(source);
        self.meta.clear();
        push_fresh_meta(&mut self.meta, source.len());
    }

    pub(crate) fn carried_unaccepted(&self) -> Vec<Event> {
        core::iter::zip(&self.events, &self.meta)
            .filter(|(event, meta)| {
                !meta.accepted.load(AtomicOrdering::Relaxed) && event.carried_across_discard()
            })
            .map(|(event, _)| event.clone())
            .collect()
    }

    fn apply_order(&mut self, order: &[usize]) {
        let mut events: Vec<Option<Event>> = core::mem::take(&mut self.events)
            .into_iter()
            .map(Some)
            .collect();
        let mut meta: Vec<Option<EventMeta>> = core::mem::take(&mut self.meta)
            .into_iter()
            .map(Some)
            .collect();
        for &source in order {
            let event = events.get_mut(source).and_then(Option::take);
            let meta = meta.get_mut(source).and_then(Option::take);
            if let (Some(event), Some(meta)) = (event, meta) {
                self.events.push(event);
                self.meta.push(meta);
            }
        }
    }

    fn resolve_range(&self, range: impl RangeBounds<usize>) -> (usize, usize) {
        let len = self.events.len();
        let start = match range.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start.saturating_add(1),
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&end) => end.saturating_add(1),
            Bound::Excluded(&end) => end,
            Bound::Unbounded => len,
        };
        (start, end)
    }
}
