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
use super::{ElementClass, Event, Network};
use crate::simulator::{Payload, Timeout};
use eee_hyst::Time;
use log::{debug, info, trace};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Node {
    header_size: u16,
    payload_size: u16,
    tx_window: u64,
    last_acked: u64,
    last_sent: u64,
    last_recv: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttachedNode {
    addr: Address,
    header_size: u16,
    payload_size: u16,
    tx_window: u64,
    pub link_addr: Address,
    last_acked: u64,
    last_sent: u64,
    last_recv: u64,
}

impl Node {
    pub fn create(header_size: u16, payload_size: u16, tx_window: u16) -> Node {
        Node {
            header_size,
            payload_size,
            tx_window: u64::from(tx_window),
            last_acked: 0,
            last_sent: u64::from(tx_window), // A trick to not have to modify the node at start
            last_recv: 0,
        }
    }

    pub fn attach_to_link(&self, self_addr: Address, link_addr: Address) -> AttachedNode {
        AttachedNode {
            addr: self_addr,
            link_addr,
            header_size: self.header_size,
            payload_size: self.payload_size,
            tx_window: self.tx_window,
            last_acked: self.last_acked,
            last_sent: self.last_sent,
            last_recv: self.last_recv,
        }
    }
}

impl AttachedNode {
    fn get_dst_address(&self, net: &Network) -> Address {
        let link = get_link_by_addr(net, self.link_addr);
        if self.addr == link.src_addr {
            link.dst_addr
        } else {
            link.src_addr
        }
    }

    pub fn start(&self, net: &mut Network, dst_addr: Address, now: Time) -> Vec<Event> {
        let link = get_mut_link_by_addr(net, self.link_addr);

        let mut res = Vec::new();
        for seqno in 1..=self.last_sent {
            res.extend(self.transmit(seqno, dst_addr, now, self.payload_size, link))
        }
        res
    }

    fn transmit(
        &self,
        seqno: u64,
        dst_addr: Address,
        now: Time,
        payload_size: u16,
        link: &mut AttachedLink,
    ) -> Vec<Event> {
        let mut res = Vec::with_capacity(2);

        let p = Packet {
            seqno,
            header_size: self.header_size,
            payload_size,
            src_addr: self.addr,
            dst_addr,
        };

        let delivery_time = link.advance_delivery_time(self.addr, &p, now);

        if payload_size > 0 {
            res.push(Event {
                due_time: delivery_time + link.calc_timeout(&p),
                target: self.addr,
                kind: Timeout(seqno),
            });
        }

        info!("{} sending {}", now.as_secs(), p);
        res.push(Event {
            due_time: delivery_time,
            target: self.link_addr,
            kind: Payload(p),
        });

        res
    }

    fn process_timeout(
        &self,
        dst_addr: Address,
        seqno: u64,
        now: Time,
        link: &mut AttachedLink,
    ) -> Vec<Event> {
        self.transmit(seqno, dst_addr, now, self.payload_size, link)
    }

    fn process_ack(&mut self, packet: &Packet, now: Time, link: &mut AttachedLink) -> Vec<Event> {
        info!("{} ACK received {}", now.as_secs(), packet);

        debug!("Current window: ({}, {}]", self.last_acked, self.last_sent);
        self.last_acked = packet.seqno;

        let mut res = Vec::new();
        for seqno in self.last_sent + 1..=self.last_acked + self.tx_window {
            res.extend(self.transmit(seqno, packet.src_addr, now, self.payload_size, link));
        }
        self.last_sent = self.last_acked + self.tx_window;
        debug!("Updated window: ({}, {}]", self.last_acked, self.last_sent);

        res
    }

    fn process_data(&mut self, packet: &Packet, now: Time, link: &mut AttachedLink) -> Vec<Event> {
        info!("{} DATA received {}", now.as_secs(), packet);
        if packet.seqno == self.last_recv + 1 {
            self.last_recv = packet.seqno;
            self.transmit(self.last_recv, packet.src_addr, now, 0, link)
        } else {
            debug!(
                "Ignoring unexpected packet {}, expecting {}",
                packet.seqno,
                self.last_recv + 1
            );
            vec![]
        }
    }

    pub fn process(&mut self, event: &Event, now: Time, net: &mut Network) -> Vec<Event> {
        match event.kind {
            Payload(packet) => {
                if packet.payload_size == 0 {
                    // An ack
                    if packet.seqno > self.last_acked && packet.seqno <= self.last_sent {
                        self.process_ack(&packet, now, get_mut_link_by_addr(net, self.link_addr))
                    } else {
                        debug!(
                            "Ignoring incorrect ack {}, expecting from ({}, {}]",
                            packet.seqno, self.last_acked, self.last_sent
                        );
                        vec![]
                    }
                } else {
                    self.process_data(&packet, now, get_mut_link_by_addr(net, self.link_addr))
                }
            }
            Timeout(seqno) => {
                if seqno > self.last_acked {
                    debug!("Processing timeout {}", seqno);
                    self.process_timeout(
                        self.get_dst_address(net),
                        seqno,
                        now,
                        get_mut_link_by_addr(net, self.link_addr),
                    )
                } else {
                    trace!(
                        "{} Ignoring timeout for {}, minimum is {}",
                        now.as_secs(),
                        seqno,
                        self.last_acked + 1
                    );
                    vec![]
                }
            }
        }
    }
}

fn get_link_by_addr(net: &Network, link_addr: Address) -> &AttachedLink {
    let element = net.get_ref_by_addr(link_addr);
    if let ElementClass::Link(ref link) = element.class {
        return link;
    }

    panic!("Could not find a link at address: {}", link_addr);
}

fn get_mut_link_by_addr(net: &mut Network, link_addr: Address) -> &mut AttachedLink {
    let element = net.get_mut_by_addr(link_addr);
    if let ElementClass::Link(ref mut link) = element.class {
        return link;
    }

    panic!("Could not find a link at address: {}", link_addr);
}
