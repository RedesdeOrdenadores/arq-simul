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
use super::packet::Packet;
use super::{Event, Network};
use crate::simulator::Payload;
use datacounter::DataCounter;
use log::trace;
use rand::Rng;

use eee_hyst::Time;
use std::cmp::max;

#[derive(Clone, Copy, Debug)]
pub struct Link {
    capacity: f64,
    propagation_delay: Time,
    bit_error_rate: f64,
}

#[derive(Clone, Debug)]
pub struct AttachedLink {
    pub src_addr: Address,
    pub dst_addr: Address,
    capacity: f64,
    propagation_delay: Time,
    bit_error_rate: f64,

    last_tx_from_src: Time,
    last_tx_from_dst: Time,

    counter: DataCounter,
}

impl Link {
    pub fn create(capacity: f64, propagation_delay: Time, bit_error_rate: f64) -> Link {
        Link {
            capacity,
            propagation_delay,
            bit_error_rate,
        }
    }

    pub fn attach_nodes(&self, src_addr: Address, dst_addr: Address) -> AttachedLink {
        AttachedLink {
            src_addr,
            dst_addr,
            capacity: self.capacity,
            propagation_delay: self.propagation_delay,
            bit_error_rate: self.bit_error_rate,
            last_tx_from_src: Time(0),
            last_tx_from_dst: Time(0),

            counter: DataCounter::default(),
        }
    }
}

impl AttachedLink {
    fn drop_packet(&self, packet: &Packet) -> bool {
        let bit_size = 8 * i32::from(packet.header_size + packet.payload_size);
        let prob_tx = (1.0 - self.bit_error_rate).powi(bit_size);

        rand::thread_rng().gen::<f64>() > prob_tx
    }

    pub fn process(&mut self, event: &Event, now: Time, _net: &Network) -> Vec<Event> {
        let mut res = Vec::with_capacity(1);

        if let Payload(packet) = event.kind {
            self.counter = self.counter.received_packet(&packet);

            if self.drop_packet(&packet) {
                trace!("Packet got lost, sorry");
            } else {
                self.counter = self.counter.delivered_packet(&packet);
                res.push(Event {
                    due_time: now + self.propagation_delay,
                    target: packet.dst_addr,
                    kind: Payload(packet),
                })
            };
        } else {
            panic!("Link event with no attached packet to transmit");
        }

        res
    }

    pub fn tx(&self, packet: &Packet) -> Time {
        Time::from_secs(f64::from(8 * (packet.header_size + packet.payload_size)) / self.capacity)
    }

    pub fn calc_timeout(&self, packet: &Packet) -> Time {
        self.tx(&Packet {
            payload_size: 0,
            ..*packet
        }) + self.propagation_delay
            + self.propagation_delay
    }

    pub fn advance_delivery_time(&mut self, src_addr: Address, packet: &Packet, now: Time) -> Time {
        let tx_time = self.tx(packet);

        let time = if src_addr == self.src_addr {
            &mut self.last_tx_from_src
        } else if src_addr == self.dst_addr {
            &mut self.last_tx_from_dst
        } else {
            panic!("Not a valid node address");
        };

        *time = max(now, *time) + tx_time;

        *time
    }

    pub fn show_stats(&self) {
        println!(
            "Received {} bytes ({} of data)",
            self.counter.raw_received, self.counter.good_received
        );
        println!(
            "Delivered {} bytes ({} of data)",
            self.counter.raw_delivered, self.counter.good_delivered
        );
    }
}
