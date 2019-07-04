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

use crate::network::packet::Packet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DataCounter {
    pub raw_received: u64,
    pub good_received: u64,
    pub raw_delivered: u64,
    pub good_delivered: u64,
}

impl DataCounter {
    pub fn received_packet(&self, packet: &Packet) -> DataCounter {
        DataCounter {
            raw_received: self.raw_received + raw(packet),
            good_received: self.good_received + good(packet),
            ..*self
        }
    }

    pub fn delivered_packet(&self, packet: &Packet) -> DataCounter {
        DataCounter {
            raw_delivered: self.raw_received + raw(packet),
            good_delivered: self.good_received + good(packet),
            ..*self
        }
    }
}

fn raw(packet: &Packet) -> u64 {
    u64::from(packet.header_size + packet.payload_size)
}

fn good(packet: &Packet) -> u64 {
    u64::from(packet.payload_size)
}
