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

pub mod address;
mod link;
pub mod packet;
mod terminal;

use super::{Event, Target};
use eee_hyst::Time;
pub use link::{AttachedLink, Link, LinkAddress};
pub use terminal::{AttachedTerminal, Terminal, TerminalAddress};

use rand::Rng;

use std::vec::Vec;

#[derive(Clone, Debug, Default)]
pub struct Network {
    nodes: Vec<AttachedTerminal>,
    edges: Vec<AttachedLink>,
}

impl Network {
    pub fn start(&self, terminal_addr: TerminalAddress, now: Time) -> Vec<Event> {
        let src_terminal = self.get_ref_terminal_by_addr(terminal_addr).clone();
        src_terminal.start(now)
    }

    fn add_terminal(&mut self, terminal: AttachedTerminal) -> TerminalAddress {
        assert_eq!(self.nodes.len(), usize::from(terminal.addr));

        let address = terminal.addr;
        self.nodes.push(terminal);

        address
    }

    fn add_link(&mut self, link: AttachedLink) -> LinkAddress {
        let (src_link_addr, dst_link_addr) = (
            self.get_ref_terminal_by_addr(link.src_addr).link_addr,
            self.get_ref_terminal_by_addr(link.dst_addr).link_addr,
        );

        let element_addr = LinkAddress::create(self.edges.len());
        self.edges.push(link);

        assert_eq!(src_link_addr, element_addr);
        assert_eq!(dst_link_addr, element_addr);

        element_addr
    }

    pub fn add_link_and_terminals(
        &mut self,
        orig: Terminal,
        dst: Terminal,
        link: Link,
    ) -> (TerminalAddress, TerminalAddress, LinkAddress) {
        let addr_orig = TerminalAddress::create(self.nodes.len());
        let addr_dst = TerminalAddress::create(usize::from(addr_orig) + 1);
        let link_addr = LinkAddress::create(self.edges.len());

        let (attached_orig, attached_dst) = (
            orig.attach_to_link(addr_orig, link_addr),
            dst.attach_to_link(addr_dst, link_addr),
        );

        assert_eq!(self.add_terminal(attached_orig), addr_orig);
        assert_eq!(self.add_terminal(attached_dst), addr_dst);
        assert_eq!(
            self.add_link(link.attach_terminals(addr_orig, addr_dst)),
            link_addr
        );

        (addr_orig, addr_dst, link_addr)
    }

    pub fn get_ref_terminal_by_addr(&self, addr: TerminalAddress) -> &AttachedTerminal {
        if let Some(terminal) = self.nodes.get(usize::from(addr)) {
            return terminal;
        }

        panic!("No node at address {}", addr);
    }

    pub fn get_mut_terminal_by_addr(&mut self, addr: TerminalAddress) -> &mut AttachedTerminal {
        if let Some(terminal) = self.nodes.get_mut(usize::from(addr)) {
            return terminal;
        }

        panic!("No node at address {}", addr);
    }

    pub fn get_ref_link_by_addr(&self, addr: LinkAddress) -> &AttachedLink {
        if let Some(link) = self.edges.get(usize::from(addr)) {
            return link;
        }

        panic!("Could not find link at address {}", addr);
    }

    pub fn get_mut_link_by_addr(&mut self, addr: LinkAddress) -> &mut AttachedLink {
        if let Some(link) = self.edges.get_mut(usize::from(addr)) {
            return link;
        }

        panic!("Could not find link at address {}", addr);
    }

    pub fn process_event<R: Rng>(&mut self, event: &Event, now: Time, rng: &mut R) -> Vec<Event> {
        match event.target {
            Target::Terminal(terminal_addr) => {
                let terminal = self.get_ref_terminal_by_addr(terminal_addr);
                let link = self.get_ref_link_by_addr(terminal.link_addr).clone();

                self.get_mut_terminal_by_addr(terminal_addr)
                    .process(event, now, &link)
            }
            Target::Link(link_addr) => self
                .get_mut_link_by_addr(link_addr)
                .process(event, now, rng),
        }
    }
}
