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
mod node;
pub mod packet;

use super::Event;
use address::Address;
use eee_hyst::Time;
pub use link::{AttachedLink, Link};
pub use node::{AttachedNode, Node};

use std::vec::Vec;
#[derive(Clone, Copy, Debug)]
enum ElementClass {
    Node(AttachedNode),
    Link(AttachedLink),
}

#[derive(Clone, Copy, Debug)]
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

    fn add_element(&mut self, element: ElementClass) -> Address {
        let element = Element {
            addr: Address::create(self.elements.len()),
            class: element,
        };
        self.elements.push(element);

        element.addr
    }

    fn add_node(&mut self, node: AttachedNode) -> Address {
        self.add_element(ElementClass::Node(node))
    }

    fn add_link(&mut self, link: AttachedLink) -> Address {
        let element_addr = self.add_element(ElementClass::Link(link));

        if let (ElementClass::Node(src), ElementClass::Node(dst)) = (
            self.get_ref_by_addr(link.src_addr).class,
            self.get_ref_by_addr(link.dst_addr).class,
        ) {
            assert_eq!(src.link_addr, element_addr);
            assert_eq!(dst.link_addr, element_addr);
        } else {
            panic!(
                "Could not found attached nodes to link at addr {}.",
                element_addr
            );
        }

        element_addr
    }

    pub fn add_link_and_nodes(
        &mut self,
        orig: Node,
        dst: Node,
        link: Link,
    ) -> (Address, Address, Address) {
        let addr_orig = Address::create(self.elements.len());
        let addr_dst = Address::create(usize::from(addr_orig) + 1);
        let link_addr = Address::create(usize::from(addr_dst) + 1);

        let (attached_orig, attached_dst) = (
            orig.attach_to_link(addr_orig, link_addr),
            dst.attach_to_link(addr_dst, link_addr),
        );

        assert_eq!(self.add_node(attached_orig), addr_orig);
        assert_eq!(self.add_node(attached_dst), addr_dst);
        assert_eq!(
            self.add_link(link.attach_nodes(addr_orig, addr_dst)),
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

    pub fn get_ref_node_by_addr(&self, addr: Address) -> &AttachedNode {
        match self.get_ref_by_addr(addr).class {
            ElementClass::Node(ref node) => node,
            _ => panic!("Could not find node at address {}", addr),
        }
    }

    pub fn get_ref_link_by_addr(&self, addr: Address) -> &AttachedLink {
        match self.get_ref_by_addr(addr).class {
            ElementClass::Link(ref link) => link,
            _ => panic!("Could not find link at address {}", addr),
        }
    }

    fn get_mut_by_addr(&mut self, addr: Address) -> &mut Element {
        if let Some(element) = self.elements.get_mut(usize::from(addr)) {
            return element;
        };

        panic!("No node at address {}", addr);
    }

    pub fn process_event(&mut self, event: &Event, now: Time) -> Vec<Event> {
        let (evs, element) = {
            let e = self.get_ref_by_addr(event.target);
            let (addr, class) = (e.addr, e.class);
            match class {
                ElementClass::Node(mut n) => (
                    n.process(event, now, self),
                    Element {
                        addr,
                        class: ElementClass::Node(n),
                    },
                ),
                ElementClass::Link(mut n) => (
                    n.process(event, now, self),
                    Element {
                        addr,
                        class: ElementClass::Link(n),
                    },
                ),
            }
        };

        *self.get_mut_by_addr(event.target) = element;
        evs
    }
}
