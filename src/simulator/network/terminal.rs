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

use super::address::Address;
use super::link::AttachedLink;
use super::packet::Packet;
use super::{Event, LinkAddress};
use crate::simulator::{Payload, Target, Timeout};
use eee_hyst::Time;
use log::{debug, info, trace};
use std::cmp::max;

pub type TerminalAddress = Address;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Terminal {
    header_size: u16,
    payload_size: u16,
    tx_window: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachedTerminal {
    pub addr: TerminalAddress,
    header_size: u16,
    payload_size: u16,
    tx_window: u64,
    pub link_addr: LinkAddress,
    last_acked: u64,
    last_sent: u64,
    last_recv: u64,

    last_tx_sched: Time,
}

impl Terminal {
    pub fn create(header_size: u16, payload_size: u16, tx_window: u16) -> Terminal {
        Terminal {
            header_size,
            payload_size,
            tx_window: u64::from(tx_window),
        }
    }

    pub fn attach_to_link(
        &self,
        self_addr: TerminalAddress,
        link_addr: LinkAddress,
    ) -> AttachedTerminal {
        AttachedTerminal {
            addr: self_addr,
            link_addr,
            header_size: self.header_size,
            payload_size: self.payload_size,
            tx_window: self.tx_window,
            last_acked: 0,
            last_sent: self.tx_window, // A trick to not have to modify the terminal at start
            last_recv: 0,
            last_tx_sched: Time(0),
        }
    }
}

impl AttachedTerminal {
    fn get_dst_address(&self, link: &AttachedLink) -> TerminalAddress {
        if self.addr == link.src_addr {
            link.dst_addr
        } else {
            link.src_addr
        }
    }

    pub fn start(&self, now: Time) -> Vec<Event> {
        (1..=self.last_sent)
            .map(|seqno| Event {
                due_time: now + Time(seqno), // FIXME: Just a hack to make them timeout orderly
                target: Target::Terminal(self.addr),
                kind: Timeout(seqno),
            })
            .collect()
    }

    fn transmit(
        &mut self,
        seqno: u64,
        dst_addr: TerminalAddress,
        now: Time,
        payload_size: u16,
        link: &AttachedLink,
    ) -> Vec<Event> {
        let mut res = Vec::with_capacity(2);

        let p = Packet {
            seqno,
            header_size: self.header_size,
            payload_size,
            src_addr: self.addr,
            dst_addr,
        };

        let delivery_time = self.advance_delivery_time(link, p, now);

        if payload_size > 0 {
            res.push(Event {
                due_time: delivery_time + link.calc_timeout(p),
                target: Target::Terminal(self.addr),
                kind: Timeout(seqno),
            });
        }

        info!("{} sending {}", now.as_secs(), p);
        res.push(Event {
            due_time: delivery_time,
            target: Target::Link(self.link_addr),
            kind: Payload(p),
        });

        res
    }

    fn process_timeout(
        &mut self,
        dst_addr: TerminalAddress,
        seqno: u64,
        now: Time,
        link: &AttachedLink,
    ) -> Vec<Event> {
        if seqno > self.last_acked {
            debug!("Processing timeout {}", seqno);
            self.transmit(seqno, dst_addr, now, self.payload_size, link)
        } else {
            trace!(
                "{} Ignoring timeout for {}, minimum is {}",
                now.as_secs(),
                seqno,
                self.last_acked + 1
            );
            Vec::new()
        }
    }

    fn process_ack(&mut self, packet: &Packet, now: Time, link: &AttachedLink) -> Vec<Event> {
        info!("{} ACK received {}", now.as_secs(), packet);

        if packet.seqno > self.last_acked && packet.seqno <= self.last_sent {
            debug!("Current window: ({}, {}]", self.last_acked, self.last_sent);
            self.last_acked = packet.seqno;

            let res = (self.last_sent + 1..=self.last_acked + self.tx_window)
                .map(|seqno| self.transmit(seqno, packet.src_addr, now, self.payload_size, link))
                .flatten()
                .collect();

            self.last_sent = self.last_acked + self.tx_window;

            debug!("Updated window: ({}, {}]", self.last_acked, self.last_sent);

            res
        } else {
            debug!(
                "Ignoring incorrect ack {}, expecting from ({}, {}]",
                packet.seqno, self.last_acked, self.last_sent
            );

            Vec::new()
        }
    }

    fn process_data(&mut self, packet: &Packet, now: Time, link: &AttachedLink) -> Vec<Event> {
        info!("{} DATA received {}", now.as_secs(), packet);
        if packet.seqno <= self.last_recv + 1 {
            // New data

            self.last_recv = max(self.last_recv, packet.seqno);
            self.transmit(packet.seqno, packet.src_addr, now, 0, link)
        } else {
            debug!(
                "Ignoring unexpected packet {}, expecting {}",
                packet.seqno,
                self.last_recv + 1
            );
            vec![]
        }
    }

    pub fn process(&mut self, event: Event, now: Time, link: &AttachedLink) -> Vec<Event> {
        match event.kind {
            Payload(ref packet) => {
                if packet.payload_size == 0 {
                    self.process_ack(packet, now, link)
                } else {
                    self.process_data(packet, now, link)
                }
            }

            Timeout(seqno) => self.process_timeout(self.get_dst_address(link), seqno, now, link),
        }
    }

    pub fn get_transmitted_packets(&self) -> u64 {
        self.last_acked
    }

    fn advance_delivery_time(&mut self, link: &AttachedLink, packet: Packet, now: Time) -> Time {
        let tx_time = link.tx(packet);

        self.last_tx_sched = max(now, self.last_tx_sched) + tx_time;

        self.last_tx_sched
    }
}
