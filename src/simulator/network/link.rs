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
use super::Event;
use super::TerminalAddress;
use crate::simulator::{Payload, Target};
use datacounter::DataCounter;
use log::trace;
use rand::Rng;
use std::convert::TryFrom;

use eee_hyst::Time;

pub type LinkAddress = Address;

#[derive(Clone, Copy, Debug)]
pub struct Link {
    capacity: f64,
    propagation_delay: Time,
    bit_error_rate: f64,
}

#[derive(Clone, Debug)]
pub struct AttachedLink {
    pub src_addr: TerminalAddress,
    pub dst_addr: TerminalAddress,
    capacity: f64,
    propagation_delay: Time,
    bit_error_rate: f64,

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

    pub fn attach_terminals(
        &self,
        src_addr: TerminalAddress,
        dst_addr: TerminalAddress,
    ) -> AttachedLink {
        AttachedLink {
            src_addr,
            dst_addr,
            capacity: self.capacity,
            propagation_delay: self.propagation_delay,
            bit_error_rate: self.bit_error_rate,

            counter: DataCounter::default(),
        }
    }
}

impl AttachedLink {
    fn drop_packet<R: Rng>(&self, packet: Packet, rng: &mut R) -> bool {
        let bit_size = i32::try_from(8 * (packet.header_size + packet.payload_size)).unwrap();
        let prob_tx = (1.0 - self.bit_error_rate).powi(bit_size);

        rng.gen::<f64>() > prob_tx
    }

    pub fn process<R: Rng>(&mut self, event: Event, now: Time, rng: &mut R) -> Vec<Event> {
        if let Payload(packet) = event.kind {
            self.counter = self.counter.received_packet(packet);

            if self.drop_packet(packet, rng) {
                trace!("Packet got lost, sorry");
                Vec::new()
            } else {
                self.counter = self.counter.delivered_packet(packet);
                vec![
                    (Event {
                        due_time: now + self.propagation_delay,
                        target: Target::Terminal(packet.dst_addr),
                        kind: Payload(packet),
                    }),
                ]
            }
        } else {
            panic!("Link event with no attached packet to transmit")
        }
    }

    pub fn tx(&self, packet: Packet) -> Time {
        Time::from_secs(f64::from(8 * (packet.header_size + packet.payload_size)) / self.capacity)
    }

    pub fn calc_timeout(&self, packet: Packet) -> Time {
        self.tx(Packet {
            payload_size: 0,
            ..packet
        }) + self.propagation_delay
            + self.propagation_delay
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
