/*
 * Copyright (C) 2019 Miguel Rodríguez Pérez <miguel@det.uvigo.gal>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

mod network;

use eee_hyst::Time;
use network::address::Address;
use network::packet::Packet;
use std::cmp::Ordering;
use std::collections::binary_heap::BinaryHeap;

pub use self::EventKind::{Payload, Timeout};
pub use network::{Link, Network, Node};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Payload(Packet),
    Timeout(u64),
}

impl EventKind {
    fn weigth(&self) -> usize {
        match self {
            Payload(_) => 0,
            Timeout(_) => 1,
        }
    }
}

impl Ord for EventKind {
    fn cmp(&self, other: &Self) -> Ordering {
        self.weigth().cmp(&other.weigth())
    }
}

impl PartialOrd for EventKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Copy, Eq)]
pub struct Event {
    pub due_time: Time,
    pub target: Address,
    pub kind: EventKind,
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .due_time
            .cmp(&self.due_time)
            .then_with(|| other.kind.cmp(&self.kind))
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.due_time == other.due_time
    }
}

#[derive(Debug, Default)]
pub struct Simulator {
    event_queue: BinaryHeap<Event>,
}

impl Simulator {
    pub fn new() -> Simulator {
        Simulator {
            event_queue: BinaryHeap::new(),
        }
    }

    pub fn add_events(&mut self, events: &[Event]) {
        self.event_queue.extend(events);
    }

    pub fn peek(&self) -> Option<&Event> {
        self.event_queue.peek()
    }

    pub fn pop(&mut self) -> Option<Event> {
        self.event_queue.pop()
    }
}
