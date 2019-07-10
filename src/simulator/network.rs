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

use super::Event;
use address::Address;
use eee_hyst::Time;
pub use link::{AttachedLink, Link};
pub use terminal::{AttachedTerminal, Terminal};

use std::vec::Vec;

#[derive(Clone, Debug)]
enum ElementClass {
    Terminal(AttachedTerminal),
    Link(AttachedLink),
}

#[derive(Clone, Debug)]
struct Element {
    pub addr: Address,
    pub class: ElementClass,
}

#[derive(Clone, Debug, Default)]
pub struct Network {
    elements: Vec<Element>,
}

impl Network {
    pub fn new() -> Network {
        Network {
            elements: Vec::new(),
        }
    }

    pub fn start(&self, terminal_addr: Address, now: Time) -> Vec<Event> {
        let src_terminal = self.get_ref_terminal_by_addr(terminal_addr).clone();
        src_terminal.start(now)
    }

    fn add_element(&mut self, element: ElementClass) -> Address {
        let element = Element {
            addr: Address::create(self.elements.len()),
            class: element,
        };
        let addr = element.addr;

        self.elements.push(element);
        addr
    }

    fn add_terminal(&mut self, terminal: AttachedTerminal) -> Address {
        self.add_element(ElementClass::Terminal(terminal))
    }

    fn add_link(&mut self, link: AttachedLink) -> Address {
        let (src_link_addr, dst_link_addr) = (
            self.get_ref_terminal_by_addr(link.src_addr).link_addr,
            self.get_ref_terminal_by_addr(link.dst_addr).link_addr,
        );

        let element_addr = self.add_element(ElementClass::Link(link));

        assert_eq!(src_link_addr, element_addr);
        assert_eq!(dst_link_addr, element_addr);

        element_addr
    }

    pub fn add_link_and_terminals(
        &mut self,
        orig: Terminal,
        dst: Terminal,
        link: Link,
    ) -> (Address, Address, Address) {
        let addr_orig = Address::create(self.elements.len());
        let addr_dst = Address::create(usize::from(addr_orig) + 1);
        let link_addr = Address::create(usize::from(addr_dst) + 1);

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

    fn get_ref_by_addr(&self, addr: Address) -> &Element {
        if let Some(element) = self.elements.get(usize::from(addr)) {
            return element;
        };

        panic!("No element at address {}", addr);
    }

    pub fn get_ref_terminal_by_addr(&self, addr: Address) -> &AttachedTerminal {
        match self.get_ref_by_addr(addr).class {
            ElementClass::Terminal(ref terminal) => terminal,
            _ => panic!("Could not find terminal at address {}", addr),
        }
    }

    pub fn get_ref_link_by_addr(&self, addr: Address) -> &AttachedLink {
        match self.get_ref_by_addr(addr).class {
            ElementClass::Link(ref link) => link,
            _ => panic!("Could not find link at address {}", addr),
        }
    }

    pub fn get_mut_link_by_addr(&mut self, addr: Address) -> &mut AttachedLink {
        match self.get_mut_by_addr(addr).class {
            ElementClass::Link(ref mut link) => link,
            _ => panic!("Could not find link at address {}", addr),
        }
    }

    fn get_mut_by_addr(&mut self, addr: Address) -> &mut Element {
        if let Some(element) = self.elements.get_mut(usize::from(addr)) {
            return element;
        };

        panic!("No terminal at address {}", addr);
    }

    pub fn process_event(&mut self, event: &Event, now: Time) -> Vec<Event> {
        let (addr, (evs, class)) = {
            let e = self.get_mut_by_addr(event.target);

            (
                e.addr,
                match e.class.clone() {
                    ElementClass::Terminal(mut n) => {
                        (n.process(event, now, self), ElementClass::Terminal(n))
                    }
                    ElementClass::Link(mut l) => (l.process(event, now), ElementClass::Link(l)),
                },
            )
        };

        *self.get_mut_by_addr(event.target) = Element { addr, class };
        evs
    }
}
