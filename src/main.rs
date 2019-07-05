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

#[macro_use]
extern crate log;

use eee_hyst::Time;
use rand::distributions::Bernoulli;
use structopt::StructOpt;

use arq_simul::simulator::{Link, Network, Node, Simulator};

#[derive(StructOpt, Debug)]
struct Opt {
    /// Link capacity in bits/s
    #[structopt(short = "C", long = "capacity", default_value = "10e9")]
    capacity: f64,

    /// Header length in bytes
    #[structopt(long = "header", default_value = "40")]
    header_length: u16,

    /// Payload length in bytes
    #[structopt(long = "payload", default_value = "1460")]
    payload_length: u16,

    /// Window size (in packets)
    #[structopt(short = "w", long = "wsize", default_value = "1")]
    tx_window: u16,

    /// Packet drop probability
    #[structopt(short = "d", long = "drop", default_value = "0.0")]
    drop: f64,

    /// Propagation delay, in seconds
    #[structopt(short = "p", long = "prop_delay", default_value = "1e-3")]
    delay: f64,

    /// Simulation duration, in seconds
    #[structopt(short = "l", long = "duration", default_value = "0.1")]
    duration: f64,

    /// Verbose level
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: usize,
}

fn main() {
    let opt = Opt::from_args();

    stderrlog::new()
        .module(module_path!())
        .verbosity(opt.verbose)
        .init()
        .unwrap();

    if opt.capacity <= 0.0 {
        error!("Capacity has to be strictly positive.");
        return;
    }

    let drop_distribution = match Bernoulli::new(opt.drop) {
        Ok(dist) => dist,
        Err(_) => {
            error!("{} is not a valid probability value.", opt.drop);
            return;
        }
    };

    let delay = if opt.delay >= 0.0 {
        Time::from_secs(opt.delay)
    } else {
        error!("Propagation delay has to be positive.");
        return;
    };

    let duration = if opt.duration > 0.0 {
        Time::from_secs(opt.duration)
    } else {
        error!("Simulation duration has to be strictly positive.");
        return;
    };

    let mut network = Network::new();
    let (src_addr, dst_addr, link_addr) = network.add_link_and_nodes(
        Node::create(opt.header_length, opt.payload_length, opt.tx_window),
        Node::create(opt.header_length, 0, opt.tx_window),
        Link::create(opt.capacity, delay, drop_distribution),
    );

    let mut simulator = Simulator::new();
    let mut clock = Time(0);

    let src_node = *network.get_ref_node_by_addr(src_addr);
    simulator.add_events(&src_node.start(&mut network, dst_addr, clock));

    while clock < duration {
        match simulator.pop() {
            Some(event) => {
                clock = event.due_time;
                let evs = network.process_event(&event, clock);
                simulator.add_events(&evs);
            }
            None => {
                error!("We have run out out events!");
                break;
            }
        }
    }

    let link = network.get_ref_link_by_addr(link_addr);

    link.show_stats();
    let acked_packets = network
        .get_ref_node_by_addr(src_addr)
        .get_transmitted_packets();
    println!(
        "Acknowledged {} bytes ({} of data)",
        acked_packets * u64::from(opt.header_length + opt.payload_length),
        acked_packets * u64::from(opt.payload_length)
    );
    println!(
        "Efficiency: {}% ({}% considering headers)",
        100.0 * 8.0 * (acked_packets * u64::from(opt.header_length + opt.payload_length)) as f64
            / (opt.capacity * duration.as_secs()),
        100.0 * 8.0 * (acked_packets * u64::from(opt.payload_length)) as f64
            / (opt.capacity * duration.as_secs())
    );
}
