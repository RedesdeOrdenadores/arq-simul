ARQ-Simul
=====

[![Build
Status](https://travis-ci.org/RedesdeOrdenadores/arq-simul.svg?branch=master)](https://travis-ci.org/RedesdeOrdenadores/arq-simul)
[![arq-simul](https://snapcraft.io/arq-simul/badge.svg)](https://snapcraft.io/arq-simul)

## Overview

This is a simple discrete time event simulator that shows the behavior of the
main ARQ algorithms. It is built with didactic objectives to be uses in
introductory Computer Networks subjects.

## Usage
    arq-simul [FLAGS] [OPTIONS]

### FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Verbose level

### OPTIONS:
    -b, --ber <ber>                   Bit error rate [default: 0.0]
    -C, --capacity <capacity>         Link capacity in bits/s [default: 10e9]
    -p, --prop_delay <delay>          Propagation delay, in seconds [default: 1e-3]
    -l, --duration <duration>         Simulation duration, in seconds [default: 0.1]
        --header <header_length>      Header length in bytes [default: 40]
        --payload <payload_length>    Payload length in bytes [default: 1460]
    -w, --wsize <tx_window>           Window size (in packets) [default: 1]

## Legal

Copyright ⓒ 2019 Miguel Rodríguez Pérez <miguel@det.uvigo.gal>.

This simulator is licensed under the GNU General Public License, version 3
(GPL-3.0). For information see LICENSE
