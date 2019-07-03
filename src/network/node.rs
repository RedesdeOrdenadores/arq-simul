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
use super::{ElementClass, Event, EventKind, Network};
use eee_hyst::Time;
use log::{debug, info, trace, warn};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Node {
    header_size: u16,
    payload_size: u16,
    last_acked: u64,
    last_sent: u64,
    last_recv: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttachedNode {
    addr: Address,
    header_size: u16,
    payload_size: u16,
    pub link_addr: Address,
    last_acked: u64,
    last_sent: u64,
    last_recv: u64,
}

impl Node {
    pub fn create(header_size: u16, payload_size: u16) -> Node {
        Node {
            header_size,
            payload_size,
            last_acked: 0,
            last_sent: 1,
            last_recv: 0,
        }
    }

    pub fn attach_to_link(&self, self_addr: Address, link_addr: Address) -> AttachedNode {
        AttachedNode {
            addr: self_addr,
            header_size: self.header_size,
            payload_size: self.payload_size,
            link_addr,
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

    pub fn start(&self, net: &Network, dst_addr: Address, now: Time) -> Vec<Event> {
        if let ElementClass::Link(link) = net.get_ref_by_addr(self.link_addr).class {
            self.transmit(self.last_sent, dst_addr, now, self.payload_size, &link)
        } else {
            panic!("There was no link attached in the network");
        }
    }

    fn transmit(
        &self,
        seqno: u64,
        dst_addr: Address,
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

        if payload_size > 0 {
            res.push(Event {
                due_time: link.get_delivery_time(self.addr, now) + link.calc_timeout(&p),
                target: self.addr,
                kind: EventKind::Timeout(seqno),
            });
        }

        info!("{} sending {}", now.as_secs(), p);
        res.push(Event {
            due_time: link.get_delivery_time(self.addr, now),
            target: self.link_addr,
            kind: EventKind::Packet(p),
        });

        res
    }

    fn process_timeout(&self, dst_addr: Address, now: Time, link: &AttachedLink) -> Vec<Event> {
        debug!("Processing timeout");
        self.transmit(self.last_sent, dst_addr, now, self.payload_size, link)
    }

    fn process_ack(&mut self, packet: &Packet, now: Time, link: &AttachedLink) -> Vec<Event> {
        self.last_acked = packet.seqno;
        self.last_sent += 1;
        self.transmit(
            self.last_sent,
            packet.src_addr,
            now,
            self.payload_size,
            link,
        )
    }

    fn process_data(&mut self, packet: &Packet, now: Time, link: &AttachedLink) -> Vec<Event> {
        info!("{} DATA received {}", now.as_secs(), packet);
        if packet.seqno == self.last_recv + 1 {
            self.last_recv = packet.seqno;
            self.transmit(self.last_recv, packet.src_addr, now, 0, link)
        } else {
            debug!(
                "Ignoring duplicate packet {}, expecting {}",
                packet.seqno,
                self.last_recv + 1
            );
            vec![]
        }
    }

    pub fn process(&mut self, event: &Event, now: Time, net: &Network) -> Vec<Event> {
        match event.kind {
            EventKind::Packet(payload) => {
                if payload.payload_size == 0 {
                    // An ack
                    if payload.seqno == self.last_sent {
                        self.process_ack(&payload, now, &get_link_by_addr(net, self.link_addr))
                    } else {
                        debug!(
                            "Ignoring incorrect ack {}, expecting{}",
                            payload.seqno, self.last_sent
                        );
                        vec![]
                    }
                } else {
                    self.process_data(&payload, now, &get_link_by_addr(net, self.link_addr))
                }
            }
            EventKind::Timeout(seqno) => {
                if seqno > self.last_acked {
                    self.process_timeout(
                        self.get_dst_address(net),
                        now,
                        &get_link_by_addr(net, self.link_addr),
                    )
                } else {
                    trace!(
                        "Ignorint timeout for {}, minimum is {}",
                        seqno,
                        self.last_acked + 1
                    );
                    vec![]
                }
            }
        }
    }
}

fn get_link_by_addr(net: &Network, link_addr: Address) -> AttachedLink {
    let element = net.get_ref_by_addr(link_addr);
    if let ElementClass::Link(link) = element.class {
        return link;
    }

    panic!("Could not find a link at address: {}", link_addr);
}
