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

mod datacounter;

use super::address::Address;
use super::{Event, EventKind, Network};
use crate::network::packet::Packet;
use datacounter::DataCounter;
use log::{info, trace};
use rand;
use rand::distributions::{Bernoulli, Distribution};

use eee_hyst::Time;
use std::cmp::max;

#[derive(Clone, Copy, Debug)]
pub struct Link {
    capacity: f64,
    propagation_delay: Time,
    drop_distribution: Bernoulli,
}

#[derive(Clone, Copy, Debug)]
pub struct AttachedLink {
    pub src_addr: Address,
    pub dst_addr: Address,
    capacity: f64,
    propagation_delay: Time,
    drop_distribution: Bernoulli,

    last_tx_from_src: Time,
    last_tx_from_dst: Time,

    counter: DataCounter,
}

impl Link {
    pub fn create(capacity: f64, propagation_delay: Time, drop_distribution: Bernoulli) -> Link {
        Link {
            capacity,
            propagation_delay,
            drop_distribution,
        }
    }

    pub fn attach_nodes(&self, src_addr: Address, dst_addr: Address) -> AttachedLink {
        AttachedLink {
            src_addr,
            dst_addr,
            capacity: self.capacity,
            propagation_delay: self.propagation_delay,
            drop_distribution: self.drop_distribution,
            last_tx_from_src: Time(0),
            last_tx_from_dst: Time(0),

            counter: DataCounter::default(),
        }
    }
}

impl AttachedLink {
    pub fn process(&mut self, event: &Event, now: Time, _net: &Network) -> Vec<Event> {
        let mut res = Vec::with_capacity(1);

        if let EventKind::Packet(payload) = event.kind {
            let tx_length = self.tx(&payload);
            let tx_time = if payload.src_addr == self.src_addr {
                &mut self.last_tx_from_src
            } else if payload.src_addr == self.dst_addr {
                &mut self.last_tx_from_dst
            } else {
                panic!("Not a valid node address");
            };

            self.counter = self.counter.received_packet(&payload);

            if self.drop_distribution.sample(&mut rand::thread_rng()) {
                trace!("Packet got lost, sorry");
            } else {
                self.counter = self.counter.delivered_packet(&payload);
                res.push(Event {
                    due_time: max(now, *tx_time) + self.propagation_delay + tx_length,
                    target: payload.dst_addr,
                    kind: EventKind::Packet(payload),
                })
            };

            *tx_time = max(now, *tx_time) + tx_length;
        } else {
            panic!("Link event with no attached packet to transmit");
        }

        res
    }

    pub fn tx(&self, packet: &Packet) -> Time {
        Time::from_secs(f64::from(8 * (packet.header_size + packet.payload_size)) / self.capacity)
    }

    pub fn calc_timeout(&self, packet: &Packet) -> Time {
        self.tx(&packet)
            + self.tx(&Packet {
                payload_size: 0,
                ..*packet
            })
            + self.propagation_delay
            + self.propagation_delay
    }

    pub fn get_delivery_time(&self, src_addr: Address, now: Time) -> Time {
        max(
            now,
            if src_addr == self.src_addr {
                self.last_tx_from_src
            } else if src_addr == self.dst_addr {
                self.last_tx_from_dst
            } else {
                panic!("Not a valid node address");
            },
        )
    }

    pub fn show_stats(&self) {
        info!(
            "Received {} bytes ({} of data)",
            self.counter.raw_received, self.counter.good_received
        );
        info!(
            "Delivered {} bytes ({} of data)",
            self.counter.raw_delivered, self.counter.good_delivered
        );
    }
}
