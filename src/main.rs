/*
 * Copyright (C) 2019–2023 Miguel Rodríguez Pérez <miguel@det.uvigo.gal>
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

use clap::Parser;
use eee_hyst::Time;

use arq_simul::simulator::{Link, Network, Simulator, Terminal};
use log::error;

/// A simple discrete time event simulator that shows the behavior of the main
/// ARQ algorithms. It is built with didactic objectives to be used in
/// introductory Computer Networks subject.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Link capacity in bits/s
    #[clap(short = 'C', long = "capacity", default_value = "10e9")]
    capacity: f64,

    /// Header length in bytes
    #[clap(long = "header", default_value = "40")]
    header_length: u32,

    /// Payload length in bytes
    #[clap(long = "payload", default_value = "1460")]
    payload_length: u32,

    /// Window size (in packets)
    #[clap(short = 'w', long = "wsize", default_value = "1")]
    tx_window: u16,

    /// Bit error rate
    #[clap(short = 'b', long = "ber", default_value = "0.0")]
    ber: f64,

    /// Propagation delay, in seconds
    #[clap(short = 'p', long = "prop_delay", default_value = "1e-3")]
    delay: f64,

    /// Simulation duration, in seconds
    #[clap(short = 'l', long = "duration", default_value = "0.1")]
    duration: f64,

    /// Simulation seed
    #[clap(short = 's', long = "seed")]
    seed: Option<u64>,

    /// Verbose level
    #[clap(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    verbose: u8,
}

fn main() {
    let args = Args::parse();

    let filter_level = match log::LevelFilter::iter().nth(args.verbose.into()) {
        Some(level) => level,
        None => log::LevelFilter::max(),
    };

    env_logger::builder()
        .default_format()
        .format_module_path(false)
        .format_target(false)
        .filter_level(filter_level)
        .init();

    if args.capacity <= 0.0 {
        error!("Capacity has to be strictly positive.");
        return;
    }

    if args.ber < 0.0 || args.ber > 1.0 {
        error!("BER has to be between 0 and 1");
        return;
    }

    let delay = if args.delay >= 0.0 {
        Time::from_secs(args.delay)
    } else {
        error!("Propagation delay has to be positive.");
        return;
    };

    let duration = if args.duration > 0.0 {
        Time::from_secs(args.duration)
    } else {
        error!("Simulation duration has to be strictly positive.");
        return;
    };

    let mut network = Network::default();
    let (src_addr, _dst_addr, link_addr) = network.add_link_and_terminals(
        Terminal::create(args.header_length, args.payload_length, args.tx_window),
        Terminal::create(args.header_length, 0, args.tx_window),
        Link::create(args.capacity, delay, args.ber),
    );

    let mut simulator = match args.seed {
        Some(seed) => Simulator::from_seed(seed),
        None => Simulator::default(),
    };
    let mut clock = Time(0);

    simulator.add_events(&network.start(src_addr, clock));

    while clock < duration {
        match simulator.pop() {
            Some(event) => {
                clock = event.due_time;
                let evs = network.process_event(event, clock, &mut simulator.rng);
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
        .get_ref_terminal_by_addr(src_addr)
        .get_transmitted_packets();
    println!(
        "Acknowledged {} bytes ({} of data)",
        acked_packets * u64::from(args.header_length + args.payload_length),
        acked_packets * u64::from(args.payload_length)
    );
    println!(
        "Efficiency: {}% ({}% considering headers)",
        100.0 * 8.0 * (acked_packets * u64::from(args.header_length + args.payload_length)) as f64
            / (args.capacity * duration.as_secs()),
        100.0 * 8.0 * (acked_packets * u64::from(args.payload_length)) as f64
            / (args.capacity * duration.as_secs())
    );
}
